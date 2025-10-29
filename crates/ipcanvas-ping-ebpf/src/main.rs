#![no_std]
#![no_main]

use core::net::Ipv6Addr;

use aya_ebpf::{
    bindings::xdp_action,
    macros::{map, xdp},
    maps::{Array, RingBuf},
    programs::XdpContext,
};
use aya_log_ebpf::debug;
use ipcanvas_ping_common::{Ipv6Prefix, PingEvent};
use ipcanvas_ping_ebpf::ptr_at;
use network_types::{
    eth::{EthHdr, EtherType},
    icmp::IcmpV6Hdr,
    ip::{IpProto, Ipv6Hdr},
};

/// eBPF map to hold the IPv6 prefix to match against
/// (Stored as 16 u8 bytes representing the 128-bit IPv6 address, and a prefix length as a u8)
#[map]
static PREFIX: Array<[u8; 17]> = Array::<[u8; 17]>::with_max_entries(1, 0);

/// eBPF map to pass the Ping events to user space
///
/// A ping event consists of the source and destination IPv6 addresses (16 bytes each)
/// for a total of 32 bytes.
///
/// The ring buffer should hold at least 1000 events of 32 bytes each, so we allocate 32,000 bytes.
#[map]
static PING: RingBuf = RingBuf::with_byte_size(32768, 0);

#[xdp]
pub fn ipcanvas_ping(ctx: XdpContext) -> u32 {
    // Check for IPv6
    match try_ipv6(&ctx) {
        Ok(_) => {}
        Err(_) => return xdp_action::XDP_PASS, // Not a packet for us
    }

    // Check for ICMPv6 Echo Request
    let ipv6_offset = EthHdr::LEN;
    match try_icmp_echo_request(&ctx, ipv6_offset) {
        Ok(_) => {}
        Err(_) => return xdp_action::XDP_PASS, // Not a packet for us
    }

    // Extract source and destination addresses
    let (source_addr, dest_addr) = match extract_ipv6_addresses(&ctx, ipv6_offset) {
        Ok((src, dst)) => (src, dst),
        Err(_) => return xdp_action::XDP_PASS, // Unable to extract addresses
    };

    debug!(
        &ctx,
        "ICMPv6 Echo Request from {} to {}", source_addr, dest_addr
    );

    // Check if the destination address matches our prefix.
    let prefix_bytes = match PREFIX.get(0) {
        Some(bytes) => bytes,
        None => return xdp_action::XDP_ABORTED, // No prefix configured
    };
    let prefix: Ipv6Prefix = (*prefix_bytes).into();
    if !prefix.matches(&dest_addr) {
        return xdp_action::XDP_PASS; // Does not match prefix
    }

    debug!(&ctx, "Destination {} matches prefix", dest_addr);

    // Prepare the ping event (source and destination addresses)
    let event = PingEvent {
        source_address: source_addr.octets(),
        destination_address: dest_addr.octets(),
    };

    // Send the ping event to user space via the ring buffer
    match PING.output(event, 0) {
        Ok(_) => {
            debug!(&ctx, "Ping event sent to user space");
        }
        Err(_) => {
            debug!(&ctx, "Failed to send ping event to user space - dropped");
            return xdp_action::XDP_DROP;
        }
    }

    // Send back an ICMPv6 Echo Reply (TODO, need a checksum recalculation here)
    xdp_action::XDP_PASS
}

pub fn try_ipv6(ctx: &XdpContext) -> Result<(), ()> {
    let ethhdr: *const EthHdr = ptr_at(ctx, 0)?;
    match unsafe { (*ethhdr).ether_type() } {
        Ok(EtherType::Ipv6) => Ok(()),
        _ => Err(()),
    }
}

pub fn try_icmp_echo_request(ctx: &XdpContext, offset: usize) -> Result<(), ()> {
    let ipv6hdr: *const Ipv6Hdr = ptr_at(&ctx, offset)?;

    if let IpProto::Ipv6Icmp = unsafe { (*ipv6hdr).next_hdr } {
        let icmp_hdr: *const IcmpV6Hdr = ptr_at(&ctx, offset + Ipv6Hdr::LEN)?;
        if unsafe { (*icmp_hdr).type_ } == 128 {
            // Echo Request
            return Ok(());
        }
    }
    Err(())
}

pub fn extract_ipv6_addresses(ctx: &XdpContext, offset: usize) -> Result<(Ipv6Addr, Ipv6Addr), ()> {
    let ipv6hdr: *const Ipv6Hdr = ptr_at(&ctx, offset)?;

    // Get the IPv6 source and destination addresses (from the Network Byte Order)
    let src_addr: u128 = u128::from_be_bytes(unsafe { (*ipv6hdr).src_addr });
    let dst_addr: u128 = u128::from_be_bytes(unsafe { (*ipv6hdr).dst_addr });

    Ok((Ipv6Addr::from(src_addr), Ipv6Addr::from(dst_addr)))
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[unsafe(link_section = "license")]
#[unsafe(no_mangle)]
static LICENSE: [u8; 9] = *b"AGPL-3.0\0";
