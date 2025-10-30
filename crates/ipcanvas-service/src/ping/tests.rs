//! Test module for the ping-server implementation.

use super::*;

#[test]
fn ping_server_buffers_min_size() {
    // Test that PingServer enforces minimum buffer sizes in debug builds
    let server = PingServer::new(64, 16);
    assert_eq!(server.ingest.capacity(), 64);
    assert_eq!(server.egress.capacity(), 16);

    if cfg!(debug_assertions) {
        let result = std::panic::catch_unwind(|| {
            PingServer::new(16, 16);
        });
        assert!(
            result.is_err(),
            "Expected panic for ingest capacity <= 32 bytes"
        );

        let result = std::panic::catch_unwind(|| {
            PingServer::new(64, 0);
        });
        assert!(
            result.is_err(),
            "Expected panic for egress capacity <= 0 events"
        );
    }
}

#[test]
fn ping_server_ingress_do_not_exceed_capacity() {
    let mut server = PingServer::new(64, 16);
    let data = vec![0u8; 100]; // 100 bytes of data

    // Try to ingest more data than capacity
    let result = server.ingest(&data);
    assert!(result.is_err(), "Expected IngestFull error");
    match result {
        Err(PingServerError::IngestFull { read }) => {
            assert_eq!(read, 64, "Expected to read up to capacity");
            assert_eq!(server.ingest.len(), 64, "Ingest buffer should be full");
        }
        _ => panic!("Unexpected error type"),
    }

    // Ingest less data than capacity, but the buffer is already full
    let result = server.ingest(&data[..10]);
    assert!(result.is_err(), "Expected IngestFull error");
    match result {
        Err(PingServerError::IngestFull { read }) => {
            assert_eq!(read, 0, "Expected to read 0 bytes as buffer is full");
            assert_eq!(server.ingest.len(), 64, "Ingest buffer should remain full");
        }
        _ => panic!("Unexpected error type"),
    }
}

#[test]
fn ping_server_ingress_do_not_exceed_capacity_partial() {
    let mut server = PingServer::new(50, 16);
    let data = vec![0u8; 100]; // 100 bytes of data

    // Ingest a bit of data, so the buffer is partially filled
    let result = server.ingest(&data[..30]);
    assert!(result.is_ok(), "Expected successful ingest");
    assert_eq!(
        server.ingest.len(),
        30,
        "Ingest buffer should have 30 bytes"
    );

    // Try to ingest more data than remaining capacity
    let result = server.ingest(&data[..30]);
    assert!(result.is_err(), "Expected IngestFull error");
    match result {
        Err(PingServerError::IngestFull { read }) => {
            assert_eq!(read, 20, "Expected to read up to remaining capacity");
            assert_eq!(server.ingest.len(), 50, "Ingest buffer should be full");
        }
        _ => panic!("Unexpected error type"),
    }
}

#[test]
fn ping_server_progress_should_error_if_insufficient_ingress_data() {
    let mut server = PingServer::new(64, 16);

    // Ingest less than 32 bytes
    let data = vec![0u8; 20];
    let result = server.ingest(&data);
    assert!(result.is_ok(), "Expected successful ingest");
    assert_eq!(
        server.ingest.len(),
        20,
        "Ingest buffer should have 20 bytes"
    );

    // Try to make progress
    let result = server.progress();
    assert!(result.is_err(), "Expected IngestEmpty error");
    match result {
        Err(PingServerError::IngestEmpty) => {}
        _ => panic!("Unexpected error type"),
    }
}

#[test]
fn ping_server_progress_should_error_if_insufficient_place_in_egress() {
    let mut server = PingServer::new(128, 2); // Small egress capacity

    // Ingest enough data for 3 PingEvents (96 bytes)
    let data = vec![0u8; 96];
    let result = server.ingest(&data);
    assert!(result.is_ok(), "Expected successful ingest");
    assert_eq!(
        server.ingest.len(),
        96,
        "Ingest buffer should have 96 bytes"
    );

    // Try to make progress
    let result = server.progress();
    assert!(result.is_err(), "Expected EgressFull error");
    match result {
        Err(PingServerError::EgressFull) => {
            assert_eq!(
                server.egress.len(),
                2,
                "Egress buffer should be full with 2 events"
            );
            assert_eq!(
                server.ingest.len(),
                96 - 64,
                "Ingest buffer should have remaining data"
            );
        }
        _ => panic!("Unexpected error type"),
    }
}

#[test]
fn ping_server_progress_processes_events_correctly() {
    let mut server = PingServer::new(128, 4);
    // Ingest enough data for 4 PingEvents (128 bytes)
    let data = vec![0u8; 128];
    let result = server.ingest(&data);
    assert!(result.is_ok(), "Expected successful ingest");
    assert_eq!(
        server.ingest.len(),
        128,
        "Ingest buffer should have 128 bytes"
    );

    // Try to make progress
    let result = server.progress();
    assert!(result.is_ok(), "Expected successful progress");
    assert_eq!(server.egress.len(), 4, "Egress buffer should have 4 events");
    assert_eq!(server.ingest.len(), 0, "Ingest buffer should be empty");

    // NOTE: This test will probably fail in the future, when other events will be supported.
}

