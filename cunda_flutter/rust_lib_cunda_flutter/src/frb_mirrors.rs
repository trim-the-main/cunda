pub use defmt_parser::Level;
pub use frb_prpc_juggle::client_interface::FrbPostcardRpcError;
pub use postcard_rpc::standard_icd::WireError;
pub use postcard_rpc::standard_icd::{FrameTooLong, FrameTooShort};

#[flutter_rust_bridge::frb(mirror(FrameTooLong))]
pub struct _FrameTooLong {
    /// The length of the too-long frame
    pub len: u32,
    /// The maximum frame length supported
    pub max: u32,
}

#[flutter_rust_bridge::frb(mirror(FrameTooShort))]
pub struct _FrameTooShort {
    pub len: u32,
}

#[flutter_rust_bridge::frb(mirror(WireError))]
pub enum _WireError {
    /// The frame exceeded the buffering capabilities of the server
    FrameTooLong(FrameTooLong),
    /// The frame was shorter than the minimum frame size and was rejected
    FrameTooShort(FrameTooShort),
    /// Deserialization of a message failed
    DeserFailed,
    /// Serialization of a message failed, usually due to a lack of space to
    /// buffer the serialized form
    SerFailed,
    /// The key associated with this request was unknown
    UnknownKey,
    /// The server was unable to spawn the associated handler, typically due
    /// to an exhaustion of resources
    FailedToSpawn,
    /// The provided key is below the minimum key size calculated to avoid hash
    /// collisions, and was rejected to avoid potential misunderstanding
    KeyTooSmall,
}

#[flutter_rust_bridge::frb(mirror(FrbPostcardRpcError))]
pub enum _FrbPostcardRpError {
    InternalError,
    OnlyOneDartStreamAllowed,
    DeserializationError,
    AlreadySubscribedtoTopic,
    NotSubscribedToTopic,
    RpcError(WireError),
}

#[flutter_rust_bridge::frb(mirror(Level))]
pub enum _Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

pub use nmea_ubx_gps::pvt::FixType;
#[flutter_rust_bridge::frb(mirror(FixType))]
pub enum _FixType {
    NoFix,
    DeadReckoningOnly,
    Fix2D,
    Fix3D,
    GnssPlusDeadReckoning,
    TimeOnlyFix,
}

pub use nmea_ubx_gps::datetime_wrappers::{WireDate, WireTime};

#[allow(dead_code)]
#[flutter_rust_bridge::frb(mirror(WireDate))]
pub struct _WireDate(time::Date);

#[allow(dead_code)]
#[flutter_rust_bridge::frb(mirror(WireTime))]
pub struct _WireTime(time::Time);
