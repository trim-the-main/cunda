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

#[derive(Debug, defmt::Format)]
struct TxBuffer {
    buf: [u8; 1024],
    index: usize,
}

impl TxBuffer {
    const fn new() -> Self {
        Self {
            buf: [0u8; 1024],
            index: 0,
        }
    }
    const fn reset(&mut self) {
        self.index = 0;
    }
}

/// Groups the TX buffer and active-writer count used for batching
/// multiple serialized messages before flushing over BLE indications.
pub(crate) struct TxBatchState {
    buffer: Mutex<TxBuffer>,
    active_writers: AtomicU8,
}

impl TxBatchState {
    pub const fn new() -> Self {
        Self {
            buffer: Mutex::new(TxBuffer::new()),
            active_writers: AtomicU8::new(0),
        }
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
pub struct BleWireTx<'storage, 'stack, 'server> {
    server: &'storage GattServerRpc<'server>,
    ack_queue: &'storage AckQueue,
    gatt_conn: &'storage RwLock<Option<GattConnection<'stack, 'server, DefaultPacketPool>>>,
    batch: &'storage TxBatchState,
}

impl<'storage, 'stack, 'server> BleWireTx<'storage, 'stack, 'server> {
    pub(crate) fn new<const CH_SIZE: usize>(
        storage: &'storage PrpcBleStorage<'stack, 'server, CH_SIZE>,
    ) -> Self {
        Self {
            server: &storage.server,
            ack_queue: &storage.ack_queue,
            gatt_conn: &storage.gatt_conn,
            batch: &storage.tx_batch,
        }
    }
    fn announce_tx_intent(&self) -> TxSendIntentGuard<'_> {
        self.batch.active_writers.fetch_add(1, Ordering::SeqCst);
        TxSendIntentGuard {
            r: &self.batch.active_writers,
        }
    }

    fn get_mtu(gatt_conn: &GattConnection<'_, '_, DefaultPacketPool>) -> u16 {
        gatt_conn.raw().att_mtu()
    }

    pub async fn get_current_mtu(&self) -> Option<u16> {
        let guard = self.gatt_conn.read().await;
        let Some(ref conn) = *guard else {
            return None;
        };
        Some(Self::get_mtu(conn))
    }

    fn serialize_into_buffer<T: serde::Serialize + ?Sized>(
        &self,
        buffer: &mut [u8],
        hdr: &VarHeader,
        msg: &T,
    ) -> Option<usize> {
        let mut flavor = flava_flav(buffer).ok()?;
        header_to_flavor(&hdr, &mut flavor).ok()?;
        let used = body_to_flavor(msg, flavor).ok()?;
        Some(used.len())
    }

    fn msg_chunk_size(gatt_conn: &GattConnection<'_, '_, DefaultPacketPool>) -> usize {
        Self::get_mtu(gatt_conn) as usize - GATT_OVERHEAD
    }

    async fn flush_buffer(
        gatt_conn: &GattConnection<'_, '_, DefaultPacketPool>,
        tx_buffer: &mut TxBuffer,
        ack_q: &AckQueue,
        tx_characteristic: &Characteristic<BytesCh>,
    ) -> WakeReason {
        for ch in tx_buffer.buf[..tx_buffer.index].chunks(Self::msg_chunk_size(gatt_conn)) {
            let value = BytesCh::try_from_slice(ch).expect("Max MTU is exceeded.");

            match ack_q
                .wait_once_with(async || {
                    match ack_q.set_data_inflight() {
                        Ok(_) => {}
                        Err(_) => {
                            return ack_q.process_disconnect();
                        }
                    }
                    match tx_characteristic
                        .indicate(gatt_conn, &value)
                        .await
                        .map_err(|_| {
                            defmt::debug!("Error sending indication on the `TX` characteristic");
                            WireTxErrorKind::ConnectionClosed
                        }) {
                        Ok(_) => {}
                        Err(_) => ack_q.process_disconnect(),
                    }

                    defmt::debug!("SENT NORMAL {}", value);
                })
                .await
            {
                WakeReason::Ack => {
                    defmt::debug!("Got ack for {}", value);
                    continue;
                }
                WakeReason::Disconnected => {
                    tx_buffer.reset();
                    return WakeReason::Disconnected;
                }
            }
        }
        tx_buffer.reset();
        WakeReason::Ack
    }

    fn am_i_only_writer(&self) -> bool {
        let count = self.batch.active_writers.load(Ordering::SeqCst);
        assert!(count > 0, "Writer counter is messed up, this is bad");
        count == 1
    }
}

impl<'storage, 'stack, 'server> WireTx for BleWireTx<'storage, 'stack, 'server> {
    type Error = WireTxErrorKind;

    async fn send<T: serde::Serialize + ?Sized>(
        &self,
        hdr: postcard_rpc::header::VarHeader,
        msg: &T,
    ) -> Result<(), Self::Error> {
        let guard = self.gatt_conn.read().await;
        let ack_q = self.ack_queue;
        let tx_char = &self.server.rpc_service.tx;

        let Some(ref gatt_conn) = *guard else {
            return Err(WireTxErrorKind::ConnectionClosed);
        };

        let send_intent_guard = self.announce_tx_intent();
        let mut buffer_guard = self.batch.buffer.lock().await;

        // Load bearing shadowing: required to change the drop order.
        // We always want to decrease the writer counter before releasing the buffer lock.
        let send_intent_guard = send_intent_guard;

        defmt::debug!("Got the buffer lock");
        if ack_q.is_data_inflight() {
            defmt::debug!("waiting for all the inflight data to stop");
            ack_q.wait_once_with(async || {}).await;
            defmt::debug!("acks arrived");
        }

        if !ack_q.is_connected() {
            defmt::debug!("disconnected");
            buffer_guard.reset();
            return Err(WireTxErrorKind::ConnectionClosed);
        }

        let mut need_flush = false;
        loop {
            let begin = buffer_guard.index;
            match self.serialize_into_buffer(&mut buffer_guard.buf[begin..], &hdr, msg) {
                None => {
                    if begin == 0 {
                        return Err(WireTxErrorKind::Other);
                    } else {
                        match Self::flush_buffer(gatt_conn, &mut *buffer_guard, ack_q, tx_char)
                            .await
                        {
                            WakeReason::Ack => assert!(buffer_guard.index == 0),
                            WakeReason::Disconnected => {
                                return Err(WireTxErrorKind::ConnectionClosed);
                            }
                        }
                    }
                }
                Some(used_length) => {
                    buffer_guard.index += used_length;
                    break;
                }
            };
        }

        if buffer_guard.index > Self::msg_chunk_size(gatt_conn) as usize {
            need_flush = true;
        }

        if self.am_i_only_writer() {
            need_flush = true;
        }

        if need_flush {
            match Self::flush_buffer(gatt_conn, &mut *buffer_guard, ack_q, tx_char).await {
                WakeReason::Ack => Ok(()),
                WakeReason::Disconnected => return Err(WireTxErrorKind::ConnectionClosed),
            }
        } else {
            match ack_q
                .wait_once_with(async move || {
                    drop(send_intent_guard);
                    drop(buffer_guard);
                })
                .await
            {
                WakeReason::Ack => return Ok(()),
                WakeReason::Disconnected => return Err(WireTxErrorKind::ConnectionClosed),
            }
        }
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
