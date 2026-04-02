#![cfg_attr(not(feature = "flutter"), no_std)]

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

pub mod v1;

// Re-export the latest version
pub use v1::endpoints;
pub use v1::topics;
