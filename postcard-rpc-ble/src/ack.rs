use core::sync::atomic::{AtomicUsize, Ordering};

use maitake_sync::WaitQueue;

#[derive(Debug, PartialEq)]
pub(crate) enum WakeReason {
    Ack,
    Disconnected,
}

#[derive(Debug)]
pub(crate) struct AckQueue {
    q: WaitQueue,
    state: AtomicUsize,
}

impl AckQueue {
    const INFLIGHT: usize = 1 << 0;
    const CONNECTED: usize = 1 << 1;
    const DISCONNECTED: usize = 0;

    pub const fn new() -> Self {
        Self {
            q: WaitQueue::new(),
            state: AtomicUsize::new(0),
        }
    }

    pub fn is_data_inflight(&self) -> bool {
        let state = self.state.load(Ordering::SeqCst);
        state & Self::INFLIGHT == Self::INFLIGHT
    }

    #[allow(dead_code)]
    pub fn is_connected(&self) -> bool {
        let state = self.state.load(Ordering::SeqCst);
        state & Self::CONNECTED == Self::CONNECTED
    }

    pub fn process_connected(&self) {
        defmt::debug!("Processing new connection");
        self.state.store(Self::CONNECTED, Ordering::SeqCst);
    }

    pub fn process_disconnect(&self) {
        defmt::debug!("Processing disconnect");
        self.state.store(Self::DISCONNECTED, Ordering::SeqCst);
        self.q.wake_all()
    }

    pub fn process_ack(&self) {
        defmt::debug!("Processing ACK");
        loop {
            let state = self.state.load(Ordering::SeqCst);
            assert!(state & Self::INFLIGHT == Self::INFLIGHT);
            match self.state.compare_exchange(
                state,
                state & !Self::INFLIGHT,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return self.q.wake_all(),
                Err(_) => continue,
            }
        }
    }

    pub async fn wait_if_inflight(&self) -> WakeReason {
        let wait = self.q.wait();
        let mut wait = core::pin::pin!(wait);
        let _ = wait.as_mut().subscribe();
        if self.is_data_inflight() {
            let _ = wait.await;
        }
        if self.state.load(Ordering::Acquire) & Self::CONNECTED == Self::CONNECTED {
            WakeReason::Ack
        } else {
            WakeReason::Disconnected
        }
    }

    pub async fn wait_once_with<F: AsyncFnOnce() -> ()>(&self, f: F) -> WakeReason {
        let wait = self.q.wait();
        let mut wait = core::pin::pin!(wait);
        let _ = wait.as_mut().subscribe();
        f().await;
        let _ = wait.await;
        if self.state.load(Ordering::Acquire) & Self::CONNECTED == Self::CONNECTED {
            WakeReason::Ack
        } else {
            WakeReason::Disconnected
        }
    }

    pub fn set_data_inflight(&self) -> Result<(), ()> {
        match self.state.compare_exchange(
            Self::CONNECTED,
            Self::INFLIGHT | Self::CONNECTED,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let aq = AckQueue::new();
        assert!(!aq.is_connected());
        assert!(!aq.is_data_inflight());
    }

    #[test]
    fn test_connect_disconnect_cycle() {
        let aq = AckQueue::new();

        aq.process_connected();
        assert!(aq.is_connected());
        assert!(!aq.is_data_inflight());

        aq.process_disconnect();
        assert!(!aq.is_connected());
        assert!(!aq.is_data_inflight());
    }

    #[test]
    fn test_set_inflight_requires_connected() {
        let aq = AckQueue::new();
        assert!(aq.set_data_inflight().is_err());

        aq.process_connected();
        assert!(aq.set_data_inflight().is_ok());
        assert!(aq.is_data_inflight());
    }

    #[test]
    fn test_set_inflight_fails_when_already_inflight() {
        let aq = AckQueue::new();
        aq.process_connected();
        assert!(aq.set_data_inflight().is_ok());
        assert!(aq.set_data_inflight().is_err());
    }

    #[test]
    fn test_process_ack_clears_inflight_keeps_connected() {
        let aq = AckQueue::new();
        aq.process_connected();
        aq.set_data_inflight().unwrap();

        aq.process_ack();
        assert!(aq.is_connected());
        assert!(!aq.is_data_inflight());
    }

    #[tokio::test]
    async fn test_wait_once_with_ack() {
        let aq = AckQueue::new();
        aq.process_connected();

        // The closure inside wait_once_with runs *before* the wait.
        // We simulate: subscribe → set inflight + ack → wake.
        let reason = aq
            .wait_once_with(async || {
                aq.set_data_inflight().unwrap();
                aq.process_ack();
            })
            .await;
        assert_eq!(reason, WakeReason::Ack);
    }

    #[tokio::test]
    async fn test_wait_once_with_disconnect() {
        let aq = AckQueue::new();
        aq.process_connected();

        let reason = aq
            .wait_once_with(async || {
                aq.process_disconnect();
            })
            .await;
        assert_eq!(reason, WakeReason::Disconnected);
    }

    #[tokio::test]
    async fn test_wait_once_with_closure_runs_after_subscribe() {
        // Verifies the subscribe-then-run ordering: the closure runs
        // after subscribing to the WaitQueue, so wakes inside the
        // closure are observed.
        let aq = AckQueue::new();
        aq.process_connected();

        let reason = aq
            .wait_once_with(async || {
                // This wake happens after subscribe, so we should see it
                aq.set_data_inflight().unwrap();
                aq.process_ack();
            })
            .await;
        assert_eq!(reason, WakeReason::Ack);
        assert!(aq.is_connected());
        assert!(!aq.is_data_inflight());
    }
}
