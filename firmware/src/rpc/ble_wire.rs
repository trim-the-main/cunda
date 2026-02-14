use core::ops::DerefMut;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_time::Duration;
use maitake_sync::RwLock;
use postcard::ser_flavors::Cobs;
use postcard::{
    Serializer,
    ser_flavors::{Flavor, Slice},
};
use postcard_rpc::header::VarHeader;
use postcard_rpc::server::{
    self, AsWireTxErrorKind, Dispatch, WireRx, WireRxErrorKind, WireTx, WireTxErrorKind,
};
use serde::Serialize;
use static_cell::StaticCell;
use trouble_host::{gatt::GattConnection, prelude::*};

use crate::ble::WriteEventWrapper;
use crate::rpc::accumulator::{AccumulatorYieldError, RX_BUF_SIZE};
use crate::rpc::dispatcher::BleDispatcher;
use crate::{
    ble::{BleRpcMessageBuffer, GattServerRpc, PeriphRole},
    rpc::accumulator::Accumulator,
};

struct BleWireInner {
    server: GattServerRpc<'static>, // Holds the rpc_service and the characteristics
    gatt_conn: Option<GattConn>,    // Connection object
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
        }
    }

    fn store_connection(
        &'static mut self,
        conn: Connection<'static, DefaultPacketPool>,
    ) -> Result<(), Error> {
        match conn.with_attribute_server(&self.server) {
            Ok(gatt_conn) => {
                self.gatt_conn = Some(gatt_conn);
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
}

impl BleWireStorage {
    pub const fn new() -> Self {
        Self {
            inner: StaticCell::new(),
        }
    }

    pub fn init(
        &'static self,
        periph_role: PeriphRole,
        ch: &'static Channel<CriticalSectionRawMutex, WriteEventWrapper, 16>,
    ) -> (BleWireRx, BleWireTx) {
        let wire = &*self.inner.init(RwLock::new(BleWireInner::new()));

        let rx = BleWireRx::new(wire, periph_role, ch.sender());
        let tx = BleWireTx::new(wire);
        (rx, tx)
    }
}

// Rx and Tx shares a reference through RwLock to the GattServer and GattConnection
// Tracking the connection state is the job of Rx handle. When we receive the
// GattEvent that says we're disconnected, we try to acquire the Write lock to
// the shared struct and delete the connection object. Until we delete it there may
// be other Tx requests but the Ble library should also fail them with connection closed.

pub struct BleWireRx {
    inner: &'static RwLock<BleWireInner>,

    // Rx side owns this object and it is used for advertising (in wait_connection)
    periph_role: PeriphRole,

    // During the connection loop, we operate under the Read lock on the inner object
    // (we share it with BleWireTx handles). There is one Rx handle but potentially
    // multiple Tx handles. When the connection drops, Rx handle will receive a
    // disconnection event
    disconnected: bool,
    dispatcher_channel: Sender<'static, CriticalSectionRawMutex, WriteEventWrapper, 16>,
}

impl BleWireRx {
    const ADV_TIMEOUT: Duration = Duration::from_secs(2);
    fn new(
        inner: &'static RwLock<BleWireInner>,
        periph_role: PeriphRole,
        dispatcher_channel: Sender<'static, CriticalSectionRawMutex, WriteEventWrapper, 16>,
    ) -> Self {
        Self {
            inner,
            periph_role,
            disconnected: true,
            dispatcher_channel,
        }
    }

    pub async fn receive_from_gatt_conn(
        &mut self,
        rx_handle: u16,
        gatt_conn: &GattConnection<'static, 'static, DefaultPacketPool>,
    ) -> Result<(), WireRxErrorKind> {
        loop {
            match gatt_conn.next().await {
                GattConnectionEvent::Disconnected { reason } => {
                    defmt::info!("Got disconnected event {}", reason);
                    self.disconnected = true;
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
                        // read_event
                        //     .reject(AttErrorCode::READ_NOT_PERMITTED)
                        //     .expect("Rejecting gatt event shouldn't fail.");
                    }
                    GattEvent::Write(write_event) => {
                        if write_event.handle() == rx_handle {
                            defmt::info!("Received data: {:?}", write_event.data());
                            self.dispatcher_channel
                                .send(WriteEventWrapper::new(write_event))
                                .await;
                            return Ok(());
                        } else {
                            defmt::warn!(
                                "Unexpected GattEvent::Write received at {} instead of {}",
                                write_event.handle(),
                                rx_handle,
                            );
                            // write_event
                            //     .reject(AttErrorCode::WRITE_NOT_PERMITTED)
                            //     .expect("Rejecting gatt write event failed.");
                        }
                    }
                    GattEvent::Other(_other_event) => {
                        defmt::warn!("Received GattEvent::Other unexpectedly");
                        // other_event
                        //     .reject(AttErrorCode::UNLIKELY_ERROR)
                        //     .expect("Rejecting gatt event shouldn't fail.");
                    }
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
                    defmt::debug!(
                        "Keep on advertising, `advertiser.accept()` returned Error: {:?}",
                        err
                    );
                }
            }
        }
    }
    pub async fn work_on_connection(&mut self) -> Result<(), WireRxErrorKind> {
        let inner = self.inner.read().await;
        let BleWireInner {
            gatt_conn: Some(ref gatt_conn),
            ref server,
        } = *inner
        else {
            return Err(WireRxErrorKind::Other);
        };

        while self
            .receive_from_gatt_conn(server.rpc_service.rx.handle, gatt_conn)
            .await
            .is_ok()
        {}
        return Err(WireRxErrorKind::ConnectionClosed);
    }

    pub async fn wait_connection(&mut self) {
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

#[derive(Clone)]
pub struct BleWireTx {
    inner: &'static RwLock<BleWireInner>,
}
impl BleWireTx {
    fn new(inner: &'static RwLock<BleWireInner>) -> Self {
        Self { inner }
    }

    pub(crate) async fn get_current_mtu(&self) -> Option<u16> {
        let guard = self.inner.read().await;

        let Some(ref conn) = guard.gatt_conn else {
            return None;
        };
        Some(conn.raw().att_mtu())
    }
}

// TODO: Send data that doesn't fit into one BleRpcMessageBuffer
// TODO: Accumulate multiple postcard_rpc frames into one BleRpcMessageBuffer
impl WireTx for BleWireTx {
    type Error = WireTxErrorKind;

    async fn send<T: serde::Serialize + ?Sized>(
        &self,
        hdr: postcard_rpc::header::VarHeader,
        msg: &T,
    ) -> Result<(), Self::Error> {
        // let's get to the buffer first
        let guard = self.inner.read().await;
        let inner = &*guard;

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

        let mut value = BleRpcMessageBuffer::new();
        // Create a cobs-encoding flavor using our temp buffer
        value.used_length = {
            let mut flavor = flava_flav(&mut value.msg)?;

            // Put the header into the buffer, which will cobs encode it
            header_to_flavor(&hdr, &mut flavor)?;

            // Now do normal serialization (and cobs encoding)
            let used = body_to_flavor(msg, flavor)?;
            used.len()
        };
        defmt::debug!("Sending data {:?}", value.msg[..value.used_length]);

        inner
            .server
            .rpc_service
            .tx
            .notify(gatt_conn, &value)
            .await
            .map_err(|_| {
                defmt::debug!("Error notifying the `TX` characteristic");
                WireTxErrorKind::ConnectionClosed
            })?;

        defmt::debug!(
            "SENT NORMAL {=usize} {=[u8]}",
            value.used_length,
            value.msg[..value.used_length]
        );
        // We did it! yaaaay!
        Ok(())
    }

    async fn send_raw(&self, buf: &[u8]) -> Result<(), Self::Error> {
        let guard = self.inner.read().await;
        let inner = &*guard;

        let Some(ref gatt_conn) = inner.gatt_conn else {
            return Err(WireTxErrorKind::ConnectionClosed);
        };
        let value = BleRpcMessageBuffer::from_slice(buf).ok_or(WireTxErrorKind::Other)?;
        inner
            .server
            .rpc_service
            .tx
            .notify(gatt_conn, &value)
            .await
            .map_err(|_| WireTxErrorKind::ConnectionClosed)?;

        // defmt::println!("SENT NORMAL {=usize} {=[u8]}", used.len(), used);
        // We did it! yaaaay!
        Ok(())
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
    rx: Receiver<'static, CriticalSectionRawMutex, WriteEventWrapper, 16>,
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
