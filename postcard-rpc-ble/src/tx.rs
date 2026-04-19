use core::sync::atomic::{AtomicU8, Ordering};

use maitake_sync::{Mutex, RwLock};
use postcard::ser_flavors::Cobs;
use postcard::{
    Serializer,
    ser_flavors::{Flavor, Slice},
};
use postcard_rpc::header::VarHeader;
use postcard_rpc::server::{WireTx, WireTxErrorKind};
use serde::Serialize;
use trouble_host::{gatt::GattConnection, prelude::*};

use crate::PrpcBleStorage;
use crate::ack::{AckQueue, WakeReason};
use crate::gatt::{BytesCh, GATT_OVERHEAD, GattServerRpc};

pub(crate) trait ChunkSender {
    async fn send_chunk_unreliable(&self, data: &[u8]) -> Result<(), ()>;
    async fn send_chunk_reliable(&self, data: &[u8]) -> Result<(), ()>;
    fn chunk_size(&self) -> usize;
}

struct BleChunkSender<'a, 'stack, 'server> {
    gatt_conn: &'a GattConnection<'stack, 'server, DefaultPacketPool>,
    tx_not_acked: &'a Characteristic<BytesCh>,
    tx_acked: &'a Characteristic<BytesCh>,
}

impl ChunkSender for BleChunkSender<'_, '_, '_> {
    async fn send_chunk_reliable(&self, data: &[u8]) -> Result<(), ()> {
        let value = BytesCh::try_from_slice(data).ok_or(())?;
        self.tx_acked
            .indicate(self.gatt_conn, &value)
            .await
            .map_err(|err| defmt::error!("Error sending the indication {}", err))
    }

    async fn send_chunk_unreliable(&self, data: &[u8]) -> Result<(), ()> {
        let value = BytesCh::try_from_slice(data).ok_or(())?;
        self.tx_not_acked
            .notify(self.gatt_conn, &value)
            .await
            .map_err(|err| defmt::error!("Error sending the notification {}", err))
    }

    fn chunk_size(&self) -> usize {
        self.gatt_conn.raw().att_mtu() as usize - GATT_OVERHEAD
    }
}

#[derive(Debug, defmt::Format)]
struct TxBuffer<const SIZE: usize> {
    buf: [u8; SIZE],
    index: usize,
}

impl<const SIZE: usize> TxBuffer<SIZE> {
    const fn new() -> Self {
        Self {
            buf: [0u8; SIZE],
            index: 0,
        }
    }
    const fn reset(&mut self) {
        self.index = 0;
    }
}

/// Groups the TX buffer and active-writer count used for batching
/// multiple serialized messages before flushing over BLE indications.
pub(crate) struct TxBatchState<const BUFFER_SIZE: usize> {
    buffer: Mutex<TxBuffer<BUFFER_SIZE>>,
    active_writers: AtomicU8,
}

impl<const BUFFER_SIZE: usize> TxBatchState<BUFFER_SIZE> {
    pub const fn new() -> Self {
        Self {
            buffer: Mutex::new(TxBuffer::new()),
            active_writers: AtomicU8::new(0),
        }
    }

    fn announce_tx_intent(&self) -> TxSendIntentGuard<'_> {
        self.active_writers.fetch_add(1, Ordering::SeqCst);
        TxSendIntentGuard {
            r: &self.active_writers,
        }
    }

    fn am_i_only_writer(&self) -> bool {
        let count = self.active_writers.load(Ordering::SeqCst);
        assert!(count > 0, "Writer counter is messed up, this is bad");
        count == 1
    }
}

struct TxSendIntentGuard<'s> {
    r: &'s AtomicU8,
}

impl<'s> Drop for TxSendIntentGuard<'s> {
    fn drop(&mut self) {
        defmt::debug!("Dropping tx intent");
        self.r.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(Clone)]
pub struct BleWireTx<'storage, 'stack, 'server, const BUFFER_SIZE: usize> {
    server: &'storage GattServerRpc<'server>,
    ack_queue: &'storage AckQueue,
    gatt_conn: &'storage RwLock<Option<GattConnection<'stack, 'server, DefaultPacketPool>>>,
    batch: &'storage TxBatchState<BUFFER_SIZE>,
}

