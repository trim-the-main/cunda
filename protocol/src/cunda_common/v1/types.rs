use crate::cunda_common::VersionString;

use nmea_ubx_gps::datetime_wrappers::{WireDate, WireTime};
use nmea_ubx_gps::pvt::FixType;
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

pub use crate::cunda_common::{DeviceId, GitRevSha};
pub use crate::types::{EmptyRes, NoArg};

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Default)]
pub struct OtaMData {
    pub size: u32,
    pub hash_sha256: [u8; 32],
    pub version: VersionString,
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone)]
pub enum OtaResult {
    TransferReady,
    TransferComplete,
    Restarting,
    StorageError,
    VerificationError,
    NotSupported,
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Default)]
pub struct OtaBytes {
    pub offset: u32,
    pub data: ProtocolVecType!(u8, 4096),
}

/// LOGS TOPIC
#[derive(Serialize, Deserialize, Schema, Debug, Clone, Default)]
pub struct LogMessage {
    pub defmt_bytes: ProtocolVecType!(u8, 1024),
}

#[derive(Serialize, Deserialize, Schema, Debug, Copy, Clone, Default)]
pub struct Percent(pub u8);

#[derive(Serialize, Deserialize, Schema, Debug, Copy, Clone, Default)]
pub struct CpuUsage {
    pub core0: Percent,
    pub core1: Percent,
}
#[derive(Serialize, Deserialize, Schema, Debug, Copy, Clone, Default)]
pub struct MemoryUsage {
    pub used: u32,
    pub total: u32,
}

#[derive(Serialize, Deserialize, Schema, Debug, Copy, Clone, Default)]
pub struct SysStats {
    pub cpu_usage: CpuUsage,
    pub memory_usage: MemoryUsage,
    pub uptime: u32,
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Default)]
pub struct SysSettings {
    // TODO: move the heapless constants to somewhere
    pub wifi_ssid: ProtocolStringType!(32),
    pub wifi_password: ProtocolStringType!(63),
    pub ble_device_name: ProtocolStringType!(20),
}

// GPS stuff
#[derive(Serialize, Deserialize, Schema, Debug, Clone, Default)]
pub struct RawNmea0183Sentence(pub ProtocolStringType!(100));

pub enum NmeaSentenceError {
    Utf8Error(core::str::Utf8Error),
    TooLong(usize),
}

impl TryFrom<&[u8]> for RawNmea0183Sentence {
    type Error = NmeaSentenceError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        type String = ProtocolStringType!(100);
        let s = core::str::from_utf8(value)
            .map_err(|utf8_err| NmeaSentenceError::Utf8Error(utf8_err))?;
        let inner = String::try_from(s).map_err(|_e| NmeaSentenceError::TooLong(s.len()))?;
        Ok(Self(inner))
    }
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Default)]
pub struct GpsDataWire {
    pub fix_type: FixType,
    pub utc_date: Option<WireDate>,
    pub utc_time: Option<WireTime>,

    pub lat: Option<i32>, // 1e-7 degrees
    pub lon: Option<i32>,
    pub h_msl: Option<i32>, // mm
    pub h_acc: Option<u32>, // mm
    pub v_acc: Option<u32>, // mm
    pub pdop: Option<u16>,  // 1e-2 scale

    pub sog: Option<i32>,
    pub cog: Option<i32>,
    pub s_acc: Option<u32>,
    pub c_acc: Option<u32>,

    pub mag_decl: Option<i16>,
    pub mag_decl_acc: Option<u16>,

    pub num_satellites_used: u8,
}

impl From<nmea_ubx_gps::pvt::GpsData> for GpsDataWire {
    fn from(data: nmea_ubx_gps::pvt::GpsData) -> Self {
        Self {
            fix_type: data.fix_type,
            utc_date: data.utc_date,
            utc_time: data.utc_time,
            lat: data.pos.map(|pos| pos.lat),
            lon: data.pos.map(|pos| pos.lon),
            h_msl: data.pos.and_then(|pos| pos.h_msl),
            h_acc: data.pos.and_then(|pos| pos.h_acc).map(|acc| acc.into()),
            v_acc: data.pos.and_then(|pos| pos.v_acc).map(|acc| acc.into()),
            pdop: data.pos.and_then(|pos| pos.pdop).map(|pdop| pdop.into()),
            sog: data.vel.map(|vel| vel.sog),
            cog: data.vel.map(|vel| vel.cog),
            s_acc: data.vel.and_then(|vel| vel.s_acc).map(|acc| acc.into()),
            c_acc: data.vel.and_then(|vel| vel.s_acc).map(|acc| acc.into()),
            mag_decl: data.mag.map(|mag| mag.decl),
            mag_decl_acc: data.mag.map(|mag| mag.acc),
            num_satellites_used: data.num_satellites_used,
        }
    }
}

impl GpsDataWire {
    #[cfg_attr(feature = "flutter", flutter_rust_bridge::frb(sync))]
    #[cfg(feature = "flutter")]
    pub fn utc_date_time(&self) -> Option<chrono::NaiveDateTime> {
        let (Some(WireDate(d)), Some(WireTime(t))) = (self.utc_date, self.utc_time) else {
            return None;
        };
        Some(chrono::NaiveDateTime::new(d, t))
    }
}
