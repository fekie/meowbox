#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use defmt::info;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::timer::timg::TimerGroup;
use esp_println as _;

use core::cell::RefCell;
use critical_section::Mutex;
use defmt::println;
use esp_hal::handler;
use esp_hal::interrupt::Priority;
use esp_hal::interrupt::software::SoftwareInterrupt;
use esp_hal::interrupt::software::SoftwareInterruptControl;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;

static SWINT0: Mutex<RefCell<Option<SoftwareInterrupt<0>>>> = Mutex::new(RefCell::new(None));

#[handler(priority = Priority::Priority1)]
fn swint0_handler() {
    info!("SW interrupt0");
    critical_section::with(|cs| {
        if let Some(swint) = SWINT0.borrow_ref(cs).as_ref() {
            swint.reset();
        }
    });
}

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.2.0

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    info!("Embassy initialized!");

    // TODO: Spawn some tasks
    let _ = spawner;

    let mut sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    critical_section::with(|cs| {
        sw_int
            .software_interrupt0
            .set_interrupt_handler(swint0_handler);
        SWINT0
            .borrow_ref_mut(cs)
            .replace(sw_int.software_interrupt0);
    });

    critical_section::with(|cs| {
        if let Some(swint) = SWINT0.borrow_ref(cs).as_ref() {
            swint.raise();
        }
    });

    loop {
        info!("Hello world!");
        Timer::after(Duration::from_secs(1)).await;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0/examples
}