impl<'storage, 'stack, 'server, const BUFFER_SIZE: usize>
    BleWireTx<'storage, 'stack, 'server, BUFFER_SIZE>
{
    pub(crate) fn new<const CH_SIZE: usize>(
        storage: &'storage PrpcBleStorage<'stack, 'server, CH_SIZE, BUFFER_SIZE>,
    ) -> Self {
        Self {
            server: &storage.server,
            ack_queue: &storage.ack_queue,
            gatt_conn: &storage.gatt_conn,
            batch: &storage.tx_batch,
        }
    }
    pub async fn get_current_mtu(&self) -> Option<u16> {
        let guard = self.gatt_conn.read().await;
        let Some(ref conn) = *guard else {
            return None;
        };
        Some(conn.raw().att_mtu())
    }
}

impl<'storage, 'stack, 'server, const BUFFER_SIZE: usize> WireTx
    for BleWireTx<'storage, 'stack, 'server, BUFFER_SIZE>
{
    type Error = WireTxErrorKind;

    async fn send<T: serde::Serialize + ?Sized>(
        &self,
        hdr: postcard_rpc::header::VarHeader,
        msg: &T,
    ) -> Result<(), Self::Error> {
        let guard = self.gatt_conn.read().await;
        let ack_q = self.ack_queue;

        let Some(ref gatt_conn) = *guard else {
            return Err(WireTxErrorKind::ConnectionClosed);
        };

        let sender = BleChunkSender {
            gatt_conn,
            tx_not_acked: &self.server.rpc_service.tx_not_acked,
            tx_acked: &self.server.rpc_service.tx_acked,
        };

        batched_send(&sender, ack_q, &self.batch, &hdr, msg).await
    }

    async fn send_raw(&self, _buf: &[u8]) -> Result<(), Self::Error> {
        unimplemented!("we dont need to pass along messages yet")
    }

    async fn send_log_str(
        &self,
        _kkind: postcard_rpc::header::VarKeyKind,
        _s: &str,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn send_log_fmt<'a>(
        &self,
        _kkind: postcard_rpc::header::VarKeyKind,
        _a: core::fmt::Arguments<'a>,
    ) -> Result<(), Self::Error> {
        todo!()
    }
}

fn serialize_into_buffer<T: serde::Serialize + ?Sized>(
    buffer: &mut [u8],
    hdr: &VarHeader,
    msg: &T,
) -> Option<usize> {
    let mut flavor = flava_flav(buffer).ok()?;
    header_to_flavor(hdr, &mut flavor).ok()?;
    let used = body_to_flavor(msg, flavor).ok()?;
    Some(used.len())
}

async fn flush_buffer<const SIZE: usize>(
    sender: &impl ChunkSender,
    tx_buffer: &mut TxBuffer<SIZE>,
    ack_q: &AckQueue,
) -> WakeReason {
    let mut flush_iter = tx_buffer.buf[..tx_buffer.index]
        .chunks(sender.chunk_size())
        .peekable();
    while let Some(ch) = flush_iter.next() {
        let chunk_data = ch;
        let is_last = flush_iter.peek().is_none();
        let needs_ack = chunk_data.iter().any(|b| *b == 0);
        if !is_last {
            defmt::debug!("Using unreliable send before the last chunk");
            let res = if needs_ack {
                match ack_q
                    .wait_once_with(async || {
                        match ack_q.set_data_inflight() {
                            Ok(_) => {}
                            Err(_) => {
                                return ack_q.process_disconnect();
                            }
                        }
                        match sender.send_chunk_reliable(chunk_data).await {
                            Ok(_) => {}
                            Err(_) => {
                                defmt::debug!("Error sending chunk");
                                ack_q.process_disconnect();
                            }
                        }
                    })
                    .await
                {
                    WakeReason::Ack => Ok(()),
                    WakeReason::Disconnected => Err(()),
                }
            } else {
                sender.send_chunk_unreliable(chunk_data).await
            };
            if res.is_err() {
                defmt::error!("Error sending chunk using a ble notification");
                ack_q.process_disconnect();
                tx_buffer.reset();
                return WakeReason::Disconnected;
            }
        } else {
            defmt::debug!("Using reliable send for the last chunk");
            match ack_q
                .wait_once_with(async || {
                    match ack_q.set_data_inflight() {
                        Ok(_) => {}
                        Err(_) => {
                            return ack_q.process_disconnect();
                        }
                    }
                    match sender.send_chunk_reliable(chunk_data).await {
                        Ok(_) => {}
                        Err(_) => {
                            defmt::debug!("Error sending chunk");
                            ack_q.process_disconnect();
                        }
                    }
                })
                .await
            {
                WakeReason::Ack => {
                    continue;
                }
                WakeReason::Disconnected => {
                    tx_buffer.reset();
                    return WakeReason::Disconnected;
                }
            }
        }
    }
    tx_buffer.reset();
    WakeReason::Ack
}

