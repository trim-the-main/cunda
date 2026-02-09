use embassy_executor::{Spawner, SpawnerTraceExt};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use esp_hal::gpio::{Input, Output};
use esp_radio::ble::Config;
use esp_radio::ble::controller::BleConnector;

use maitake_sync::Mutex;
use postcard_rpc::server::Server;
use static_cell::{ConstStaticCell, StaticCell};
use trouble_host::prelude::*;

use postcard_rpc::server::Dispatch;

pub(crate) mod constants {
    pub(crate) const CONNECTIONS_MAX: usize = 1;
    pub(crate) const L2CAP_CHANNELS_MAX: usize = 2;
    pub(crate) const N_CMD_SLOTS: usize = 8;
    pub(crate) const MSG_MTU: usize = 255;
}

use crate::ble::constants::*;
use crate::rpc::accumulator::RX_BUF_SIZE;
use crate::rpc::ble_wire::BleWireStorage;
use crate::rpc::dispatcher::{BleDispatcher, DispatchContext};

pub struct BleRpcMessageBuffer {
    pub msg: [u8; Self::MSG_SIZE],
    pub used_length: usize,
}

impl BleRpcMessageBuffer {
    pub(crate) const MSG_SIZE: usize = MSG_MTU;

    pub(crate) fn from_slice(src: &[u8]) -> Option<Self> {
        if src.len() > Self::MSG_SIZE - cobs::max_encoding_overhead(250) {
            return None;
        }
        let mut retval: BleRpcMessageBuffer = Default::default();
        let mut enc = cobs::CobsEncoder::new(&mut retval.msg);
        if enc.push(src).is_err() {
            return None;
        }
        retval.used_length = enc.finalize() + 1;
        Some(retval)
    }

    pub(crate) fn new() -> Self {
        Self {
            msg: [0u8; Self::MSG_SIZE],
            used_length: 0,
        }
    }
}

impl Default for BleRpcMessageBuffer {
    fn default() -> Self {
        Self {
            msg: [0u8; Self::MSG_SIZE],
            used_length: 0,
        }
    }
}

impl AsGatt for BleRpcMessageBuffer {
    fn as_gatt(&self) -> &[u8] {
        &self.msg[0..self.used_length]
    }

    const MIN_SIZE: usize = 0;

    const MAX_SIZE: usize = Self::MSG_SIZE;
}

impl FromGatt for BleRpcMessageBuffer {
    fn from_gatt(data: &[u8]) -> Result<Self, trouble_host::types::gatt_traits::FromGattError> {
        let mut msg = [0u8; Self::MSG_SIZE];
        msg[..data.len()].copy_from_slice(data);
        Ok(Self {
            msg,
            used_length: data.len(),
        })
    }
}

#[gatt_service(uuid = "408813DF-5DD4-1F87-EC11-CDB001100000")]
pub struct RpcService {
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, name = "rx", read, value = "rx buffer")]
    #[characteristic(uuid = "408813df-5dd4-1f87-ec11-cdb001100001", write)]
    pub rx: BleRpcMessageBuffer,

    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, name = "tx", read, value = "tx buffer")]
    #[characteristic(uuid = "408813df-5dd4-1f87-ec11-cdb001100002", notify)]
    pub tx: BleRpcMessageBuffer,
}

#[gatt_server(mutex_type = CriticalSectionRawMutex)]
pub struct GattServerRpc {
    pub rpc_service: RpcService,
}

pub(crate) type PeriphRole =
    Peripheral<'static, ExternalController<BleConnector<'static>, N_CMD_SLOTS>, DefaultPacketPool>;

pub async fn ble_init(
    spawner: Spawner,
    device: esp_hal::peripherals::BT<'static>,
    button: &'static Mutex<Input<'static>>,
    led: Output<'static>,
) {
    static RADIO: StaticCell<esp_radio::Controller<'static>> = StaticCell::new();
    let radio_init =
        RADIO.init(esp_radio::init().expect("Failed to initialize Wi-Fi/BLE controller"));
    let config = Config::default();
    let transport = BleConnector::new(radio_init, device, config).unwrap();
    let ble_controller: ExternalController<BleConnector<'_>, N_CMD_SLOTS> =
        ExternalController::<_, N_CMD_SLOTS>::new(transport);

    let resources = {
        static BLE_HOST_RESOURCES: StaticCell<
            HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX>,
        > = StaticCell::new();
        BLE_HOST_RESOURCES.init(HostResources::new())
    };

    let stack = {
        static BLE_STACK: StaticCell<
            Stack<'_, ExternalController<BleConnector<'_>, N_CMD_SLOTS>, DefaultPacketPool>,
        > = StaticCell::new();
        BLE_STACK.init(trouble_host::new(ble_controller, resources))
    };
    let Host {
        peripheral, runner, ..
    } = stack.build();

    spawner
        .spawn_named("BleBackendTask", ble_backend_task(runner))
        .expect("Failed to spawn BleBackendTask");
    spawner
        .spawn_named(
            "BleFrontendTask",
            ble_frontend_task(peripheral, spawner, button, led),
        )
        .expect("Failed to spawn BleFrontendTask");
}

/// Background task for communicating with lower level, today we panic if it fails but maybe
/// we can do a more graceful restart later. I dont think there's any recoverable error here.
#[embassy_executor::task]
pub async fn ble_backend_task(
    mut runner: Runner<
        'static,
        ExternalController<BleConnector<'static>, N_CMD_SLOTS>,
        DefaultPacketPool,
    >,
) {
    if let Err(_e) = runner.run().await {
        defmt::error!("Ble runner background task gave an error");
        panic!("Ble runner failure");
    }
}

#[embassy_executor::task]
pub async fn ble_frontend_task(
    ble_periph_role: Peripheral<
        'static,
        ExternalController<BleConnector<'static>, N_CMD_SLOTS>,
        DefaultPacketPool,
    >,
    spawner: Spawner,
    button: &'static Mutex<Input<'static>>,
    led: Output<'static>,
) {
    static BLE_WIRE_STORAGE: BleWireStorage = BleWireStorage::new();
    static PACKET_RX_BUF: ConstStaticCell<[u8; RX_BUF_SIZE]> =
        ConstStaticCell::new([0u8; RX_BUF_SIZE]);

    let context = DispatchContext::new(button, led);
    let dispatcher = BleDispatcher::new(context, spawner.into());
    let vkk = dispatcher.min_key_len();

    let (rx_impl, tx_impl) = BLE_WIRE_STORAGE.init(ble_periph_role);

    let mut server = Server::new(
        tx_impl,
        rx_impl,
        &mut PACKET_RX_BUF.take()[..],
        dispatcher,
        vkk,
    );

    loop {
        // If the host disconnects, we'll get an error here. Regardless of which end raised
        // the error we will reenter the server run loop which will poll the Rx end of the
        // wire. If it was the Rx end that raised the error, we will directly go into the
        // advertise loop. If it was the Tx end that raised the Disconnect error then we
        // will try to receive from Rx which will raise the Disconnect error again and we
        // will loop once more this time reaching to advertise.
        let err = server.run().await;
        match err {
            postcard_rpc::server::ServerError::TxFatal(err) => {
                defmt::error!("TxFatal: {}", err);
            }
            postcard_rpc::server::ServerError::RxFatal(err) => {
                defmt::error!("RxFatal: {:?}", err);
            }
        }
    }
}