#[test]
fn ping_server_egress_when_empty() {
    let mut server = PingServer::new(128, 4);

    // Egress when egress buffer is empty
    let events = server.egress(2);
    assert_eq!(events.len(), 0, "Expected no events egressed");
}

#[test]
fn ping_server_egress_partial() {
    let mut server = PingServer::new(128, 4);
    // Ingest enough data for 3 PingEvents (96 bytes)
    let data = vec![0u8; 96];
    let result = server.ingest(&data);
    assert!(result.is_ok(), "Expected successful ingest");

    // Make progress to process events
    let result = server.progress();
    assert!(result.is_ok(), "Expected successful progress");
    assert_eq!(server.egress.len(), 3, "Egress buffer should have 3 events");

    // Egress some events
    let events = server.egress(2);
    assert_eq!(events.len(), 2, "Expected 2 events egressed");

    assert_eq!(
        server.egress.len(),
        1,
        "Egress buffer should have 1 event remaining"
    );

    // Egress remaining events
    let events = server.egress(2);
    assert_eq!(events.len(), 1, "Expected 1 event egressed");
}

#[test]
fn ping_server_egress_all() {
    let mut server = PingServer::new(128, 4);
    // Ingest enough data for 4 PingEvents (128 bytes)
    let data = vec![0u8; 128];
    let result = server.ingest(&data);
    assert!(result.is_ok(), "Expected successful ingest");

    // Make progress to process events
    let result = server.progress();
    assert!(result.is_ok(), "Expected successful progress");
    assert_eq!(server.egress.len(), 4, "Egress buffer should have 4 events");

    // Egress all events
    let events = server.egress(10);
    assert_eq!(events.len(), 4, "Expected 4 events egressed");
    assert_eq!(server.egress.len(), 0, "Egress buffer should be empty");
}

#[test]
fn ping_server_handle_ping_event() {
    // Currently only one event type is supported, so this test is simple
    let redx10y0 = PingEvent {
        destination_address: [0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 255, 0, 0, 0, 0],
        source_address: [0; 16],
    };
    let bluex20y30 = PingEvent {
        destination_address: [0, 0, 0, 0, 0, 0, 0, 20, 0, 10, 0, 0, 0, 0, 0, 255],
        source_address: [0; 16],
    };
    let whitex256y256 = PingEvent {
        destination_address: [0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 255, 0, 255, 0, 255],
        source_address: [0; 16],
    };

    let red_event = PingServer::handle_ping_event(&redx10y0);
    assert_eq!(
        red_event,
        vec![Event::PlacePixel { x: 10, y: 0, color: crate::events::PixelColor { r: 255, g: 0, b: 0 } }],
        "Red pixel event mismatch"
    );

    let blue_event = PingServer::handle_ping_event(&bluex20y30);
    assert_eq!(
        blue_event,
        vec![Event::PlacePixel { x: 20, y: 10, color: crate::events::PixelColor { r: 0, g: 0, b: 255 } }],
        "Blue pixel event mismatch"
    );

    let white_event = PingServer::handle_ping_event(&whitex256y256);
    assert_eq!(
        white_event,
        vec![Event::PlacePixel { x: 256, y: 256, color: crate::events::PixelColor { r: 255, g: 255, b: 255 } }],
        "White pixel event mismatch"
    );
}

#[test]
fn ping_server_handle_incoming_ping_event() {
    // Currently only one event type is supported, so this test is simple
    let redx10y0 = PingEvent {
        destination_address: [0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 255, 0, 0, 0, 0],
        source_address: [0; 16],
    };
    let bluex20y30 = PingEvent {
        destination_address: [0, 0, 0, 0, 0, 0, 0, 20, 0, 10, 0, 0, 0, 0, 0, 255],
        source_address: [0; 16],
    };
    let whitex256y256 = PingEvent {
        destination_address: [0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 255, 0, 255, 0, 255],
        source_address: [0; 16],
    };

    let mut server = PingServer::new(96, 4); // Enough for 3 PingEvents
    let mut buf = [0u8; 96];
    buf[0..32].copy_from_slice(redx10y0.as_bytes());
    buf[32..64].copy_from_slice(bluex20y30.as_bytes());
    buf[64..96].copy_from_slice(whitex256y256.as_bytes());

    let result = server.ingest(&buf);
    assert!(result.is_ok(), "Expected successful ingest");

    let result = server.progress();
    assert!(result.is_ok(), "Expected successful progress");
    assert_eq!(server.egress.len(), 3, "Egress buffer should have 3 events");

    let events = server.egress(3);
    assert_eq!(events.len(), 3, "Expected 3 events egressed");
    assert_eq!(
        events[0],
        Event::PlacePixel {
            x: 10,
            y: 0,
            color: crate::events::PixelColor { r: 255, g: 0, b: 0 }
        },
        "Red pixel event mismatch"
    );
    assert_eq!(
        events[1],
        Event::PlacePixel {
            x: 20,
            y: 10,
            color: crate::events::PixelColor { r: 0, g: 0, b: 255 }
        },
        "Blue pixel event mismatch"
    );
    assert_eq!(
        events[2],
        Event::PlacePixel {
            x: 256,
            y: 256,
            color: crate::events::PixelColor { r: 255, g: 255, b: 255 }
        },
        "White pixel event mismatch"
    );
}