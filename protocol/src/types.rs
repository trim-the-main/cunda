use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Default)]
pub struct NoArg {}

impl From<()> for NoArg {
    fn from(_value: ()) -> Self {
        Self {}
    }
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Default)]
pub struct EmptyRes {}

impl From<()> for EmptyRes {
    fn from(_value: ()) -> Self {
        Self {}
    }
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone)]
pub enum PError {
    SpawnError,
}
