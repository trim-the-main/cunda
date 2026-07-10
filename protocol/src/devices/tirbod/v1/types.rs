pub use crate::cunda_common::v1::types::*;
pub use crate::types::*;
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Schema, Debug, Clone)]
pub struct ApplSettings {}

impl ApplSettings {
    pub const fn new() -> Self {
        Self {}
    }
}

impl Default for ApplSettings {
    fn default() -> Self {
        Self {}
    }
}
