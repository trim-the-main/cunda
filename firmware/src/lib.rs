#![no_std]
#![feature(macro_metavar_expr)]
#![feature(maybe_uninit_uninit_array_transpose)]
pub mod ble;
mod rpc;

mod stats;
// mod sys_stats {

//     // We keep track of entry and exit of tasks through embassy trace feature
//     // As tasks begin, we look at the clock and save it to the table, and when
//     // they exit we remove the begin time and accumulate the runtime. We do it
//     // until the stat window ends. When the stat windows closes, stat collector
//     // task goes ahead and reinitializes all the runtime counters.
//     //
//     use embassy_time::{Duration, Instant};

//     //TODO change u32 to nonzero?
//     enum CpuConsumerId {
//         ExecutorId(u32),
//         TaskId(u32),
//     }

//     struct Counter {
//         pub(super) runtime: Duration,
//         pub(super) begin: Instant,
//     }

//     //
//     static STATS_TABLE: heapless::vec::Vec<(CpuConsumerId, Counter), 12> =
//         heapless::vec::Vec::new();
// }
