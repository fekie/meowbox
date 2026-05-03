// pub fn foo() {
//     let peripherals = Peripherals::take();

//     let system = peripherals.SYSTEM.split();
//     let clocks = ClockControl::max(system.clock_control).freeze();

//     esp_alloc::heap_allocator!(size: 72 * 1024);

//     let dc = peripherals.GPIO9;
//     let mosi = peripherals.GPIO18;
//     let sclk = peripherals.GPIO19;
//     let miso = peripherals.GPIO20;
//     let cs = peripherals.GPIO21;
//     let rst = peripherals.GPIO22;

//     let mut tft =
//         TFT::new(peripherals.SPI2, sclk, miso, mosi, cs, rst, dc);

//     tft.clear(Rgb565::WHITE);
//     tft.println("Hello from ESP32-S3", 100, 40);
// }

use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel,
};

pub static BACKLIGHT_CH: Channel<
    CriticalSectionRawMutex,
    BacklightCommand,
    8,
> = Channel::new();

use esp_hal::gpio::{self, Level};

pub enum BacklightCommand {
    Toggle,
}

#[embassy_executor::task]
pub async fn backlight_listener(mut bl_pin: gpio::Output<'static>) {
    loop {
        let cmd = BACKLIGHT_CH.receive().await;

        match cmd {
            BacklightCommand::Toggle => {
                let current = bl_pin.output_level();

                match current {
                    Level::High => bl_pin.set_low(),
                    Level::Low => bl_pin.set_high(),
                }
            }
        }
    }
}
