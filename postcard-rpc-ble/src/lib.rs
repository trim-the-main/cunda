#![no_std]

mod ack;
mod conn;
mod dispatcher;
mod gatt;
mod tx;

use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use maitake_sync::RwLock;
use trouble_host::{gatt::GattConnection, prelude::*};

use ack::AckQueue;
use gatt::GattServerRpc;
use tx::TxBatchState;

pub use conn::PrpcBleConn;
pub use dispatcher::DispatcherRunner;
pub use tx::BleWireTx;

pub struct PrpcBleStorage<'stack, 'server, const CH_SIZE: usize> {
    server: GattServerRpc<'server>,
    ack_queue: AckQueue,
    gatt_conn: RwLock<Option<GattConnection<'stack, 'server, DefaultPacketPool>>>,
    tx_batch: TxBatchState,
    rx_channel: Channel<NoopRawMutex, WriteEvent<'stack, 'server, DefaultPacketPool>, CH_SIZE>,
}

impl<'stack, 'server, const CH_SIZE: usize> PrpcBleStorage<'stack, 'server, CH_SIZE> {
    pub fn new(cfg: PeripheralConfig<'server>) -> Self {
        Self {
            server: GattServerRpc::new_with_config(GapConfig::Peripheral(cfg))
                .expect("Failed to create GATT server"),
            ack_queue: AckQueue::new(),
            gatt_conn: RwLock::new(None),
            tx_batch: TxBatchState::new(),
            rx_channel: Channel::new(),
        }
    }

    /// Initialize the BLE RPC transport. Consumes the `&'static mut` reference
    /// to guarantee single initialization at compile time.
    ///
    /// The caller should store `PrpcBle` in a `StaticCell` and call `.init()`
    /// on the returned `&'static mut PrpcBle`.
    pub fn init<'storage>(
        &'storage mut self,
    ) -> (
        PrpcBleConn<'storage, 'stack, 'server, CH_SIZE>,
        DispatcherRunner<'storage, 'stack, 'server, CH_SIZE>,
        BleWireTx<'storage, 'stack, 'server>,
    )
    where
        'storage: 'server,
    {
        let conn = PrpcBleConn::new(&*self);

        let d_runner = DispatcherRunner::new(self.rx_channel.receiver());
        let tx = BleWireTx::new(&*self);

        (conn, d_runner, tx)
    }
}
