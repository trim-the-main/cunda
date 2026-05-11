use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

pub const VERSION: u32 = 1;

pub mod endpoints;
pub mod topics;

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
