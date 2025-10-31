use anyhow::Result;
use clap::Parser;
use ipcanvas_service::ping::{PingServer, PingServerError};
use log::{debug, info, trace, warn};
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
};

/// ipcanvas-service: operation center of ipcanvas.
///
/// This service manages the ping events received from ipcanvas-ping,
/// persist and manage the canvas state, and serve the canvas data to
/// clients over WebRTC data channels.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Opts {
    /// Address to bind for the ping-service.
    #[arg(long, default_value = "0.0.0.0:7894")]
    ping_addr: String,

    /// Address to bind for the WebRTC service.
    #[arg(long, default_value = "0.0.0.0:7895")]
    webrtc_addr: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opts::parse();
    env_logger::init();
    info!("ipcanvas-service starting...");
    info!("Ping service listening on {}", opts.ping_addr);
    info!("WebRTC service listening on {}", opts.webrtc_addr);

    let ping_socket = TcpListener::bind(opts.ping_addr).await?;
    let ctrl_c = tokio::signal::ctrl_c();

    tokio::pin!(ctrl_c);

    loop {
        tokio::select! {
            _ = &mut ctrl_c => {
                info!("Received Ctrl+C signal");
                break;
            }
            ping_sock_result = ping_socket.accept() => {
                match ping_sock_result {
                    Ok((socket, addr)) => {
                        info!("New ping connection from {}", addr);
                        tokio::spawn(async move {
                            if let Err(e) = handle_ping_connection(socket).await {
                                warn!("Error handling ping connection from {}: {}", addr, e);
                            }
                        });
                    }
                    Err(e) => {
                        warn!("Failed to accept ping connection: {}", e);
                    }
                }
            }
        }
    }

    info!("ipcanvas-service shutting down.");
    Ok(())
}

/// Handle an individual ping connection
async fn handle_ping_connection(mut socket: TcpStream) -> Result<()> {
    let mut ping_server = PingServer::default();
    let (mut reader, _) = socket.split();

    let mut read_buf = [0u8; 4096];
    let mut read_len = 0;

    loop {
        trace!("Handling ping connection loop");
        // Read the outputs from the server
        let _events = ping_server.egress(8);
        // TODO: push them elsewhere (channel)
        // For now, just print them
        for event in _events {
            debug!("PingServer event: {:?}", event);
        }

        // Try to make progress
        let rst = ping_server.progress();
        let mut should_block_read = false;
        match rst {
            Ok(()) => {}
            Err(PingServerError::IngestEmpty) => {
                // Need more ingest to make progress
                should_block_read = true;
            }
            Err(PingServerError::EgressFull) => {
                // Cannot make progress until egress is drained
                // (Should happen if progress have been made)
            }
            Err(PingServerError::Unknown) | Err(PingServerError::IngestFull { .. }) => {
                // Should never happen, just retry later
                debug!("PingServer encountered an unexpected error during progress");
            }
        }

        // If read_buffer has been entirely consumed, read more data
        if read_len == 0 {
            // Read from the socket (and block if no progress can be made otherwise)
            match reader.try_read(&mut read_buf) {
                Ok(0) => {
                    // Connection closed
                    break;
                }
                Ok(n) => {
                    read_len = n;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    read_len = 0;
                }
                Err(e) => {
                    warn!("Error reading from ping socket: {}", e);
                    break;
                }
            }

            // Block if read_len is zero and we need more data to make progress
            if read_len == 0 && should_block_read {
                match reader.read(&mut read_buf).await {
                    Ok(0) => {
                        // Connection closed
                        break;
                    }
                    Ok(n) => {
                        read_len = n;
                    }
                    Err(e) => {
                        warn!("Error reading from ping socket: {}", e);
                        break;
                    }
                }
            }
        }

        // Ingest the read data
        if read_len > 0 {
            match ping_server.ingest(&read_buf[..read_len]) {
                Ok(()) => {
                    // All data ingested
                    read_len = 0;
                }
                Err(PingServerError::IngestFull { read }) => {
                    // Copy the un-ingested data back to the front of the buffer
                    read_buf.copy_within(read..read_len, 0);
                    read_len -= read;
                }
                Err(_) => {}
            }
        }
    }

    Ok(())
}
