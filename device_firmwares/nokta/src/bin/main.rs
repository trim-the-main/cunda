#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Instant, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig};
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::rmt::Rmt;
use esp_hal::system::Stack;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::uart::Uart;
use esp_rtos::embassy::Executor;
use nokta::temperature_service::TemperatureService;
use static_cell::StaticCell;

extern crate alloc;

use maitake_sync::Mutex;

use esp_hal_rmt_onewire::OneWire;

use nokta::ble::ble_init;
use nokta::context::RuntimeContext;
use nokta::i2c::defs::{I2CType, SharedI2C};
use nokta::nmea_gps_parser::{NmeaGpsParserService, nmea_consumer_service_run_to_completion};
use nokta::ublox_nmea_service::{UbloxPublisher, UbloxPublisherError};

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
    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

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
    esp_rtos::start_second_core(
        peripherals.CPU_CTRL,
        sw_int.software_interrupt1,
        app_core_stack,
        move || {
            static EXECUTOR: StaticCell<Executor> = StaticCell::new();
            let executor = EXECUTOR.init(Executor::new());
            executor.run(|spawner| {
                static I2C_BUS: StaticCell<Mutex<I2CType>> = StaticCell::new();
                let i2c_bus = SharedI2C::new(I2C_BUS.init(Mutex::new(i2c.into_async())));
                let tx_ready_pin = Input::new(peripherals.GPIO27, InputConfig::default());
                spawner.spawn(
                    ublox_gps_worker(i2c_bus.clone(), tx_ready_pin, rt_ctxt)
                        .expect("Failed to spawn ublox_gps_worker"),
                );

                if let Ok(uart1) = Uart::new(
                    peripherals.UART1,
                    esp_hal::uart::Config::default().with_baudrate(38000),
                ) {
                    let uart1 = uart1
                        .with_rx(peripherals.GPIO18)
                        .with_tx(peripherals.GPIO19)
                        .into_async();
                    spawner.spawn(
                        uart_writer_worker(uart1, rt_ctxt)
                            .expect("Failed to spawn nmea_gps_parser_worker"),
                    );
                } else {
                    defmt::error!("Failed to create UART");
                }
                let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80_u32))
                    .unwrap()
                    .into_async();
                spawner.spawn(
                    temperature_worker(
                        OneWire::new(rmt.channel0, rmt.channel2, peripherals.GPIO4).unwrap(),
                        rt_ctxt,
                    )
                    .expect("Failed to spawn temp worker"),
                );
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
async fn ublox_gps_worker(
    i2c: SharedI2C<'static, I2CType>,
    mut data_ready_pin: Input<'static>,
    rt_ctxt: &'static RuntimeContext,
) {
    const READ_DELAY: u64 = 1100;
    defmt::info!("ublox pub worker has started");

    let mut ublox_publisher = UbloxPublisher::new(i2c, rt_ctxt);
    ublox_publisher.configure_ubx().await;

    loop {
        let mut t_driver_work = Duration::from_micros(0);
        for _ in 0..4 {
            let t0 = Instant::now();
            match ublox_publisher.do_work().await {
                Ok(true) => t_driver_work += Instant::now() - t0,
                Ok(false) => break,
                Err(UbloxPublisherError::DeviceDidNotRespond) => {
                    defmt::warn!(
                        "Ublox device did not respond, assuming it is disconnected. Shutting down the worker"
                    );
                    return;
                }
                Err(_) => unreachable!("ublox do_work only returns DeviceDidNotRespond error"),
            };
        }
        defmt::trace!("ublox publisher worked for {:?}\n", t_driver_work);
        if t_driver_work.as_millis() < READ_DELAY {
            match select(
                data_ready_pin.wait_for_high(),
                Timer::after_millis(READ_DELAY - t_driver_work.as_millis()),
            )
            .await
            {
                Either::First(_) => {
                    defmt::trace!("Waking up from the txready signal")
                }
                Either::Second(_) => {
                    defmt::trace!("Waking up from timer")
                }
            }
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

#[embassy_executor::task]
async fn uart_writer_worker(
    mut uart: Uart<'static, esp_hal::Async>,
    rt_ctxt: &'static RuntimeContext,
) {
    let mut nmea_sub = rt_ctxt
        .nmea_bcast_channel
        .subscriber()
        .expect("Could not subscribe to nmea channel");
    loop {
        let msg = nmea_sub.next_message().await;
        match msg {
            embassy_sync::pubsub::WaitResult::Lagged(x) => {
                defmt::warn!("uart worker lagged {} messages", x)
            }
            embassy_sync::pubsub::WaitResult::Message(sentence_bytes) => {
                let mut remaining_bytes = sentence_bytes.len();
                while remaining_bytes > 0 {
                    match uart.write_async(&sentence_bytes).await {
                        Ok(written_bytes) => {
                            remaining_bytes -= written_bytes;
                        }
                        Err(err) => {
                            defmt::error!("Failed to write bytes to Uart interface: {}", err);
                        }
                    }
                }
                match uart.flush_async().await {
                    Ok(_) => {}
                    Err(err) => defmt::error!("Error flushing uart {}", err),
                };
            }
        }
    }
}

#[embassy_executor::task]
async fn temperature_worker(ow: OneWire<'static>, rt_ctxt: &'static RuntimeContext) {
    defmt::info!("Temperature worker starting");

    let mut srv = TemperatureService::new(ow, rt_ctxt);
    loop {
        srv.read_sensors_and_announce().await;
        Timer::after_secs(10).await;
    }
}
