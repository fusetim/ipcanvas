use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use ipcanvas_service::{
    canvas::{Canvas, diff::CanvasDiff},
    events::Event,
    ping::{PingServer, PingServerError},
};
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
use tracing::{debug, event, info, span, trace, warn};

const EVENT_BUFFER_SIZE: usize = 128;
const DIFF_BUFFER_SIZE: usize = 10;

/// ipcanvas-service: operation center of ipcanvas.
///
/// This service manages the ping events received from ipcanvas-ping,
/// persist and manage the canvas state, and serve the canvas data to
/// clients over WebRTC data channels.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Opts {
    /// Address to bind for the ping-service.
    #[arg(long, short = 'p', default_value = "0.0.0.0:7894")]
    ping_addr: String,

    /// Address to bind for the WebSocket service.
    #[arg(long, short = 'w', default_value = "0.0.0.0:7895")]
    websocket_addr: String,

    /// Width of the canvas in pixels.
    ///
    /// Should be a multiple of 256.
    #[arg(long = "width", default_value = "4096")]
    canvas_width: u32,

    /// Height of the canvas in pixels.
    ///
    /// Should be a multiple of 256.
    #[arg(long = "height", default_value = "4096")]
    canvas_height: u32,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let opts = Opts::parse();
    env_logger::init();
    console_subscriber::init();

    info!("ipcanvas-service starting...");
    info!("Ping service listening on {}", opts.ping_addr);
    info!("WebSocket service listening on {}", opts.websocket_addr);

    let (event_sender, event_receiver) = mpsc::channel::<Event>(EVENT_BUFFER_SIZE);
    let (diff_sender, mut diff_receiver) = mpsc::channel::<CanvasDiff>(DIFF_BUFFER_SIZE);
    // Prepare the canvas task
    {
        let canvas = Canvas::new(opts.canvas_width as u16, opts.canvas_height as u16);

        // Spawn the canvas management task - diff will be sent every 100ms
        tokio::spawn(canvas_task(
            canvas,
            Duration::from_secs(1),
            event_receiver,
            diff_sender,
        ));
    }

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
                let sender = event_sender.clone();
                match ping_sock_result {
                    Ok((socket, addr)) => {
                        info!("New ping connection from {}", addr);
                        tokio::spawn(async move {
                            if let Err(e) = handle_ping_connection(socket, sender).await {
                                warn!("Error handling ping connection from {}: {}", addr, e);
                            }
                        });
                    }
                    Err(e) => {
                        warn!("Failed to accept ping connection: {}", e);
                    }
                }
            }
            diff = diff_receiver.recv() => {
                match diff {
                    Some(canvas_diff) => {
                        info!("Canvas diff received with {} changed pixels", canvas_diff.changed_pixels().len());
                        for pixel in canvas_diff.changed_pixels() {
                            debug!("Changed pixel at ({}, {}) with color {:?}", pixel.x, pixel.y, pixel.color);
                        }
                        // TODO: Handle the canvas diff (e.g., send to WebSocket clients)
                    }
                    None => {
                        warn!("Canvas diff sender has been closed");
                        break;
                    }
                }
            }
        }
    }

    info!("ipcanvas-service shutting down.");
    Ok(())
}

/// Handle an individual ping connection
async fn handle_ping_connection(
    mut socket: TcpStream,
    events_sender: mpsc::Sender<Event>,
) -> Result<()> {
    let span = span!(tracing::Level::TRACE, "handle_ping_connection");
    let _enter = span.enter();

    let mut ping_server = PingServer::default();
    let (mut reader, _) = socket.split();

    let mut read_buf = [0u8; 4096];
    let mut read_len = 0;

    loop {
        trace!("Handling ping connection loop");
        // Read the outputs from the server
        let to_egress = ping_server.ready_events();
        if to_egress > 0 {
            match events_sender
                .reserve_many(to_egress.min(EVENT_BUFFER_SIZE))
                .await
            {
                Ok(mut permit) => {
                    let n = permit.len();
                    let events = ping_server.egress(n);
                    for event in events {
                        permit.next().expect("permit is allocated").send(event);
                    }
                    trace!("Sent {} events to event channel", n);
                }
                Err(e) => {
                    warn!(
                        "Failed to send events to event channel - channel closed: {}",
                        e
                    );
                    break;
                }
            }
        }

        // Try to make progress
        let rst = ping_server.progress();
        let mut should_block_read = false;
        match rst {
            Ok(()) => {
                trace!("PingServer made progress");
            }
            Err(PingServerError::IngestEmpty) => {
                // Need more ingest to make progress
                should_block_read = true;
                trace!("PingServer is waiting for more ingest");
            }
            Err(PingServerError::EgressFull) => {
                // Cannot make progress until egress is drained
                // (Should happen if progress have been made)
                trace!("PingServer egress is full, waiting for drain");
            }
            Err(PingServerError::Unknown) | Err(PingServerError::IngestFull { .. }) => {
                // Should never happen, just retry later
                debug!("PingServer encountered an unexpected error during progress");
            }
        }

        // If read_buffer has been entirely consumed, read more data
        if read_len == 0 {
            trace!("PingServer trying to read from socket");
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
                trace!("PingServer is blocking until more data is available");
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
            trace!("PingServer ingesting {} bytes", read_len);
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

/// Canvas management task
///
/// This task received the updates to the canvas from the ping service,
/// calculate the new state of the canvas, and create diffs for other tasks
async fn canvas_task(
    mut canvas: Canvas,
    update_interval: Duration,
    mut events_listener: mpsc::Receiver<Event>,
    diff_sender: mpsc::Sender<CanvasDiff>,
) {
    let span = span!(tracing::Level::TRACE, "canvas_task");
    let _enter = span.enter();
    let mut prev_canvas = canvas.clone();

    // Diff are sent periodically (every 100ms)
    let mut interval = tokio::time::interval(update_interval);

    loop {
        tokio::select! { biased;
            event = events_listener.recv() => {
                event!(tracing::Level::TRACE, "Received canvas event");
                match event {
                    Some(Event::PlacePixel { x, y, color }) => {
                        if let Err(_) = canvas.set_pixel(x, y, color) {
                            warn!("Failed to place pixel at ({}, {}): out of bounds", x, y);
                        }
                    }
                    Some(Event::PlaceLabel { .. }) => {
                        todo!("Handle PlaceLabel event");
                    }
                    Some(_) => { unimplemented!() }
                    None => {
                        // Channel closed, exit the task
                        break;
                    }
                }
            }
            _ = interval.tick() => {
                event!(tracing::Level::TRACE, "Canvas update interval ticked");
                // Calculate the diff between the current canvas and the previous canvas
                let diff = prev_canvas.diff(&canvas);
                if diff.is_empty() {
                    // No changes, skip sending
                    continue;
                }
                // Send the diff to other tasks
                if let Err(e) = diff_sender.send(diff).await {
                    warn!("Receiver for canvas diff has been closed: {}", e);
                    break;
                }
                // Update the previous canvas
                prev_canvas = canvas.clone();
            }
        }
    }

    // On channel closure, send the final diff
    let diff = prev_canvas.diff(&canvas);
    let _ = diff_sender.send(diff).await;
}
