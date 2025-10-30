//! PingServer: sans-io server that ingests raw data from the Ping listener and produces Canvas Events.

use std::mem;
use ipcanvas_ping_common::PingEvent;
use crate::events::Event;

#[cfg(test)]
mod tests;

/// PingServer: sans-io server that ingests raw data from the Ping listener and produces Canvas Events.
///
/// The PingServer maintains two internal buffers:
/// - Ingest buffer: holds raw data ingested from the Ping listener
/// - Egress buffer: holds processed Canvas [Event] ready to be consumed by the application
/// 
/// The server comes with internal buffers of configurable sizes for both ingest and egress.
/// The user is responsible for ensuring that the buffers are sized appropriately for their use case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PingServer {
    ingest: Vec<u8>,
    egress: Vec<Event>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PingServerError {
    /// Ingest blocks, as the buffer is full
    /// 
    /// The `read` field indicates how many bytes were read before the buffer became full.
    IngestFull{ read: usize },
    /// Ingest is empty, no data to process
    IngestEmpty,
    /// Egress blocks, as the buffer is full
    EgressFull,
    /// Unknown error
    Unknown,
}

impl PingServer {
    /// Create a new PingServer with specified capacities for ingest and egress buffers
    pub fn new(ingest_capacity: usize, egress_capacity: usize) -> Self {
        debug_assert!(ingest_capacity > 32, "Ingest capacity must be greater than 32 bytes");
        debug_assert!(egress_capacity > 0, "Egress capacity must be greater than 0 events");
        PingServer {
            ingest: Vec::with_capacity(ingest_capacity),
            egress: Vec::with_capacity(egress_capacity),
        }
    }

    /// Ingest raw data into the server's ingest buffer
    pub fn ingest(&mut self, data: &[u8]) -> Result<(), PingServerError> {
        // Ingest should never exceed the vec capacity
        let available_space = self.ingest.capacity() - self.ingest.len();
        let to_read = available_space.min(data.len());
        self.ingest.extend_from_slice(&data[..to_read]);
        if to_read < data.len() {
            // Buffer full, cannot ingest more data
            Err(PingServerError::IngestFull { read: to_read })
        } else {
            Ok(())
        }
    }

    /// Make progress, try to process ingested data into events
    pub fn progress(&mut self) -> Result<(), PingServerError> {
        // Ingress data are expected to be in multiples of 32 bytes (size of PingEvent)
        debug_assert_eq!(mem::size_of::<PingEvent>(), 32);
        if self.ingest.len() < 32 {
            // Not enough data to make progress
            return Err(PingServerError::IngestEmpty);
        }

        // Otherwise, process as many PingEvents as possible
        let mut offset = 0;
        let mut buf = [0u8; 32];
        let mut flag_egress_full = false;
        while offset + 32 <= self.ingest.len() {
            // Check if egress buffer has space,
            // otherwise, we won't be able to make more progress
            if self.egress.len() >= self.egress.capacity() {
                flag_egress_full = true;
                break;
            }

            // Parse PingEvent
            buf.copy_from_slice(&self.ingest[offset..offset + 32]);
            let ping_event = PingEvent::from_bytes(&buf);

            // TODO: For now, we will focus only on PlacePixel events.
            // TODO: We will want to allow decimal x,y coordinates in the future.
            let event = Event::PlacePixel {
                x: u16::from_be_bytes(ping_event.destination_address[6..8].try_into().expect("2-byte slice = u16")),
                y: u16::from_be_bytes(ping_event.destination_address[8..10].try_into().expect("2-byte slice = u16")),
                color: crate::events::PixelColor {
                    r: ping_event.source_address[15],
                    g: ping_event.source_address[13],
                    b: ping_event.source_address[11],
                },
            };
            self.egress.push(event);
            offset += 32;
        }

        // Remove processed data from ingest buffer
        self.ingest.drain(..offset);

        // If egress buffer was full and could not process all events, return the appropriate error
        if flag_egress_full {
            Err(PingServerError::EgressFull)
        } else {
            Ok(())
        }
    }

    /// Egress processed events from the server's egress buffer
    pub fn egress(&mut self, max_events: usize) -> Vec<Event> {
        let to_egress = self.egress.len().min(max_events);
        let events: Vec<Event> = self.egress.drain(..to_egress).collect();
        events
    }
}

impl Default for PingServer {
    fn default() -> Self {
        Self::new(4096, 32)
    }
}