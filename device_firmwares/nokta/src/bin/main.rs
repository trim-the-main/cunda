#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::system::Stack;
use esp_hal::timer::timg::TimerGroup;
use esp_rtos::embassy::Executor;
use static_cell::StaticCell;

extern crate alloc;

use maitake_sync::Mutex;

use nokta::ble::ble_init;
use nokta::context::RuntimeContext;
use nokta::i2c::defs::{I2CType, SharedI2C};
use nokta::nmea_gps_parser::{NmeaGpsParserService, nmea_consumer_service_run_to_completion};
use nokta::ublox_nmea_service::{UbloxNMEAPublisher, UbloxNMEAPublisherError};

esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
async fn second_cpu_main() {
    return;
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.1.0

    // SAFETY:
    // unwrap is safe here because this is the first call to init
    let logger = defmt_brtt::init!().unwrap();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    defmt::info!("Embassy initialized on Core 0");

    static RUNTIME_GLOBAL_STATE: StaticCell<RuntimeContext> = StaticCell::new();
    let rt_ctxt = &*RUNTIME_GLOBAL_STATE.init(RuntimeContext::new());

    // Second Core should run: i2c stuff
    // I2C: Config
    let i2c_config = esp_hal::i2c::master::Config::default()
        .with_frequency(Rate::from_khz(100))
        .with_timeout(esp_hal::i2c::master::BusTimeout::Maximum);
    let i2c = esp_hal::i2c::master::I2c::new(peripherals.I2C0, i2c_config)
        .unwrap()
        .with_sda(peripherals.GPIO16)
        .with_scl(peripherals.GPIO17);

    static SHA: StaticCell<Mutex<esp_hal::sha::Sha<'static>>> = StaticCell::new();
    let sha = SHA.init(Mutex::new(esp_hal::sha::Sha::new(peripherals.SHA)));
    let flash = esp_storage::FlashStorage::new(peripherals.FLASH).multicore_auto_park();
    nokta::storage::init(flash, sha).await;

    static APP_CORE_STACK: StaticCell<Stack<16384>> = StaticCell::new();
    let app_core_stack = APP_CORE_STACK.init(Stack::new());
    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start_second_core(
        peripherals.CPU_CTRL,
        sw_int.software_interrupt0,
        sw_int.software_interrupt1,
        app_core_stack,
        move || {
            static EXECUTOR: StaticCell<Executor> = StaticCell::new();
            let executor = EXECUTOR.init(Executor::new());
            executor.run(|spawner| {
                static I2C_BUS: StaticCell<Mutex<I2CType>> = StaticCell::new();
                let i2c_bus = SharedI2C::new(I2C_BUS.init(Mutex::new(i2c.into_async())));
                spawner
                    .spawn(ublox_gps_worker(i2c_bus.clone(), rt_ctxt))
                    .expect("Failed to spawn ublox_gps_worker");
                spawner
                    .spawn(nmea_gps_parser_worker(rt_ctxt))
                    .expect("Failed to spawn nmea_gps_parser_worker");
            });
        },
    );

    ble_init(spawner, peripherals.BT, logger, rt_ctxt).await;
    defmt::info!("BLE tasks are spawned, main thread is sleep looping.");

    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}

#[embassy_executor::task]
async fn ublox_gps_worker(i2c: SharedI2C<'static, I2CType>, rt_ctxt: &'static RuntimeContext) {
    const READ_DELAY: u64 = 500;
    defmt::info!("ublox pub worker has started");

    let mut ublox_publisher = UbloxNMEAPublisher::new(i2c, rt_ctxt);

    loop {
        let mut t_driver_work = Duration::from_micros(0);
        for _ in 0..4 {
            let t0 = Instant::now();
            match ublox_publisher.do_work().await {
                Ok(true) => t_driver_work += Instant::now() - t0,
                Ok(false) => break,
                Err(UbloxNMEAPublisherError::DeviceDidNotRespond) => {
                    defmt::warn!(
                        "Ublox device did not respond, assuming it is disconnected. Shutting down the worker"
                    );
                    return;
                }
                Err(_) => unreachable!("ublox do_work only returns DeviceDidNotRespond error"),
            };
        }
        defmt::debug!("ublox publisher worked for {:?}\n", t_driver_work);
        if t_driver_work.as_millis() < READ_DELAY {
            Timer::after_millis(READ_DELAY - t_driver_work.as_millis()).await;
        } else {
            defmt::warn!("ublox worker is lagging behind, it cannot sleep");
        }
    }
}

#[embassy_executor::task]
async fn nmea_gps_parser_worker(rt_ctxt: &'static RuntimeContext) {
    defmt::info!("nmea parser started");

    let srv = NmeaGpsParserService::new(rt_ctxt);
    nmea_consumer_service_run_to_completion(srv).await;

    unreachable!("ublox subscriber should not return");
}
