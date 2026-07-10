use core::{cell::RefCell, mem::MaybeUninit};

use defmt_brtt::DefmtConsumer;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use postcard_rpc::{Key, TopicMap, server::SpawnContext};
use protocol::devices::nokta::v1::topics;

type TopicStopSignal = Signal<CriticalSectionRawMutex, ()>;
const fn topic_state<const SIZE: usize>(map: TopicMap) -> [(&'static Key, TopicStopSignal); SIZE] {
    let mut table: [MaybeUninit<(&'static Key, TopicStopSignal)>; SIZE] =
        MaybeUninit::uninit().transpose();
    let mut i = 0;
    while i < SIZE {
        table[i].write((&map.topics[i].1, TopicStopSignal::new()));
        i += 1;
    }

    assert!(
        size_of_val(&table) == size_of::<[(&'static Key, TopicStopSignal); SIZE]>(),
        "The use of transmute_copy is not correct."
    );
    unsafe { core::mem::transmute_copy::<_, [(&'static Key, TopicStopSignal); SIZE]>(&table) }
}

#[derive(Copy, Clone)]
pub struct TopicTaskTable(&'static [(&'static Key, TopicStopSignal)]);
impl TopicTaskTable {
    pub(crate) fn stop_signal(&self, topic_key: postcard_rpc::Key) -> &TopicStopSignal {
        self.0
            .iter()
            .find(|(key, _)| **key == topic_key)
            .map(|(_key, signal)| signal)
            .expect("Every topic has to have a topic task entry")
    }
}
static TOPIC_TASK_STATE: [(&'static Key, TopicStopSignal); topics::TOPICS.topics.len()] =
    topic_state(topics::TOPICS);

pub(crate) struct DispatchContext {
    pub(crate) task_table: TopicTaskTable,
    pub(crate) logger: &'static RefCell<DefmtConsumer>,
    pub(crate) tx: crate::ble::BleWireTxImpl,
}

impl DispatchContext {
    pub fn new(logger: &'static RefCell<DefmtConsumer>, tx: crate::ble::BleWireTxImpl) -> Self {
        Self {
            task_table: TopicTaskTable(&TOPIC_TASK_STATE),
            logger,
            tx,
        }
    }
}

pub struct DispatchSpawnContext {
    pub task_table: TopicTaskTable,
    pub logger: &'static RefCell<DefmtConsumer>,
}

impl SpawnContext for DispatchContext {
    type SpawnCtxt = DispatchSpawnContext;

    fn spawn_ctxt(&mut self) -> Self::SpawnCtxt {
        DispatchSpawnContext {
            task_table: self.task_table,
            logger: self.logger,
        }
    }
}
