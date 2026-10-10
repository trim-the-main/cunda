use core::cell::RefCell;

use maitake_sync::{Mutex, RwLock, WaitCell};
use postcard::ser_flavors::Cobs;
use postcard::{
    Serializer,
    ser_flavors::{Flavor, Slice},
};
use postcard_rpc::header::VarHeader;
use postcard_rpc::server::{WireTx, WireTxErrorKind};
use serde::Serialize;
use trouble_host::prelude::*;

use crate::PrpcBleStorage;
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
struct TxBuffer<const SIZE: usize>([u8; SIZE]);

impl<const SIZE: usize> TxBuffer<SIZE> {
    const fn new() -> Self {
        Self([0u8; SIZE])
    }
}

pub(crate) struct TxBufferShared<const BUFFER_SIZE: usize> {
    buffer: Mutex<TxBuffer<BUFFER_SIZE>>,
}

impl<const BUFFER_SIZE: usize> TxBufferShared<BUFFER_SIZE> {
    pub const fn new() -> Self {
        Self {
            buffer: Mutex::new(TxBuffer::new()),
        }
    }
}

#[derive(Clone)]
pub struct BleWireTx<'storage, 'stack, 'server, const BUFFER_SIZE: usize> {
    server: &'storage GattServerRpc<'server>,
    ack_queue: &'storage RefCell<WaitCell>,
    gatt_conn: &'storage RwLock<Option<GattConnection<'stack, 'server, DefaultPacketPool>>>,
    buffer: &'storage TxBufferShared<BUFFER_SIZE>,
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
            buffer: &storage.tx_buffer,
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

        let Some(ref gatt_conn) = *guard else {
            return Err(WireTxErrorKind::ConnectionClosed);
        };
        let ack_q = self.ack_queue.borrow();

        let sender = BleChunkSender {
            gatt_conn,
            tx_not_acked: &self.server.rpc_service.tx_not_acked,
            tx_acked: &self.server.rpc_service.tx_acked,
        };

        let mut buffer_guard = self.buffer.buffer.lock().await;
        let to_send = match serialize_into_buffer(&mut buffer_guard.0, &hdr, msg) {
            None => {
                return Err(WireTxErrorKind::Other);
            }
            Some(used_length) => {
                defmt::debug!("   used {} bytes of the buffer", used_length,);
                &buffer_guard.0[..used_length]
            }
        };
        let mut peekable_chunks = to_send.chunks(sender.chunk_size()).peekable();
        while let Some(chunk) = peekable_chunks.next() {
            let is_last = peekable_chunks.peek().is_none();
            if is_last {
                let wait = ack_q.subscribe().await;

                match sender.send_chunk_reliable(chunk).await {
                    Ok(_) => {}
                    Err(_) => return Err(WireTxErrorKind::ConnectionClosed),
                }
                wait.await.map_err(|_| WireTxErrorKind::ConnectionClosed)?;
            } else {
                sender
                    .send_chunk_unreliable(chunk)
                    .await
                    .map_err(|_| WireTxErrorKind::ConnectionClosed)?;
            }
        }
        Ok(())
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
