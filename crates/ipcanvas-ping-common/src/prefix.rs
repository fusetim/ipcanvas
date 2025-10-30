use core::{convert::From, fmt::Display, net::Ipv6Addr};

/// Structure to represent an IPv6 prefix (address + prefix length)
/// Used in both eBPF and user-space code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct Ipv6Prefix {
    /// 128-bit IPv6 address, in (network) big-endian byte order
    pub address: [u8; 16],
    /// Prefix length (0-128)
    pub prefix_len: u8,
}

impl From<Ipv6Prefix> for [u8; 17] {
    fn from(prefix: Ipv6Prefix) -> Self {
        let mut bytes = [0u8; 17];
        bytes[..16].copy_from_slice(&prefix.address);
        bytes[16] = prefix.prefix_len;
        bytes
    }
}

impl From<[u8; 17]> for Ipv6Prefix {
    fn from(bytes: [u8; 17]) -> Self {
        let mut address = [0u8; 16];
        address.copy_from_slice(&bytes[..16]);
        let prefix_len = bytes[16];
        Ipv6Prefix {
            address,
            prefix_len,
        }
    }
}

impl From<(Ipv6Addr, u8)> for Ipv6Prefix {
    fn from((addr, len): (Ipv6Addr, u8)) -> Self {
        Ipv6Prefix {
            address: addr.octets(),
            prefix_len: len,
        }
    }
}

#[cfg(feature = "std")]
use core::str::FromStr;
#[cfg(feature = "std")]
impl FromStr for Ipv6Prefix {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return Err(());
        }

        let addr = parts[0].parse::<Ipv6Addr>().map_err(|_| ())?;
        let prefix_len = parts[1].parse::<u8>().map_err(|_| ())?;

        Ok(Ipv6Prefix {
            address: addr.octets(),
            prefix_len,
        })
    }
}

impl Display for Ipv6Prefix {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let addr = Ipv6Addr::from(self.address);
        write!(f, "{}/{}", addr, self.prefix_len)
    }
}

impl Ipv6Prefix {
    /// Check if the given IPv6 address matches this prefix
    pub fn matches(&self, addr: &Ipv6Addr) -> bool {
        debug_assert!(self.prefix_len <= 128);
        let addr_bytes = addr.octets();

        let full_bytes = (self.prefix_len / 8) as u8;
        let remaining_bits = self.prefix_len - (full_bytes * 8);

        // Check full bytes
        // It is ugly, but it is necessary to eliminate loop/iterations to pass the eBPF verifier
        for k in 0..16 {
            if !(full_bytes <= k || addr_bytes[k as usize] == self.address[k as usize]) {
                return false;
            }
        }

        // Check remaining bits
        let mut remaining_match = true;
        if remaining_bits > 0 && (full_bytes < 16) {
            let mask = 0xFF << (8 - remaining_bits);
            let masked_addr_byte = self.address[full_bytes as usize] & mask;
            let masked_prefix_byte = addr_bytes[full_bytes as usize] & mask;
            if masked_addr_byte != masked_prefix_byte {
                remaining_match = false;
            }
        }

        remaining_match
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_prefix() {
        // Test matching an IPv6 address against a /64 prefix
        let prefix = Ipv6Prefix::from((Ipv6Addr::from_str("2001:db8::").unwrap(), 64));
        let addr1 = Ipv6Addr::from_str("2001:db8::1").unwrap();
        let addr2 = Ipv6Addr::from_str("2001:db8:0:1::1").unwrap();
        let addr3 = Ipv6Addr::from_str("2001:db9::1").unwrap();
        assert!(prefix.matches(&addr1));
        assert!(!prefix.matches(&addr2));
        assert!(!prefix.matches(&addr3));

        // Test matching an IPv6 address against a /48 prefix
        let prefix = Ipv6Prefix::from((Ipv6Addr::from_str("2001:db8::").unwrap(), 48));
        let addr1 = Ipv6Addr::from_str("2001:db8::1").unwrap();
        let addr2 = Ipv6Addr::from_str("2001:db8:0:1::1").unwrap();
        let addr3 = Ipv6Addr::from_str("2001:db9::1").unwrap();
        assert!(prefix.matches(&addr1));
        assert!(prefix.matches(&addr2));
        assert!(!prefix.matches(&addr3));

        // Test matching an IPv6 address against a /127 prefix
        let prefix = Ipv6Prefix::from((Ipv6Addr::from_str("2001:db8::4320").unwrap(), 127));
        let addr1 = Ipv6Addr::from_str("2001:db8::4320").unwrap();
        let addr2 = Ipv6Addr::from_str("2001:db8::4321").unwrap();
        let addr3 = Ipv6Addr::from_str("2001:db8::4322").unwrap();
        assert!(prefix.matches(&addr1));
        assert!(prefix.matches(&addr2));
        assert!(!prefix.matches(&addr3));
    }
}
