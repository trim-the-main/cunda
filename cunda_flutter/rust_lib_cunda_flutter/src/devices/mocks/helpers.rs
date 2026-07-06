use std::sync::{atomic::AtomicBool, Arc};

use crate::frb_generated::StreamSink;

pub(crate) struct PeriodicTopicOutput<T, const N: usize> {
    list_of_outputs: [T; N],
    every: core::time::Duration,
}

impl<T, const N: usize> PeriodicTopicOutput<T, N>
where
    T: crate::frb_generated::SseEncode + Clone + Send + std::fmt::Debug + 'static,
{
    pub(crate) fn new(list_of_outputs: [T; N], every: core::time::Duration) -> Self {
        Self {
            list_of_outputs,
            every,
        }
    }

    pub(crate) async fn leak_into_sink(
        self,
        sink: StreamSink<T>,
        keep_notifying: Arc<AtomicBool>,
    ) -> flutter_rust_bridge::JoinHandle<()> {
        log::info!(
            "Starting periodic topic output, first output will be {:?}",
            self.list_of_outputs[0]
        );
        flutter_rust_bridge::spawn(async move {
            let mut interval = tokio::time::interval(self.every);
            let mut i = 0;
            loop {
                interval.tick().await;
                i = (i + 1) % N;
                if keep_notifying.load(core::sync::atomic::Ordering::Acquire) {
                    sink.add(self.list_of_outputs[i].clone()).unwrap();
                }
            }
        })
    }
}
