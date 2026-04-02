use core::ops::DerefMut;
use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_time::Duration;
use maitake_sync::{Mutex, RwLock, WaitQueue};
use postcard::ser_flavors::Cobs;
use postcard::{
    Serializer,
    ser_flavors::{Flavor, Slice},
};
use postcard_rpc::header::VarHeader;
use postcard_rpc::server::{
    self, AsWireTxErrorKind, Dispatch, WireRxErrorKind, WireTx, WireTxErrorKind,
};
use serde::Serialize;
use static_cell::{ConstStaticCell, StaticCell};
use trouble_host::{gatt::GattConnection, prelude::*};

use crate::ble::constants::GATT_MSG_OVERHEAD;
use crate::rpc::accumulator::{AccumulatorYieldError, RX_BUF_SIZE};
use crate::rpc::dispatcher::BleDispatcher;
use crate::{
    ble::{BleRpcMessageBuffer, GattServerRpc, PeriphRole},
    rpc::accumulator::Accumulator,
};

struct BleWireInner {
    server: GattServerRpc<'static>, // Holds the rpc_service and the characteristics
    gatt_conn: Option<GattConn>,    // Connection object
    ack_queue: AckQueue,            // tx tasks wait  here, rx task wake upon ack
}

impl BleWireInner {
    fn new() -> Self {
        Self {
            server: GattServerRpc::new_with_config(GapConfig::Peripheral(PeripheralConfig {
                name: "PostcardRPC",
                appearance: &appearance::power_device::GENERIC_POWER_DEVICE,
            }))
            .expect("Failed to create new server"),
            gatt_conn: None,
            ack_queue: AckQueue::new(),
        }
    }

    fn store_connection(
        &'static mut self,
        conn: Connection<'static, DefaultPacketPool>,
    ) -> Result<(), Error> {
        match conn.with_attribute_server(&self.server) {
            Ok(gatt_conn) => {
                self.gatt_conn = Some(gatt_conn);
                self.ack_queue.process_connected();
                Ok(())
            }
            Err(err) => {
                self.gatt_conn = None;
                Err(err)
            }
        }
    }
}
type GattConn = GattConnection<'static, 'static, DefaultPacketPool>;
pub struct BleWireStorage {
    inner: StaticCell<RwLock<BleWireInner>>,
    tx_buffer: ConstStaticCell<Mutex<TxBuffer>>,
    active_tx_count: ConstStaticCell<AtomicU8>,
}

impl BleWireStorage {
    pub const fn new() -> Self {
        Self {
            inner: StaticCell::new(),
            tx_buffer: ConstStaticCell::new(Mutex::new(TxBuffer::new())),
            active_tx_count: ConstStaticCell::new(AtomicU8::new(0)),
        }
    }

    pub fn init(
        &'static self,
        periph_role: PeriphRole,
        ch: &'static Channel<CriticalSectionRawMutex, IncomingData, 16>,
    ) -> (BleWireRx, BleWireTx) {
        let wire = &*self.inner.init(RwLock::new(BleWireInner::new()));

        let rx = BleWireRx::new(wire, periph_role, ch.sender());
        let tx = BleWireTx::new(wire, self.tx_buffer.take(), self.active_tx_count.take());
        (rx, tx)
    }
}

// Wrapper type to be able to send an rx buffer to another task. We receive data
// on the rx characteristic as a WriteEvent that holds on to a Packet which is owned
// by the PacketPool. We would like to send this Packet to another task (the dispatcher)
// where its data is accumulated in a cobs decoder. Dispatcher task will copy the
// data, free the packet buffer back to the PacketPool and then handle the message.
pub(crate) struct IncomingData {
    w_event: WriteEvent<'static, 'static, DefaultPacketPool>,
}
impl IncomingData {
    pub(crate) fn new(w_event: WriteEvent<'static, 'static, DefaultPacketPool>) -> Self {
        Self { w_event }
    }
    pub(crate) fn data(&self) -> &[u8] {
        self.w_event.data()
    }
}
unsafe impl Send for IncomingData {}

