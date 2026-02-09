use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, Ordering},
};

use embassy_time::{Duration, Instant};
use maitake_sync::{WaitQueue, blocking::RwLock};
use protocol::v1::{CpuUsage, Percent};
/// A simple table to track CPU consumption
///

// this is the row of the table
struct ConsumerInner {
    // Data from current epoch
    begin: Instant,
    runtime_curr_epoch: Duration,
}

struct Consumer {
    executor_id: u32,
    id: u32,
    name: &'static str,
    runtime_last_epoch: Duration, // runtime during the previous stat epoch

    // I don't want to put this behind a lock because every task entry is going to
    // access this twice, so potentially a lot. In order to synchronize we use the
    // a read access from RwSpinLock, so most of the time there should not be any
    // contention. When we find the right Consumer, there is only ever going to be
    // one call at a time for this consumer.
    inner: UnsafeCell<ConsumerInner>,
}

impl Consumer {
    fn new(executor_id: u32, id: u32) -> Self {
        Self {
            executor_id,
            id,
            name: "unknown",
            runtime_last_epoch: Duration::MIN,
            inner: UnsafeCell::new(ConsumerInner {
                begin: Instant::MAX,
                runtime_curr_epoch: Duration::MIN,
            }),
        }
    }

    // SAFETY: We need to call this function from the same executor
    // as Consumer.executor_id to ensure that there's only one consumer
    // being updated.
    unsafe fn start(&self) {
        // defmt::info!("Starting {}", self.id);
        let now = Instant::now();
        let inner = unsafe { &mut *self.inner.get() };
        inner.begin = now;
    }

    // SAFETY: We need to call this function from the same executor
    // as Consumer.executor_id to ensure that there's only one consumer
    // being updated.
    unsafe fn finish(&self, now: Instant) {
        let inner = unsafe { &mut *self.inner.get() };
        assert!(now > inner.begin);
        // defmt::info!("Finished on {} it ran {}", self.id, now - inner.begin);
        inner.runtime_curr_epoch += now - inner.begin;
        // defmt::info!("We ran {} in this window", inner.runtime_curr_epoch);
        inner.begin = Instant::MAX;
    }

    // SAFETY: We need to call this function from the same executor
    // as Consumer.executor_id to ensure that there's only one consumer
    // being updated.
    unsafe fn park(&self) {
        let inner = unsafe { &mut *self.inner.get() };
        inner.begin = Instant::MIN;
    }

    // SAFETY: Call it with no exclusive access
    unsafe fn _is_parked(&self) -> bool {
        let inner = unsafe { &*self.inner.get() };
        inner.begin == Instant::MIN
    }

    // only safe function since we require mutable reference here
    fn change_epoch(&mut self, now: Instant) {
        let inner = self.inner.get_mut();
        if inner.begin > Instant::MIN && inner.begin <= now {
            // we are running on the other executor
            inner.runtime_curr_epoch += now - inner.begin;
            inner.begin = now;
        }
        self.runtime_last_epoch = inner.runtime_curr_epoch;
        inner.runtime_curr_epoch = Duration::MIN;
    }

    fn print_stats(&self) {
        defmt::info!(
            "executor_id: {} task_id: {} runtime: {}",
            self.executor_id,
            self.id,
            self.runtime_last_epoch
        );
    }

    fn give_name(&mut self, name: &'static str) {
        self.name = name;
    }

    // fn close_epoch(&mut self, now: Instant) {
    //     if self.begin == Instant::MAX {
    //         // This guy is not running at the moment
    //         self.runtime_last_epoch = self.runtime_curr_epoch;
    //     } else {
    //         self.runtime_last_epoch = self.runtime_curr_epoch + (now - self.begin);
    //         self.begin = now;
    //     }
    //     self.runtime_curr_epoch = Duration::MIN;
    // }
}

pub struct StatsInner<const N: usize> {
    rows: heapless::Vec<Consumer, N>,
    current_window_start: Instant,
    current_window_end: Instant,
    last_window: Duration,
}

pub struct Stats<const N: usize> {
    inner: RwLock<StatsInner<N>>,
    wait_queue: WaitQueue,
    running: AtomicBool,
}
impl<const N: usize> Stats<N> {
    const EPOCH_DURATION: Duration = Duration::from_millis(2048);
    const fn new() -> Self {
        Self {
            inner: RwLock::new(StatsInner::new()),
            wait_queue: WaitQueue::new(),
            running: AtomicBool::new(false),
        }
    }
}
impl<const N: usize> StatsInner<N> {
    const fn new() -> Self {
        Self {
            rows: heapless::Vec::new(),
            current_window_start: Instant::MIN,
            current_window_end: Instant::MIN,
            last_window: Duration::MAX,
        }
    }

    fn find(&self, executor_id: u32, id: u32) -> Option<&Consumer> {
        return self
            .rows
            .iter()
            .find(|row| row.id == id && row.executor_id == executor_id);
    }

