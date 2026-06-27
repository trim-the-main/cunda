pub use super::types::{LogMessage, SysStats};

topics_for_cunda! {
    list = CUNDA_SYS_TOPICS;
    trait_name = CundaSysT;
    path = cunda_common::v1::topics;
    | TopicTy       | MessageTy     | Path               | Cfg |
    | -------       | ---------     | ----               | --- |
    | SysStatsTopic | SysStats      | "sys_stats_stream" |     |
    | SysLogsTopic  | LogMessage    | "sys_logs_stream"  |     |
}