// Rx and Tx shares a reference through RwLock to the GattServer and GattConnection
// Tracking the connection state is the job of Rx handle. When we receive the
// GattEvent that says we're disconnected, we try to acquire the Write lock to
// the shared struct and delete the connection object. Until we delete it there may
// be other Tx requests but the Ble library should also fail them with connection closed
// when they try to send. If Tx handles don't try to send during the disconnected window
// and we have a new connection, Tx handles will succeed to send to the new connection.
// Note: There's only one Rx handle but multiple Tx handles (one for sending dispatcher
// reponses and clones of the tx handle for each topic task)

pub struct BleWireRx {
    inner: &'static RwLock<BleWireInner>,

    // Rx side owns this object and it is used for advertising (in wait_connection)
    periph_role: PeriphRole,

    // During the connection loop, we operate under the Read lock on the inner object
    // (we share it with BleWireTx handles). There is one Rx handle but potentially
    // multiple Tx handles. When the connection drops, Rx handle will receive a
    // disconnection event
    disconnected: bool,
    dispatcher_channel: Sender<'static, CriticalSectionRawMutex, IncomingData, 16>,
}

impl BleWireRx {
    const ADV_TIMEOUT: Duration = Duration::from_secs(2);
    fn new(
        inner: &'static RwLock<BleWireInner>,
        periph_role: PeriphRole,
        dispatcher_channel: Sender<'static, CriticalSectionRawMutex, IncomingData, 16>,
    ) -> Self {
        Self {
            inner,
            periph_role,
            disconnected: true,
            dispatcher_channel,
        }
    }

    async fn handle_gatt_events(
        &mut self,
        rx_handle: u16,
        gatt_conn: &GattConnection<'static, 'static, DefaultPacketPool>,
        ack_q: &AckQueue,
    ) -> Result<(), WireRxErrorKind> {
        loop {
            match gatt_conn.next().await {
                GattConnectionEvent::Disconnected { reason } => {
                    defmt::info!("Got disconnected event {}", reason);
                    self.disconnected = true;
                    ack_q.process_disconnect();
                    return Err(WireRxErrorKind::ConnectionClosed);
                }
                GattConnectionEvent::PhyUpdated { tx_phy, rx_phy } => {
                    defmt::info!("GattConnectionEvent::PhyUpdates {} {}", tx_phy, rx_phy);
                }
                GattConnectionEvent::ConnectionParamsUpdated {
                    conn_interval,
                    peripheral_latency,
                    supervision_timeout,
                } => {
                    defmt::info!(
                        "GattConnectionEvent::ConnectionParamsUpdated conn_interval: {} peripheral_latency: {} supervision_timeout: {}",
                        conn_interval,
                        peripheral_latency,
                        supervision_timeout
                    );
                }
                GattConnectionEvent::RequestConnectionParams {
                    min_connection_interval,
                    max_connection_interval,
                    max_latency,
                    supervision_timeout,
                } => {
                    defmt::info!(
                        "GattConnectionEvent::RequestConnectionParams min_connection_interval: {} max_connection_interval: {} max_latency: {} supervision_timeout: {}",
                        min_connection_interval,
                        max_connection_interval,
                        max_latency,
                        supervision_timeout
                    );
                }
                GattConnectionEvent::DataLengthUpdated {
                    max_tx_octets,
                    max_tx_time,
                    max_rx_octets,
                    max_rx_time,
                } => {
                    defmt::info!(
                        "GattConnectionEvent::DataLengthUpdated max_tx_octets: {} max_tx_time: {} max_rx_octets: {} max_rx_time: {}",
                        max_tx_octets,
                        max_tx_time,
                        max_rx_octets,
                        max_rx_time,
                    );
                }
                GattConnectionEvent::Gatt { event } => match event {
                    GattEvent::Read(read_event) => {
                        defmt::warn!(
                            "Unexpected read event from the central device {}",
                            read_event.handle()
                        );
                    }
                    GattEvent::Write(write_event) => {
                        if write_event.handle() == rx_handle {
                            defmt::debug!("Received data: {:?}", write_event.data());
                            self.dispatcher_channel
                                .send(IncomingData::new(write_event))
                                .await;
                            return Ok(());
                        } else {
                            defmt::warn!(
                                "Unexpected GattEvent::Write received at {} instead of {}",
                                write_event.handle(),
                                rx_handle,
                            );
                        }
                    }
                    GattEvent::Other(other_event) => match other_event.payload().incoming() {
                        trouble_host::att::AttClient::Confirmation(_) => ack_q.process_ack(),
                        _ => {}
                    },
                },
            }
        }
    }

