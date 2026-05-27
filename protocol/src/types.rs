use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

#[cfg(feature = "flutter")]
macro_rules! ProtocolStringType {
    () => {
        ::std::string::String
    };
    ($N:expr) => {
        ::std::string::String
    };
    (capacity: $N:literal) => {
        ::std::string::String
    };
}

#[cfg(not(feature = "flutter"))]
macro_rules! ProtocolStringType {
    () => {
        ProtocolStringType!(16)
    };
    ($N:expr) => {
        ProtocolStringType!(capacity: $N)
    };
    (capacity: $N:literal) => {
        ::heapless::String<$N>
    };
}

#[cfg(feature = "flutter")]
macro_rules! ProtocolVecType {
    ($t:ty, $N:expr) => {
        ::std::vec::Vec<$t>
    };
}

#[cfg(not(feature = "flutter"))]
macro_rules! ProtocolVecType {
    ($t:ty, $N:expr) => {
        ::heapless::Vec<$t, $N>
    };
}

const fn hex_nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => panic!("invalid hex digit"),
    }
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GitRevSha(pub [u8; 20]);

impl GitRevSha {
    pub const fn from_hex_str(hex: &str) -> Option<Self> {
        let bytes = hex.as_bytes();
        if bytes.len() != 40 {
            return None;
        }
        // "git hash must be 40 hex characters");

        let mut result = [0u8; 20];
        let mut i = 0;
        while i < 20 {
            result[i] = hex_nibble(bytes[i * 2]) * 16 + hex_nibble(bytes[i * 2 + 1]);
            i += 1;
        }
        Some(Self(result))
    }
}

impl core::fmt::Display for GitRevSha {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Default)]
pub struct DeviceId {
    pub device_type: ProtocolStringType!(capacity: 16),
    pub hardware_revision: u32,
    pub serial_number: u32,
    pub firmware_version: ProtocolStringType!(capacity: 16),
    pub protocol_version: u32,
    pub git_hash: Option<GitRevSha>,
}
impl DeviceId {
    pub fn new(
        device_type: &str,
        hardware_revision: u32,
        serial_number: u32,
        firmware_version: &str,
        protocol_version: u32,
        git_hash: Option<GitRevSha>,
    ) -> Self {
        use core::str::FromStr;
        Self {
            device_type: FromStr::from_str(device_type).unwrap(),
            hardware_revision,
            serial_number,
            firmware_version: FromStr::from_str(firmware_version).unwrap(),
            protocol_version,
            git_hash,
        }
    }
}
