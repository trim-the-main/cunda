pub(crate) mod accumulator;
pub mod ble_wire;
pub mod dispatcher;

// use trouble_host::{
//     PacketPool,
//     att::AttErrorCode,
//     gatt::{GattConnection, GattConnectionEvent, GattEvent},
// };

// use crate::ble_pipe::BleRpcError;

// const ACCUMULATOR_SIZE: usize = 1024;

// pub struct RxWrapper<'c, P: PacketPool> {
//     gatt_conn: &'c GattConnection<'c, 'c, P>,
//     acc: accumulator::Accumulator<ACCUMULATOR_SIZE>,
//     rx_handle: u16,
// }

// impl<'c, P: PacketPool> RxWrapper<'c, P> {
//     pub(crate) fn new(gatt_conn: &'c GattConnection<'c, 'c, P>, rx_handle: u16) -> Self {
//         Self {
//             gatt_conn,
//             acc: accumulator::Accumulator::new(),
//             rx_handle,
//         }
//     }

//     async fn receive_from_connection(&mut self) -> Result<(), BleRpcError> {
//         loop {
//             match self.gatt_conn.next().await {
//                 GattConnectionEvent::Disconnected { reason } => {
//                     defmt::info!("Got disconnected event {}", reason);
//                     return Err(BleRpcError::Disconnected);
//                 }
//                 GattConnectionEvent::PhyUpdated { tx_phy, rx_phy } => {
//                     defmt::info!("GattConnectionEvent::PhyUpdates {} {}", tx_phy, rx_phy);
//                 }
//                 GattConnectionEvent::ConnectionParamsUpdated {
//                     conn_interval,
//                     peripheral_latency,
//                     supervision_timeout,
//                 } => {
//                     defmt::info!(
//                         "GattConnectionEvent::ConnectionParamsUpdated conn_interval: {} peripheral_latency: {} supervision_timeout: {}",
//                         conn_interval,
//                         peripheral_latency,
//                         supervision_timeout
//                     );
//                 }
//                 GattConnectionEvent::RequestConnectionParams {
//                     min_connection_interval,
//                     max_connection_interval,
//                     max_latency,
//                     supervision_timeout,
//                 } => {
//                     defmt::info!(
//                         "GattConnectionEvent::RequestConnectionParams min_connection_interval: {} max_connection_interval: {} max_latency: {} supervision_timeout: {}",
//                         min_connection_interval,
//                         max_connection_interval,
//                         max_latency,
//                         supervision_timeout
//                     );
//                 }
//                 GattConnectionEvent::DataLengthUpdated {
//                     max_tx_octets,
//                     max_tx_time,
//                     max_rx_octets,
//                     max_rx_time,
//                 } => {
//                     defmt::info!(
//                         "GattConnectionEvent::DataLengthUpdated max_tx_octets: {} max_tx_time: {} max_rx_octets: {} max_rx_time: {}",
//                         max_tx_octets,
//                         max_tx_time,
//                         max_rx_octets,
//                         max_rx_time,
//                     );
//                 }
//                 GattConnectionEvent::Gatt { event } => match event {
//                     GattEvent::Read(read_event) => {
//                         defmt::warn!(
//                             "Unexpected read event from the central device {}",
//                             read_event.handle()
//                         );
//                         read_event
//                             .reject(AttErrorCode::READ_NOT_PERMITTED)
//                             .expect("Rejecting gatt event shouldn't fail.");
//                     }
//                     GattEvent::Write(write_event) => {
//                         if write_event.handle() == self.rx_handle {
//                             defmt::trace!("Received data: {:?}", write_event.data());
//                             self.acc.feed(write_event.data()).unwrap();
//                             write_event
//                                 .accept()
//                                 .expect("Accepting gatt event shouldn't fail.");

//                             return Ok(());
//                         } else {
//                             defmt::error!(
//                                 "Unexpected GattEvent::Write received at {}",
//                                 write_event.handle()
//                             );
//                             write_event
//                                 .reject(AttErrorCode::WRITE_NOT_PERMITTED)
//                                 .expect("Rejecting gatt write event failed.");
//                         }
//                     }
//                     GattEvent::Other(other_event) => {
//                         defmt::warn!("Received GattEvent::Other unexpectedly");
//                         other_event
//                             .reject(AttErrorCode::UNLIKELY_ERROR)
//                             .expect("Rejecting gatt event shouldn't fail.");
//                     }
//                 },
//             }
//         }
//     } // pub mod ble_wire;
// }