    async fn advertise_once(
        &mut self,
        adv_name: &str,
    ) -> Result<Connection<'static, DefaultPacketPool>, Error> {
        let mut advertiser_data = [0; 31];
        let adv_size = AdStructure::encode_slice(
            &[
                AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
                AdStructure::CompleteLocalName(adv_name.as_bytes()),
                AdStructure::ManufacturerSpecificData {
                    company_identifier: (0x8472), // TIRB
                    payload: (&[]),
                },
            ],
            &mut advertiser_data[..],
        )
        .expect("Failed encoding advertiser data.");
        let mut params: AdvertisementParameters = Default::default();
        params.timeout = Some(Self::ADV_TIMEOUT);

        defmt::info!(
            "Advertising and waiting for a connection adv_size {}",
            adv_size
        );
        let advertiser = self
            .periph_role
            .advertise(
                &params,
                Advertisement::ConnectableScannableUndirected {
                    adv_data: &advertiser_data[..adv_size],
                    scan_data: &[],
                },
            )
            .await
            .expect("Advertising failed, panicing");

        advertiser.accept().await
    }

    pub async fn advertise(&mut self) {
        loop {
            let sys_config = crate::storage::SYSTEM_CONFIG.get_or_default().await;
            match self.advertise_once(&sys_config.ble_adv_name).await {
                Ok(connection) => {
                    // We got a connection, try to put it in the static storage
                    let mut guard = self.inner.write().await;
                    let inner = guard.deref_mut();
                    assert!(inner.gatt_conn.is_none());

                    // SAFETY: We have the exclusive reference to the inner storage which
                    // must be declared static.
                    let inner: &'static mut BleWireInner = unsafe { core::mem::transmute(inner) };

                    if inner.store_connection(connection).is_err() {
                        defmt::warn!(
                            "Failed to create a GattConnection from a Connection and GattRpcServer"
                        );
                        continue;
                    } else {
                        return;
                    }
                }
                Err(err) => {
                    defmt::warn!(
                        "Keep on advertising, `advertiser.accept()` returned Error: {:?}",
                        err
                    );
                }
            }
        }
    }
    pub(crate) async fn work_on_connection(&mut self) -> Result<(), WireRxErrorKind> {
        let inner = self.inner.read().await;
        let BleWireInner {
            gatt_conn: Some(ref gatt_conn),
            ref server,
            ack_queue: ref ack_q,
        } = *inner
        else {
            return Err(WireRxErrorKind::Other);
        };

        while self
            .handle_gatt_events(server.rpc_service.rx.handle, gatt_conn, ack_q)
            .await
            .is_ok()
        {}
        return Err(WireRxErrorKind::ConnectionClosed);
    }

    pub(crate) async fn wait_connection(&mut self) {
        if !self.disconnected {
            return;
        }
        {
            // Shared gatt_conn must be stale, when we get the write lock there should not be any
            // further tx handles active.
            let mut guard = self.inner.write().await;
            let inner = guard.deref_mut();
            drop(inner.gatt_conn.take());
        }

        self.advertise().await;
        self.disconnected = false;
    }
}

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

