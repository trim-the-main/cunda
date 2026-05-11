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

#[derive(serde::Serialize, serde::Deserialize, postcard_schema::Schema, Debug, Clone, Default)]
pub struct DeviceId {
    pub device_type: ProtocolStringType!(capacity: 16),
    pub hardware_revision: u32,
    pub serial_number: u32,
    pub firmware_version: ProtocolStringType!(capacity: 16),
    pub protocol_version: u32,
}
impl DeviceId {
    pub fn new(
        device_type: &str,
        hardware_revision: u32,
        serial_number: u32,
        firmware_version: &str,
        protocol_version: u32,
    ) -> Self {
        use core::str::FromStr;
        Self {
            device_type: FromStr::from_str(device_type).unwrap(),
            hardware_revision,
            serial_number,
            firmware_version: FromStr::from_str(firmware_version).unwrap(),
            protocol_version,
        }
    }
}
