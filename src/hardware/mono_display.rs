use esp_hal::{
    gpio::{Level, Output},
    i2c::master::{Config as I2cConfig, I2c},
    ledc::{
        Ledc, LowSpeed,
        timer::{self, TimerIFace},
    },
    peripherals::{GPIO6, GPIO7, I2C0, LEDC, RMT},
    rmt::{PulseCode, Rmt},
    time::Rate,
};
use esp_hal_smartled::{SmartLedsAdapter, buffer_size};
use ssd1306::{I2CDisplayInterface, Ssd1306Async, prelude::*};
use static_cell::StaticCell;

const I2C_FREQUENCY_KHZ: u32 = 400;

pub type MonoDisplay = Ssd1306Async<
    I2CInterface<I2c<'static, esp_hal::Async>>,
    DisplaySize128x64,
    ssd1306::mode::BufferedGraphicsModeAsync<DisplaySize128x64>,
>;

// For whatever reason, the compiler requires static here. I am
// assuming something in the code below contains something that is
// 'static.
pub(super) fn init(
    i2c0: I2C0<'static>,
    gpio6: GPIO6<'static>,
    gpio7: GPIO7<'static>,
) -> MonoDisplay {
    let i2c_bus: I2c<'_, esp_hal::Async> = I2c::new(
        i2c0,
        // I2cConfig is alias of esp_hal::i2c::master::I2c::Config
        I2cConfig::default()
            .with_frequency(Rate::from_khz(I2C_FREQUENCY_KHZ)),
    )
    .unwrap()
    .with_scl(gpio6)
    .with_sda(gpio7)
    .into_async();

    let interface = I2CDisplayInterface::new(i2c_bus);

    // initialize the display
    Ssd1306Async::new(
        interface,
        DisplaySize128x64,
        DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode()
}