    fn change_epoch(&mut self, now: Instant, epoch_length: Duration) {
        // We need to update the summary and change the window
        // We can assume all the tasks that are associated with this
        // executor are idling, so their accumulated runtime values
        // must show how long they run during this window. The other executor
        // tasks depend on the begin value. They may still be active so we need to
        // reflect that.
        for row in self.rows.iter_mut() {
            row.change_epoch(now);
            row.print_stats();
        }

        self.last_window = now - self.current_window_start;
        self.current_window_end = now + epoch_length;
        self.current_window_start = now;
    }
}

impl<const N: usize> Stats<N> {
    fn executor_begins_work(&self, executor_id: u32) {
        let shared = self.inner.read();
        let Some(row) = shared.find(executor_id, executor_id) else {
            // defmt::info!("Did not find the executor {}", executor_id);
            return;
        };

        // SAFETY: embassy framework guarantees that only one call to this function at a time with
        // executor id
        unsafe {
            row.start();
        }
    }

    fn add_consumer_if_not_found(&self, executor_id: u32, id: u32) {
        let shared = self.inner.read();
        if shared.find(executor_id, id).is_some() {
            return; // this is the happy path
        }

        drop(shared);

        // There's a window here during which we don't hold the lock and
        // some other functions can come inbetween. However, I think it's
        // reasonable to assume that when we reacquire the lock with exclusive
        // access we will not need to check if someone added this row to
        // the vec. It will for sure not be there, there's nobody else who
        // will add it other than this function execution.

        let mut exclusive = self.inner.write();
        if exclusive.rows.push(Consumer::new(executor_id, id)).is_err() {
            defmt::warn!("Failed to push consumer for cpu stats collection");
            return;
        }

        // If what we added is an executor, give it a name it with the current CPU
        if id == executor_id {
            // cannot fail, we have just added it holding an exclusive lock!
            let this = exclusive.rows.last_mut().unwrap();
            match esp_hal::system::Cpu::current() {
                esp_hal::system::Cpu::ProCpu => this.give_name("Cpu 0"),
                esp_hal::system::Cpu::AppCpu => this.give_name("Cpu 1"),
            };
        }
    }

    fn del_task_from_executor(&self, executor_id: u32, task_id: u32) {
        // I don't want to remove it from the vec directly because there may still
        // be some data associated with it. I use begin = 0 to indicate that this
        // task is DEAD. When the window closes we reset the counters, if the counters
        // have not moved for another window then we can remove the task from the vec
        let shared = self.inner.read();
        let Some(row) = shared.find(executor_id, task_id) else {
            return;
        };
        // SAFETY: We came here with the executor_id so there cannot be someone else
        // accessing this row
        unsafe {
            row.park();
        }
    }

    fn task_begins_work(&self, executor_id: u32, task_id: u32) {
        let shared = self.inner.read();
        let Some(row) = shared.find(executor_id, task_id) else {
            return;
        };

        // SAFETY: We came here with the executor_id so there cannot be someone else
        // accessing this row
        unsafe {
            row.start();
        }
    }

    fn task_ends_work(&self, executor_id: u32, task_id: u32) {
        let now = Instant::now();
        let shared = self.inner.read();
        let Some(row) = shared.find(executor_id, task_id) else {
            return;
        };

        // SAFETY: We came here with the executor_id so there cannot be someone else
        // accessing this row
        unsafe {
            row.finish(now);
        }
    }

    fn executor_ends_work(&self, executor_id: u32) {
        let now = Instant::now();
        let shared = self.inner.read();
        let Some(row) = shared.find(executor_id, executor_id) else {
            return;
        };

        // SAFETY: We came here with the executor_id so there cannot be someone else
        // accessing this row
        unsafe {
            row.finish(now);
        }

        if now <= (*shared).current_window_end {
            return;
        }
        drop(shared);

        if !self.running.load(Ordering::Acquire) {
            return;
        }

        let mut exclusive = self.inner.write();
        let now = Instant::now();
        // recheck if we need to change epoch after we get the exclusive lock, maybe
        // another cpu managed to do the work already
        if now <= exclusive.current_window_end {
            return;
        }
        exclusive.change_epoch(now, Self::EPOCH_DURATION);
        self.wait_queue.wake_all();
        defmt::info!(
            "Runtime window was {}, the next summary will be at {}",
            exclusive.last_window,
            exclusive.current_window_end
        );
    }

    fn cpu_stats(&self) -> CpuUsage {
        let mut cpu_usage = CpuUsage::default();
        let inner = self.inner.read();
        for row in inner.rows.iter() {
            if row.id != row.executor_id {
                continue;
            }
            if row.name == "Cpu 0" {
                cpu_usage.core0 = Percent(
                    (row.runtime_last_epoch.as_ticks() * 100 / inner.last_window.as_ticks()) as u8,
                );
            } else if row.name == "Cpu 1" {
                cpu_usage.core1 = Percent(
                    (row.runtime_last_epoch.as_ticks() * 100 / inner.last_window.as_ticks()) as u8,
                );
            }
        }

        return cpu_usage;
    }
}
unsafe impl<const N: usize> Sync for Stats<N> {}

