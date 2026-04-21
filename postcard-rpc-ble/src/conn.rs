use core::cell::RefCell;

use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Sender;
use postcard_rpc::server::WireRxErrorKind;
use trouble_host::{gatt::GattConnection, prelude::*};

use crate::PrpcBleStorage;
use crate::gatt::GattServerRpc;
use maitake_sync::{RwLock, WaitCell};

/// Connection handler. Manages GATT events and connection lifecycle.
/// Handed to the frontend task.
pub struct PrpcBleConn<'storage, 'stack, 'server, const CH_SIZE: usize> {
    server: &'storage GattServerRpc<'server>,
    ack_queue: &'storage RefCell<WaitCell>,
    gatt_conn: &'storage RwLock<Option<GattConnection<'stack, 'server, DefaultPacketPool>>>,
    dispatcher_channel:
        Sender<'storage, NoopRawMutex, WriteEvent<'stack, 'server, DefaultPacketPool>, CH_SIZE>,
}

impl<'storage, 'stack, 'server, const CH_SIZE: usize>
    PrpcBleConn<'storage, 'stack, 'server, CH_SIZE>
where
    'storage: 'server,
{
    pub(crate) fn new<const TX_BUFFER_SIZE: usize>(
        storage: &'storage PrpcBleStorage<'stack, 'server, CH_SIZE, TX_BUFFER_SIZE>,
    ) -> Self {
        Self {
            server: &storage.server,
            ack_queue: &storage.ack_queue,
            gatt_conn: &storage.gatt_conn,
            dispatcher_channel: storage.rx_channel.sender(),
        }
    }
    /// Bind a new BLE connection. Call after advertising succeeds.
    pub async fn on_connected(
        &mut self,
        conn: Connection<'stack, DefaultPacketPool>,
    ) -> Result<(), Error> {
        let mut guard = self.gatt_conn.write().await;
        assert!(guard.is_none());

        match conn.with_attribute_server(&*self.server) {
            Ok(gatt_conn) => {
                *guard = Some(gatt_conn);
                *(self.ack_queue.borrow_mut()) = WaitCell::new();
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    /// Process GATT events until disconnection.
    /// Returns when the connection drops.
    pub async fn run(&mut self) -> Result<(), WireRxErrorKind> {
        let guard = self.gatt_conn.read().await;
        let Some(ref gatt_conn) = *guard else {
            return Err(WireRxErrorKind::Other);
        };

        while self.handle_gatt_event(gatt_conn).await.is_ok() {}

        Err(WireRxErrorKind::ConnectionClosed)
    }

    /// Clean up after disconnection.
    pub async fn on_disconnected(&mut self) {
        let mut guard = self.gatt_conn.write().await;
        drop(guard.take());
    }

    async fn handle_gatt_event(
        &mut self,
        gatt_conn: &GattConnection<'stack, 'server, DefaultPacketPool>,
    ) -> Result<(), WireRxErrorKind> {
        let rx_not_acked_handle = self.server.rpc_service.rx_not_acked.handle;
        let rx_acked_handle = self.server.rpc_service.rx_acked.handle;
        let dispatcher_channel = self.dispatcher_channel;
        loop {
            match gatt_conn.next().await {
                GattConnectionEvent::Disconnected { reason } => {
                    defmt::info!("Got disconnected event {}", reason);
                    self.ack_queue.borrow().close();
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
                        if write_event.handle() == rx_not_acked_handle {
                            defmt::debug!(
                                "Received data unacked: {:?} bytes",
                                write_event.data().len()
                            );
                            dispatcher_channel.send(write_event).await;
                            return Ok(());
                        }
                        if write_event.handle() == rx_acked_handle {
                            defmt::debug!(
                                "Received data acked: {:?} bytes, last byte: {}",
                                write_event.data().len(),
                                write_event.data()[write_event.data().len() - 2]
                            );
                            dispatcher_channel.send(write_event).await;
                            return Ok(());
                        } else {
                            defmt::warn!(
                                "Unexpected GattEvent::Write received at {} instead of {} or {}",
                                write_event.handle(),
                                rx_not_acked_handle,
                                rx_acked_handle,
                            );
                        }
                    }
                    GattEvent::Other(other_event) => match other_event.payload().incoming() {
                        trouble_host::att::AttClient::Confirmation(_) => {
                            defmt::debug!("Received ack");
                            if !self.ack_queue.borrow().wake() {
                                defmt::warn!("Ack woke up nobody.")
                            }
                        }
                        trouble_host::att::AttClient::Request(att_req) => {
                            defmt::warn!("Got unexpected GattEvent::Other {}", att_req);
                        }
                        trouble_host::att::AttClient::Command(att_cmd) => {
                            defmt::warn!("Got unexpected GattEvent::Other {}", att_cmd);
                        }
                    },
                },
            }
        }
    }
}
