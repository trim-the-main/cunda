pub mod sys {
    pub use super::super::types::{LogMessage, SysStats};

    topics_for_cunda! {
        list = CUNDA_SYS_TOPICS;
        trait_name = CundaSysT;
        full_mod_path_for_msg_types = cunda_common::v1::topics::sys;
        | TopicTy       | MessageTy     | Path               | Cfg |
        | -------       | ---------     | ----               | --- |
        | SysStatsTopic | SysStats      | "sys_stats_stream" |     |
        | SysLogsTopic  | LogMessage    | "sys_logs_stream"  |     |
    }
}

pub mod gps {
    pub use super::super::types::{GpsDataWire, RawNmea0183Sentence};

    topics_for_cunda! {
        list = CUNDA_GPS_TOPICS;
        trait_name = CundaGpsT;
        full_mod_path_for_msg_types = cunda_common::v1::topics::gps;
        | TopicTy                   | MessageTy                     | Path               | Cfg |
        | -------                   | ---------                     | ----               | --- |
        | RawNmeaTopic              | RawNmea0183Sentence           | "raw_nmea_0183"    |     |
        | ParsedGpsTopic            | GpsDataWire                   | "parsed_gps_t"     |     |
    }
}
