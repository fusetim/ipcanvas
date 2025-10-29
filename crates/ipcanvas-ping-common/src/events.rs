use core::net::Ipv6Addr;

/// Ping Event, structure representing an ICMPv6 Echo Request event
/// that matches the configured IPv6 prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct PingEvent {
    /// Source IPv6 address of the Echo Request
    ///
    /// 128-bit IPv6 address, in (network) big-endian byte order
    pub source_address: [u8; 16],
    /// Destination IPv6 address of the Echo Request
    ///
    /// 128-bit IPv6 address, in (network) big-endian byte order
    pub destination_address: [u8; 16],
}

impl PingEvent {
    /// Create a new PingEvent from source and destination IPv6 addresses
    ///
    /// # Arguments
    /// * `source` - Source IPv6 address as a 16-byte array (in big-endian byte order)
    /// * `destination` - Destination IPv6 address as a 16-byte array (in big-endian byte order)
    ///
    /// # Returns
    /// A new PingEvent instance
    pub fn new(source: [u8; 16], destination: [u8; 16]) -> Self {
        PingEvent {
            source_address: source,
            destination_address: destination,
        }
    }

    /// Get a byte slice representation of the PingEvent
    pub fn as_bytes(&self) -> &[u8; 32] {
        // Safety: PingEvent is #[repr(C)] and consists of two [u8; 16] arrays,
        // so it is safe to transmute it to a [u8; 32] array.
        unsafe { &*(self as *const PingEvent as *const [u8; 32]) }
    }

    /// Create a PingEvent from a byte slice
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        let mut source = [0u8; 16];
        let mut destination = [0u8; 16];
        source.copy_from_slice(&bytes[..16]);
        destination.copy_from_slice(&bytes[16..]);
        PingEvent {
            source_address: source,
            destination_address: destination,
        }
    }

    /// Get the source IPv6 address as an Ipv6Addr
    pub fn source(&self) -> Ipv6Addr {
        Ipv6Addr::from(self.source_address)
    }

    /// Get the destination IPv6 address as an Ipv6Addr
    pub fn destination(&self) -> Ipv6Addr {
        Ipv6Addr::from(self.destination_address)
    }
}
