use embassy_executor::{Spawner, SpawnerTraceExt};
use embassy_time::Duration;
use esp_hal::gpio::{Input, Output};
use esp_radio::ble::Config;
use esp_radio::ble::controller::BleConnector;

use maitake_sync::Mutex;
use postcard_rpc::server::{Dispatch, Sender};
use postcard_rpc_ble::{BleWireTx, DispatcherRunner, PrpcBleConn, PrpcBleStorage};
use static_cell::StaticCell;
use trouble_host::prelude::*;

use crate::rpc::dispatcher::{BleDispatcher, DispatchContext};

pub(crate) mod constants {
    pub(crate) const CONNECTIONS_MAX: usize = 1;
    pub(crate) const L2CAP_CHANNELS_MAX: usize = 2;
    pub(crate) const N_CMD_SLOTS: usize = 8;
}

use crate::ble::constants::*;

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

    // Initialize the RPC transport
    static PRPC: StaticCell<PrpcBleStorage<1>> = StaticCell::new();
    let prpc = PRPC.init(PrpcBleStorage::new(PeripheralConfig {
        name: "PostcardRPC",
        appearance: &appearance::power_device::GENERIC_POWER_DEVICE,
    }));
    let (conn, d_runner, tx) = prpc.init();

    spawner
        .spawn_named(
            "BleFrontendTask",
            ble_frontend_task(conn, peripheral, spawner, button, led, tx, d_runner),
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
    mut conn: PrpcBleConn<'static, 'static, 'static, 1>,
    mut periph: PeriphRole,
    spawner: Spawner,
    button: &'static Mutex<Input<'static>>,
    led: Output<'static>,
    tx: BleWireTx<'static, 'static, 'static>,
    d_runner: DispatcherRunner<'static, 'static, 'static, 1>,
) {
    // Create and spawn the dispatcher task
    let context = DispatchContext::new(button, led, tx.clone());
    let dispatcher = BleDispatcher::new(context, spawner.into());
    let vkk = dispatcher.min_key_len();

    spawner.must_spawn(rpc_dispatcher_task(
        d_runner,
        dispatcher,
        Sender::new(tx, vkk),
    ));

    // Connection lifecycle loop
    loop {
        let sys_config = crate::storage::SYSTEM_CONFIG.get_or_default().await;
        match advertise_once(&mut periph, &sys_config.ble_adv_name).await {
            Ok(connection) => {
                if conn.on_connected(connection).await.is_ok() {
                    let _ = conn.run().await;
                }
                conn.on_disconnected().await;
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

const ADV_TIMEOUT: Duration = Duration::from_secs(2);

async fn advertise_once(
    periph: &mut PeriphRole,
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
    params.timeout = Some(ADV_TIMEOUT);

    defmt::info!(
        "Advertising and waiting for a connection adv_size {}",
        adv_size
    );
    let advertiser = periph
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

// OTA transfers can be up to 4096 bytes + postcard overhead
const RX_BUF_SIZE: usize = 4096 + 256;

#[embassy_executor::task]
async fn rpc_dispatcher_task(
    runner: DispatcherRunner<'static, 'static, 'static, 1>,
    mut dispatcher: BleDispatcher,
    tx: Sender<BleWireTx<'static, 'static, 'static>>,
) {
    runner.run::<_, RX_BUF_SIZE>(&mut dispatcher, tx).await;
}
