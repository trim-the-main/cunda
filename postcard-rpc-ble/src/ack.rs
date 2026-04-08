use core::sync::atomic::{AtomicUsize, Ordering};

use maitake_sync::WaitQueue;

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

    pub fn process_connected(&self) {
        defmt::debug!("Processing new connection");
        self.state.store(Self::CONNECTED, Ordering::SeqCst);
    }

    pub fn process_disconnect(&self) {
        defmt::debug!("Processing disconnect");
        self.state.store(Self::DISCONNECTED, Ordering::SeqCst);
        self.q.wake_all()
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

    pub fn is_data_inflight(&self) -> bool {
        let state = self.state.load(Ordering::SeqCst);
        state & Self::INFLIGHT == Self::INFLIGHT
    }

    pub fn is_connected(&self) -> bool {
        let state = self.state.load(Ordering::SeqCst);
        state & Self::CONNECTED == Self::CONNECTED
    }
}