async fn batched_send<S: ChunkSender, T: Serialize + ?Sized, const BUFFER_SIZE: usize>(
    sender: &S,
    ack_q: &AckQueue,
    batch: &TxBatchState<BUFFER_SIZE>,
    hdr: &VarHeader,
    msg: &T,
) -> Result<(), WireTxErrorKind> {
    let send_intent_guard = batch.announce_tx_intent();
    let mut buffer_guard = batch.buffer.lock().await;

    // Load bearing shadowing: required to change the drop order.
    // We always want to decrease the writer counter before releasing the buffer lock.
    let send_intent_guard = send_intent_guard;

    defmt::debug!("Got the buffer lock to send");
    match ack_q.wait_if_inflight().await {
        WakeReason::Ack => {}
        WakeReason::Disconnected => return Err(WireTxErrorKind::ConnectionClosed),
    }

    let mut need_flush = false;
    loop {
        let begin = buffer_guard.index;
        match serialize_into_buffer(&mut buffer_guard.buf[begin..], hdr, msg) {
            None => {
                if begin == 0 {
                    return Err(WireTxErrorKind::Other);
                } else {
                    match flush_buffer(sender, &mut *buffer_guard, ack_q).await {
                        WakeReason::Ack => assert!(buffer_guard.index == 0),
                        WakeReason::Disconnected => {
                            return Err(WireTxErrorKind::ConnectionClosed);
                        }
                    }
                }
            }
            Some(used_length) => {
                buffer_guard.index += used_length;
                defmt::debug!(
                    "   used {} bytes of the buffer, now the index is {}",
                    used_length,
                    buffer_guard.index
                );
                break;
            }
        };
    }

    if buffer_guard.index > sender.chunk_size() {
        need_flush = true;
    }

    if batch.am_i_only_writer() {
        need_flush = true;
    }

    if need_flush {
        match flush_buffer(sender, &mut *buffer_guard, ack_q).await {
            WakeReason::Ack => Ok(()),
            WakeReason::Disconnected => Err(WireTxErrorKind::ConnectionClosed),
        }
    } else {
        match ack_q
            .wait_once_with(async move || {
                drop(send_intent_guard);
                drop(buffer_guard);
            })
            .await
        {
            WakeReason::Ack => Ok(()),
            WakeReason::Disconnected => Err(WireTxErrorKind::ConnectionClosed),
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use std::sync::Mutex as StdMutex;
    use std::vec::Vec;

    use super::*;
    use crate::ack::AckQueue;

    struct MockChunkSender {
        chunk_size: usize,
        sent_chunks: StdMutex<Vec<Vec<u8>>>,
        ack_queue: *const AckQueue,
        fail_on_chunk: Option<usize>,
    }

    // SAFETY: only used in single-threaded tokio tests
    unsafe impl Send for MockChunkSender {}
    unsafe impl Sync for MockChunkSender {}

    impl MockChunkSender {
        fn new(chunk_size: usize, ack_queue: &AckQueue) -> Self {
            Self {
                chunk_size,
                sent_chunks: StdMutex::new(Vec::new()),
                ack_queue: ack_queue as *const AckQueue,
                fail_on_chunk: None,
            }
        }

        fn with_fail_on_chunk(mut self, n: usize) -> Self {
            self.fail_on_chunk = Some(n);
            self
        }

        fn sent_chunks(&self) -> Vec<Vec<u8>> {
            self.sent_chunks.lock().unwrap().clone()
        }

        fn ack_q(&self) -> &AckQueue {
            // SAFETY: lifetime managed by test scope
            unsafe { &*self.ack_queue }
        }

        async fn send_chunk(&self, data: &[u8]) -> Result<(), ()> {
            let mut sent = self.sent_chunks.lock().unwrap();
            if let Some(n) = self.fail_on_chunk {
                if sent.len() >= n {
                    return Err(());
                }
            }
            sent.push(data.to_vec());
            // Simulate the BLE stack confirming the indication
            self.ack_q().process_ack();
            Ok(())
        }
    }

    impl ChunkSender for MockChunkSender {
        fn chunk_size(&self) -> usize {
            self.chunk_size
        }

        async fn send_chunk_unreliable(&self, data: &[u8]) -> Result<(), ()> {
            self.send_chunk(data).await
        }

        async fn send_chunk_reliable(&self, data: &[u8]) -> Result<(), ()> {
            self.send_chunk(data).await
        }
    }

    fn make_hdr() -> VarHeader {
        VarHeader {
            key: postcard_rpc::header::VarKey::Key8(postcard_rpc::Key::for_path::<()>("test")),
            seq_no: postcard_rpc::header::VarSeq::Seq1(0),
        }
    }

    #[tokio::test]
    async fn test_single_writer_flushes_immediately() {
        let aq = AckQueue::new();
        aq.process_connected();
        let batch = TxBatchState::<1024>::new();
        let sender = MockChunkSender::new(252, &aq);
        let hdr = make_hdr();

        let result = batched_send(&sender, &aq, &batch, &hdr, &42u32).await;
        assert!(result.is_ok());
        assert!(!sender.sent_chunks().is_empty());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let hdr = make_hdr();
        let mut buf = [0u8; 256];
        let len = serialize_into_buffer(&mut buf, &hdr, &42u32);
        assert!(len.is_some());
        let len = len.unwrap();
        assert!(len > 0);

        // The output should be COBS-encoded: terminated by a 0x00 sentinel
        assert_eq!(buf[len - 1], 0x00);
    }

    #[test]
    fn test_serialize_into_buffer_too_small() {
        let hdr = make_hdr();
        let mut buf = [0u8; 2]; // too small for header + body + COBS overhead
        let len = serialize_into_buffer(&mut buf, &hdr, &42u32);
        assert!(len.is_none());
    }

    #[tokio::test]
    async fn test_message_too_large_returns_error() {
        let aq = AckQueue::new();
        aq.process_connected();
        let batch = TxBatchState::<1024>::new();
        let sender = MockChunkSender::new(252, &aq);
        let hdr = make_hdr();

        // The buffer is 1024 bytes. With VarHeader + COBS overhead, a message body
        // near that size won't fit even in an empty buffer.
        let big_msg: &[u8] = &[0xFFu8; 1020];
        let result = batched_send(&sender, &aq, &batch, &hdr, big_msg).await;
        assert!(matches!(result, Err(WireTxErrorKind::Other)));
    }

    #[tokio::test]
    async fn test_flush_chunks_by_mtu() {
        let aq = AckQueue::new();
        aq.process_connected();
        let batch = TxBatchState::<1024>::new();
        // Use a small chunk size to force multiple chunks
        let sender = MockChunkSender::new(10, &aq);
        let hdr = make_hdr();

        let result = batched_send(&sender, &aq, &batch, &hdr, &42u32).await;
        assert!(result.is_ok());

        let chunks = sender.sent_chunks();
        // With chunk_size=10, the serialized message should be split
        // Each chunk should be at most 10 bytes
        for chunk in &chunks {
            assert!(chunk.len() <= 10);
        }
    }

    #[tokio::test]
    async fn test_disconnect_before_send() {
        let aq = AckQueue::new();
        // Not connected — should fail immediately
        let batch = TxBatchState::<1024>::new();
        let sender = MockChunkSender::new(252, &aq);
        let hdr = make_hdr();

        let result = batched_send(&sender, &aq, &batch, &hdr, &42u32).await;
        assert!(matches!(result, Err(WireTxErrorKind::ConnectionClosed)));
    }

    #[tokio::test]
    async fn test_disconnect_during_flush() {
        let aq = AckQueue::new();
        aq.process_connected();
        let batch = TxBatchState::<1024>::new();
        // Fail on the first chunk — mock will return Err which triggers process_disconnect
        let sender = MockChunkSender::new(10, &aq).with_fail_on_chunk(0);
        let hdr = make_hdr();

        let result = batched_send(&sender, &aq, &batch, &hdr, &42u32).await;
        assert!(matches!(result, Err(WireTxErrorKind::ConnectionClosed)));
    }

    #[tokio::test]
    async fn test_buffer_overflow_triggers_flush() {
        let aq = AckQueue::new();
        aq.process_connected();
        let batch = TxBatchState::<1024>::new();
        let sender = MockChunkSender::new(252, &aq);
        let hdr = make_hdr();

        // Send many messages to fill the 1024-byte buffer, forcing a mid-batch flush
        // A single u32 message with COBS encoding is ~10 bytes
        // Send enough to exceed 1024 bytes
        for _ in 0..100 {
            let result = batched_send(&sender, &aq, &batch, &hdr, &42u32).await;
            assert!(result.is_ok());
        }

        // Should have sent multiple rounds of chunks
        assert!(sender.sent_chunks().len() > 1);
    }

    #[tokio::test]
    async fn test_two_writers_batch() {
        // With two concurrent writers, the first should NOT flush (because
        // am_i_only_writer is false), and the second should flush both.
        //
        // Since maitake_sync::Mutex is cooperative and we're single-threaded,
        // we use tokio::join! to interleave the two futures.
        let aq = AckQueue::new();
        aq.process_connected();
        let batch = TxBatchState::<1024>::new();
        let sender = MockChunkSender::new(252, &aq);

        let hdr1 = make_hdr();
        let hdr2 = make_hdr();

        let (r1, r2) = tokio::join!(
            batched_send(&sender, &aq, &batch, &hdr1, &1u32),
            batched_send(&sender, &aq, &batch, &hdr2, &2u32),
        );

        assert!(r1.is_ok());
        assert!(r2.is_ok());

        // Both messages should have been sent
        assert!(!sender.sent_chunks().is_empty());
    }

    #[tokio::test]
    async fn test_inflight_blocks_then_proceeds_on_ack() {
        let aq = AckQueue::new();
        aq.process_connected();
        let batch = TxBatchState::<1024>::new();
        let sender = MockChunkSender::new(252, &aq);
        let hdr = make_hdr();

        // Simulate data already inflight
        aq.set_data_inflight().unwrap();

        // The send should block waiting for inflight to clear.
        // We use join to concurrently clear the inflight state.
        let (send_result, _) =
            tokio::join!(batched_send(&sender, &aq, &batch, &hdr, &42u32), async {
                // Yield to let batched_send reach the inflight wait
                tokio::task::yield_now().await;
                // Clear inflight — this wakes the waiter
                aq.process_ack();
            });

        assert!(send_result.is_ok());
    }
}

fn flava_flav(buf: &'_ mut [u8]) -> Result<Cobs<Slice<'_>>, WireTxErrorKind> {
    Cobs::try_new(Slice::new(buf)).map_err(|_| WireTxErrorKind::Other)
}

fn header_to_flavor(hdr: &VarHeader, flava: &mut Cobs<Slice<'_>>) -> Result<(), WireTxErrorKind> {
    let mut hdr_buf = [0u8; 1 + 4 + 8];
    let (used, _unused) = hdr
        .write_to_slice(&mut hdr_buf)
        .ok_or(WireTxErrorKind::Other)?;
    flava.try_extend(used).map_err(|_| WireTxErrorKind::Other)?;
    Ok(())
}

fn body_to_flavor<'a, T: Serialize + ?Sized>(
    msg: &T,
    flava: Cobs<Slice<'a>>,
) -> Result<&'a [u8], WireTxErrorKind> {
    let mut serializer = Serializer { output: flava };
    msg.serialize(&mut serializer)
        .map_err(|_| WireTxErrorKind::Other)?;
    let used = serializer
        .output
        .finalize()
        .map_err(|_| WireTxErrorKind::Other)?;
    Ok(used)
}
