use anyhow::Result;
use clap::Parser;
use log::{info, warn};

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

    

    info!("ipcanvas-service shutting down.");
    Ok(())
}