#[derive(Debug)]
struct AckQueue {
    q: WaitQueue,
    state: AtomicUsize,
}

enum WakeReason {
    Ack,
    Disconnected,
}

impl AckQueue {
    const INFLIGHT: usize = 1 << 0;
    const CONNECTED: usize = 1 << 1;
    const DISCONNECTED: usize = 0;

    const fn new() -> Self {
        Self {
            q: WaitQueue::new(),
            state: AtomicUsize::new(0),
        }
    }
    /// Clear the inflight flag and wake up the queue
    fn process_ack(&self) {
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

    // reset the state, we have a brand new connection. We should also reset the tx buffer.
    // This function is called with the write lock on the inner storage so no concurrent
    // writers can exist.
    fn process_connected(&self) {
        defmt::debug!("Processing new connection");
        self.state.store(Self::CONNECTED, Ordering::SeqCst); // clears the INFLIGHT bit as well
    }
    fn process_disconnect(&self) {
        defmt::debug!("Processing disconnect");
        self.state.store(Self::DISCONNECTED, Ordering::SeqCst); // clears the INFLGHT bit as well
        self.q.wake_all()
    }

    // Use the subscription mechanism to eagerly wait for the wake up event before calling f().await
    // It can be used for unlocking mutexes which can wake up a task that can notify the queue.
    // The function is only called once before waiting.
    async fn wait_once_with<F: AsyncFnOnce() -> ()>(&self, f: F) -> WakeReason {
        // f (if exists) is called once between subscribe and wait
        let wait = self.q.wait();
        let mut wait = core::pin::pin!(wait);
        let _ = wait.as_mut().subscribe(); // if wait is ready here await is going to be a noop, we still call the
        f().await;
        let _ = wait.await;
        if self.state.load(Ordering::Acquire) & Self::CONNECTED == Self::CONNECTED {
            WakeReason::Ack
        } else {
            WakeReason::Disconnected
        }
    }

    // Bitflag functions
    fn set_data_inflight(&self) -> Result<(), ()> {
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

    fn is_data_inflight(&self) -> bool {
        let state = self.state.load(Ordering::SeqCst);
        state & Self::INFLIGHT == Self::INFLIGHT
    }
    fn is_connected(&self) -> bool {
        let state = self.state.load(Ordering::SeqCst);
        state & Self::CONNECTED == Self::CONNECTED
    }
}

#[derive(Clone)]
pub struct BleWireTx {
    inner: &'static RwLock<BleWireInner>, // shared between rx & txs, stores the gatt_conn
    tx_buffer: &'static Mutex<TxBuffer>,  // shared between txs
    active_tx_count: &'static AtomicU8,   // shared between txs
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

impl BleWireTx {
    fn new(
        inner: &'static RwLock<BleWireInner>,
        tx_buffer: &'static Mutex<TxBuffer>,
        active_tx_count: &'static AtomicU8,
    ) -> Self {
        Self {
            inner,
            tx_buffer,
            active_tx_count,
        }
    }

    fn announce_tx_intent(&self) -> TxSendIntentGuard<'_> {
        self.active_tx_count.fetch_add(1, Ordering::SeqCst);
        TxSendIntentGuard {
            r: &self.active_tx_count,
        }
    }

    fn get_mtu(gatt_conn: &GattConnection<'_, '_, DefaultPacketPool>) -> u16 {
        gatt_conn.raw().att_mtu()
    }

    pub(crate) async fn get_current_mtu(&self) -> Option<u16> {
        let guard = self.inner.read().await;

        let Some(ref conn) = guard.gatt_conn else {
            return None;
        };
        Some(Self::get_mtu(conn))
    }

    // Return number of bytes written to the buffer. If there's not enough
    // space we return None. In that case any number of bytes may already
    // have been written to the buffer.
    fn serialize_into_buffer<T: serde::Serialize + ?Sized>(
        &self,
        buffer: &mut [u8],
        hdr: &VarHeader,
        msg: &T,
    ) -> Option<usize> {
        let mut flavor = flava_flav(buffer).ok()?;

        // Put the header into the buffer, which will cobs encode it
        header_to_flavor(&hdr, &mut flavor).ok()?;

        // Now do normal serialization (and cobs encoding)
        let used = body_to_flavor(msg, flavor).ok()?;
        Some(used.len())
    }

    fn msg_chunk_size(gatt_conn: &GattConnection<'_, '_, DefaultPacketPool>) -> usize {
        Self::get_mtu(gatt_conn) as usize - GATT_MSG_OVERHEAD
    }

    async fn flush_buffer(
        gatt_conn: &GattConnection<'_, '_, DefaultPacketPool>,
        tx_buffer: &mut TxBuffer,
        ack_q: &AckQueue,
        tx_characteristic: &Characteristic<BleRpcMessageBuffer>,
    ) -> WakeReason {
        // We need to clear the tx buffer, send everything over
        for ch in tx_buffer.buf[..tx_buffer.index].chunks(Self::msg_chunk_size(gatt_conn)) {
            let value = BleRpcMessageBuffer::try_from_slice(ch).expect("Max MTU is exceeded.");

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

                    defmt::debug!(
                        "SENT NORMAL {=usize} {=[u8]}",
                        value.used_length,
                        value.msg[..value.used_length]
                    );
                })
                .await
            {
                WakeReason::Ack => {
                    defmt::debug!("Got ack for {:?}", value.msg[..value.used_length]);
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
        let active_writers = self.active_tx_count.load(Ordering::SeqCst);
        assert!(
            active_writers > 0,
            "Writer counter is messed up, this is bad"
        );
        active_writers == 1
    }
}

impl WireTx for BleWireTx {
    type Error = WireTxErrorKind;

    async fn send<T: serde::Serialize + ?Sized>(
        &self,
        hdr: postcard_rpc::header::VarHeader,
        msg: &T,
    ) -> Result<(), Self::Error> {
        // let's get to the connection first. The read lock on the `inner` ensures that gatt_conn
        // does not change (also the inner.ack_queue cannot be set from disconnected to connected)
        let guard = self.inner.read().await;
        let inner = &*guard;
        let ack_q = &inner.ack_queue;
        let tx_char = &inner.server.rpc_service.tx;

        let Some(ref gatt_conn) = inner.gatt_conn else {
            return Err(WireTxErrorKind::ConnectionClosed);
        };

        // NOTE: Checking gatt_conn.is_some() may not be enough in some cases. Since we keep only one instance of
        // gatt_conn and many tx handles, we may end up in a situation where tx handle corresponds to a previous
        // gatt_conn. The situation I am describing is rougly as follows: a connection is established, client starts
        // a topic publishing task (spawn request). But this task mostly sleeps waiting for an event to happen to
        // publish for the client. Imagine a topic that signals an alarm going off. It is mostly waiting.
        // While this task is in running state but waiting, the client disconnects and a new client connects.
        // If the topic task wakes up with the event and tries to publish using the tx handle, it will succeed but
        // the client is a different client.
        // Moral of the story is that when the client connects, it should not assume the topic publishing tasks are
        // off. The chatty ones are probably off because if they try to publish when there is no connection, they
        // terminate. But the long waiting ones may not be terminated.

        let send_intent_guard = self.announce_tx_intent(); // use airtime if nobody else created a send guard
        let mut buffer_guard = self.tx_buffer.lock().await;

        // Laod bearing shadowing: this is required to change the drop order
        // basically we always want to decrease the writer counter before releasing the buffer lock. This way
        // there's no window where another task grabs the buffer lock and at the time of decision to claim airtime
        // thinks there's another writer in the queue waiting for the buffer lock.
        let send_intent_guard = send_intent_guard;

        defmt::debug!("Got the buffer lock");
        // if others are waiting for an ack, we also wait
        // we don't give up the lock here so technically we should only sleep once
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

        // We have the buffer lock, we waited for all the acknowledgements arrive,
        // we're still connected. It's our turn.
        let mut need_flush = false;
        loop {
            let begin = buffer_guard.index;
            match self.serialize_into_buffer(&mut buffer_guard.buf[begin..], &hdr, msg) {
                None => {
                    if begin == 0 {
                        // not enough space, sorry this send will fail
                        return Err(WireTxErrorKind::Other);
                    } else {
                        match Self::flush_buffer(gatt_conn, &mut *buffer_guard, ack_q, tx_char)
                            .await // we must have an empty buffer here
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
            // give up the lock and let the next guy flush. We just wait for
            // acknowledgement here.
            match ack_q
                .wait_once_with(async move || {
                    // drop order matters, check the comment above
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

#[embassy_executor::task]
pub async fn rpc_dispatcher_task(
    mut d: BleDispatcher,
    tx: server::Sender<BleWireTx>,
    rx: Receiver<'static, CriticalSectionRawMutex, IncomingData, 16>,
) {
    let mut acc = Accumulator::<RX_BUF_SIZE>::new();

    loop {
        {
            defmt::debug!("Dispatcher waiting for write event");
            let w_event = rx.receive().await;
            if acc.feed(w_event.data()).is_err() {
                // only error is no space for the message
                // TODO: there is a case where we don't have enough space to put the entire
                // new write_event data into the accumulator but maybe we can put enough to
                // decode, process that frame, then put the remaning at the beginning. Handle
                // that case later. I am confident that ergot cobs accumulator deals with that
                // with minimal memcpy. The only problem with that is that I am okay to do one
                // more memcpy so tha I can free the write_event back to the packet pool before
                // the next await point.
                acc.reset();
            }
            drop(w_event); // return the underlying buffer back to the PacketPool

            loop {
                match acc.yield_frame() {
                    Ok(buf) => {
                        defmt::debug!("Acc yielded a frame");

                        let Some((hdr, body)) = VarHeader::take_from_slice(buf) else {
                            // TODO: send a nak on badly formed messages? We don't have
                            // much to say because we don't have a key or seq no or anything
                            defmt::warn!("Bad message of size {}. Discarding...", buf.len());
                            continue;
                        };
                        let fut = d.handle(&tx, &hdr, body);
                        if let Err(e) = fut.await {
                            let kind = e.as_kind();
                            match kind {
                                WireTxErrorKind::ConnectionClosed => break,
                                WireTxErrorKind::Other => continue,
                                WireTxErrorKind::Timeout => continue,
                                _ => continue,
                            }
                        }
                    }
                    Err(AccumulatorYieldError::DecodingError) => continue, // try to decode the next package
                    Err(AccumulatorYieldError::NotACobsFrame) => break,    // wait for more packets
                }
            }
        }
    }
}

/// COPY AND PASTE FROM POSTCARD_RPC embedded_io_async impls
///
fn flava_flav(buf: &'_ mut [u8]) -> Result<Cobs<Slice<'_>>, WireTxErrorKind> {
    Cobs::try_new(Slice::new(buf)).map_err(|_| WireTxErrorKind::Other)
}

fn header_to_flavor(hdr: &VarHeader, flava: &mut Cobs<Slice<'_>>) -> Result<(), WireTxErrorKind> {
    // Serialize the header to a side buffer, since it doesn't use Serde
    let mut hdr_buf = [0u8; 1 + 4 + 8];
    let (used, _unused) = hdr
        .write_to_slice(&mut hdr_buf)
        .ok_or(WireTxErrorKind::Other)?;

    // Put the header into the buffer, which will cobs encode it
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
