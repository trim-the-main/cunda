use serde::{Deserialize, Serialize};

pub use defmt_parser::Level;

#[derive(Debug, Clone)]
pub struct DefmtLogEntry {
    pub level: Option<Level>,
    pub timestamp: String,
    pub location: Option<String>,
    pub msg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogDecoderDefmt {
    table: defmt_decoder::Table,
    locations: defmt_decoder::Locations,
}

#[derive(Debug, Clone)]
pub enum LogDecodingError {
    TableDeserFailed,
    LocationDeserFailed,
    MessageDecodeFailed(Vec<DefmtLogEntry>),
    NoTableData,
}

impl LogDecoderDefmt {
    pub(crate) fn load(
        table_bytes: &[u8],
        locations_bytes: Option<&[u8]>,
    ) -> Result<Self, LogDecodingError> {
        let table: defmt_decoder::Table = postcard::from_bytes(&table_bytes).map_err(|err| {
            log::error!("Error constructing the defmt logger table {}", err);
            LogDecodingError::TableDeserFailed
        })?;
        let locations: defmt_decoder::Locations = if let Some(loc_bytes) = locations_bytes {
            postcard::from_bytes(&loc_bytes).map_err(|err| {
                log::error!("Error constructing the defmt locations table {}", err);
                LogDecodingError::LocationDeserFailed
            })?
        } else {
            Default::default()
        };
        Ok(Self { table, locations })
    }

    pub(crate) fn defmt_decode(
        &self,
        bytes: &[u8],
    ) -> Result<Vec<DefmtLogEntry>, LogDecodingError> {
        let mut stream_decoder = self.table.new_stream_decoder();
        stream_decoder.received(bytes);
        let mut ret = Vec::new();
        loop {
            match stream_decoder.decode() {
                Ok(frame) => {
                    let entry = DefmtLogEntry {
                        level: frame.level(),
                        timestamp: frame
                            .display_timestamp()
                            .map_or(String::new(), |d| d.to_string()),
                        location: self
                            .locations
                            .get(&frame.index())
                            .and_then(|loc| Some(format!("{:?}", loc))),
                        msg: frame.display_message().to_string(),
                    };
                    ret.push(entry);
                }
                Err(defmt_decoder::DecodeError::UnexpectedEof) => break,
                Err(defmt_decoder::DecodeError::Malformed) => {
                    log::error!("Defmt decoder failed due to malformed log message");
                    return Err(LogDecodingError::MessageDecodeFailed(ret));
                } // decode error after potentially
            }
        }
        Ok(ret)
    }
}
