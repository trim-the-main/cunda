#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::system::Stack;
use esp_hal::timer::timg::TimerGroup;
use esp_rtos::embassy::Executor;
use static_cell::StaticCell;

use maitake_sync::Mutex;

extern crate alloc;

use nokta::ble::ble_init;

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
                spawner.must_spawn(second_cpu_main());
            });
        },
    );

    ble_init(spawner, peripherals.BT, logger).await;
    defmt::info!("BLE tasks are spawned, main thread is sleep looping.");

    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}