static STATS: Stats<16> = Stats::new();
pub fn cpu_stats() -> CpuUsage {
    STATS.cpu_stats()
}

pub struct StatCollectionGuard;
pub fn start_collecting_stats() -> StatCollectionGuard {
    STATS.running.store(true, Ordering::Release);
    return StatCollectionGuard;
}
impl Drop for StatCollectionGuard {
    fn drop(&mut self) {
        STATS.running.store(false, Ordering::Release);
    }
}

pub async fn wait_new_cpu_stats() {
    // SAFETY: Nobody closes the waitqueue, it's static
    STATS.wait_queue.wait().await.unwrap()
}

/// This callback is called when the executor begins polling. This will always
/// be paired with a later call to `_embassy_trace_executor_idle`.
///
/// This marks the EXECUTOR state transition from IDLE -> SCHEDULING.
#[unsafe(no_mangle)]
pub extern "Rust" fn _embassy_trace_poll_start(executor_id: u32) {
    // defmt::info!("_embassy_trace_poll_start({})", executor_id);
    STATS.executor_begins_work(executor_id);
}

/// This callback is called AFTER a task is initialized/allocated, and BEFORE
/// it is enqueued to run for the first time. If the task ends (and does not
/// loop "forever"), there will be a matching call to `_embassy_trace_task_end`.
///
/// Tasks start life in the SPAWNED state.
#[unsafe(no_mangle)]
pub extern "Rust" fn _embassy_trace_task_new(executor_id: u32, task_id: u32) {
    // defmt::info!("_embassy_trace_task_new({}, {})", executor_id, task_id);
    let _ = STATS.add_consumer_if_not_found(executor_id, executor_id);
    let _ = STATS.add_consumer_if_not_found(task_id, executor_id);
}

/// This callback is called AFTER a task is destructed/freed. This will always
/// have a prior matching call to `_embassy_trace_task_new`.
#[unsafe(no_mangle)]
pub extern "Rust" fn _embassy_trace_task_end(executor_id: u32, task_id: u32) {
    // defmt::info!("_embassy_trace_task_end({}, {})", executor_id, task_id);
    STATS.del_task_from_executor(executor_id, task_id);
}

/// This callback is called AFTER a task has been dequeued from the runqueue,
/// and BEFORE the task is polled. There will always be a matching call to
/// `_embassy_trace_task_exec_end`.
///
/// This marks the TASK state transition from WAITING -> RUNNING
/// This marks the EXECUTOR state transition from SCHEDULING -> POLLING
#[unsafe(no_mangle)]
pub extern "Rust" fn _embassy_trace_task_exec_begin(executor_id: u32, task_id: u32) {
    // defmt::info!(
    //     "_embassy_trace_task_exec_begin({}, {})",
    //     executor_id,
    //     task_id,
    // );
    STATS.task_begins_work(executor_id, task_id);
}

/// This callback is called AFTER a task has completed polling. There will
/// always be a matching call to `_embassy_trace_task_exec_begin`.
///
/// This marks the TASK state transition from either:
/// * RUNNING -> IDLE - if there were no `_embassy_trace_task_ready_begin` events
///     for this task since the last `_embassy_trace_task_exec_begin` for THIS task
/// * RUNNING -> WAITING - if there WAS a `_embassy_trace_task_ready_begin` event
///     for this task since the last `_embassy_trace_task_exec_begin` for THIS task
///
/// This marks the EXECUTOR state transition from POLLING -> SCHEDULING
#[unsafe(no_mangle)]
pub extern "Rust" fn _embassy_trace_task_exec_end(executor_id: u32, task_id: u32) {
    // defmt::info!("_embassy_trace_task_exec_end({}, {})", executor_id, task_id);
    STATS.task_ends_work(executor_id, task_id);
}

/// This callback is called AFTER the waker for a task is awoken, and BEFORE it
/// is added to the run queue.
///
/// If the given task is currently RUNNING, this marks no state change, BUT the
/// RUNNING task will then move to the WAITING stage when polling is complete.
///
/// If the given task is currently IDLE, this marks the TASK state transition
/// from IDLE -> WAITING.
///
/// NOTE: This may be called from an interrupt, outside the context of the current
/// task or executor.
#[unsafe(no_mangle)]
pub extern "Rust" fn _embassy_trace_task_ready_begin(_executor_id: u32, _task_id: u32) {
    // defmt::info!(
    //     "_embassy_trace_task_ready_begin({}, {})",
    //     executor_id,
    //     task_id,
    // );
}

/// This callback is called AFTER all dequeued tasks in a single call to poll
/// have been processed. This will always be paired with a call to
/// `_embassy_trace_executor_idle`.
///
/// This marks the EXECUTOR state transition from SCHEDULING -> IDLE
#[unsafe(no_mangle)]
pub extern "Rust" fn _embassy_trace_executor_idle(executor_id: u32) {
    // defmt::info!("_embassy_trace_executor_idle({})", executor_id);
    STATS.executor_ends_work(executor_id);
}
