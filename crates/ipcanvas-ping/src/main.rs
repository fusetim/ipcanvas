use std::str::FromStr;

use anyhow::Context as _;
use aya::{
    maps::{Array, RingBuf},
    programs::{Xdp, XdpFlags},
};
use clap::Parser;
use ipcanvas_ping_common::Ipv6Prefix;
#[rustfmt::skip]
use log::{debug, warn, info};
use tokio::{io::unix::AsyncFd, signal};

#[derive(Debug, Parser)]
struct Opt {
    #[clap(short, long, default_value = "eth0")]
    iface: String,

    #[clap(short, long)]
    prefix: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = Opt::parse();

    env_logger::init();

    // Bump the memlock rlimit. This is needed for older kernels that don't use the
    // new memcg based accounting, see https://lwn.net/Articles/837122/
    let rlim = libc::rlimit {
        rlim_cur: libc::RLIM_INFINITY,
        rlim_max: libc::RLIM_INFINITY,
    };
    let ret = unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlim) };
    if ret != 0 {
        debug!("remove limit on locked memory failed, ret is: {ret}");
    }

    // This will include your eBPF object file as raw bytes at compile-time and load it at
    // runtime. This approach is recommended for most real-world use cases. If you would
    // like to specify the eBPF program at runtime rather than at compile-time, you can
    // reach for `Bpf::load_file` instead.
    let mut ebpf = aya::Ebpf::load(aya::include_bytes_aligned!(concat!(
        env!("OUT_DIR"),
        "/ipcanvas-ping"
    )))?;
    match aya_log::EbpfLogger::init(&mut ebpf) {
        Err(e) => {
            // This can happen if you remove all log statements from your eBPF program.
            warn!("failed to initialize eBPF logger: {e}");
        }
        Ok(logger) => {
            let mut logger =
                tokio::io::unix::AsyncFd::with_interest(logger, tokio::io::Interest::READABLE)?;
            tokio::task::spawn(async move {
                loop {
                    let mut guard = logger.readable_mut().await.unwrap();
                    guard.get_inner_mut().flush();
                    guard.clear_ready();
                }
            });
        }
    }
    let Opt { iface, prefix } = opt;

    // Get the prefix from the command line
    let ipv6_prefix = Ipv6Prefix::from_str(&prefix).map_err(|_| {
        anyhow::anyhow!("Invalid IPv6 prefix format, expected format is <address>/<prefix_len>")
    })?;
    info!("Using IPv6 prefix: {}", ipv6_prefix);

    // Load and attach the XDP program
    let program: &mut Xdp = ebpf.program_mut("ipcanvas_ping").unwrap().try_into()?;
    program.load()?;
    program.attach(&iface, XdpFlags::default())
        .context("failed to attach the XDP program with default flags - try changing XdpFlags::default() to XdpFlags::SKB_MODE")?;

    // Attach the PREFIX map to some value for testing
    let mut prefix: Array<_, [u8; 17]> = Array::try_from(ebpf.map_mut("PREFIX").unwrap())?;
    let ipv6_prefix_bytes: [u8; 17] = ipv6_prefix.into();
    prefix.set(0, ipv6_prefix_bytes, 0).unwrap();

    // Attach the PING map
    let ping = RingBuf::try_from(ebpf.map_mut("PING").unwrap())?;
    let ping_fd = AsyncFd::with_interest(ping, tokio::io::Interest::READABLE)?;

    // Prepare to handle Ctrl-C
    let ctrl_c = signal::ctrl_c();

    info!("Waiting for ping events...");
    let mut buf = [0u8; 32];
    tokio::pin!(ctrl_c);
    tokio::pin!(ping_fd);
    loop {
        tokio::select! {
            _ = &mut ctrl_c => {
                info!("Ctrl-C received, exiting...");
                break;
            }
            result = ping_fd.readable_mut() => {
                let mut guard = result?;
                while let Some(data) = guard.get_inner_mut().next() {
                    if data.len() != 32 {
                        warn!("Invalid PingEvent size: {}", data.len());
                        continue;
                    }
                    buf.copy_from_slice(&data);
                    let event = ipcanvas_ping_common::PingEvent::from_bytes(&buf);
                    info!(
                        "PingEvent - Source: {}, Destination: {}",
                        event.source(),
                        event.destination()
                    );
                }
                guard.clear_ready();
            }
        }
    }

    Ok(())
}
