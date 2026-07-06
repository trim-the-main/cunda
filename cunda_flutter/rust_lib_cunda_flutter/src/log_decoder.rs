use crate::defmt_log_translation::{DefmtLogEntry, LogDecodingError};

/// Interface for defmt log decoding.
pub trait LogDecoder {
    /// Load defmt table and location data to prepare for decoding.
    fn init_log_decoder(
        &mut self,
        table_bytes: &[u8],
        loc_bytes: &[u8],
    ) -> Result<(), LogDecodingError>;

    /// Decode a raw defmt byte stream into structured log entries.
    #[flutter_rust_bridge::frb(sync)]
    fn decode_log(&self, bytes: &[u8]) -> Result<Vec<DefmtLogEntry>, LogDecodingError>;
}
