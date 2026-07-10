pub use crate::cunda_common::v1::types::*;
pub use crate::types::*;
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Schema, Debug, Clone)]
pub struct ApplSettings {
    pub led_blink_duration_ms: u32,
}

impl ApplSettings {
    pub const fn new() -> Self {
        Self {
            led_blink_duration_ms: 100,
        }
    }
}

impl Default for ApplSettings {
    fn default() -> Self {
        Self {
            led_blink_duration_ms: 100,
        }
    }
}
