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

pub(crate) const fn hex_nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => panic!("invalid hex digit"),
    }
}
