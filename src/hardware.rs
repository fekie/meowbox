use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use esp_hal::gpio::InputConfig;
use esp_hal::gpio::Level;
use esp_hal::gpio::OutputConfig;
use esp_hal::gpio::Pull;
use esp_hal::gpio::{Input, Output};
use ssd1306::{I2CDisplayInterface, Ssd1306Async, prelude::*};

use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::Point,
    prelude::*,
    text::{Baseline, Text},
};

use esp_hal::timer::timg::TimerGroup;

use esp_hal::i2c::master::Config as I2cConfig;
use esp_hal::i2c::master::I2c;
use esp_hal::time::Rate;

pub type ButtonType = Mutex<CriticalSectionRawMutex, Option<Input<'static>>>;
pub static RIGHT_BUTTON: ButtonType = Mutex::new(None);
pub static LEFT_BUTTON: ButtonType = Mutex::new(None);

pub type ButtonLEDType = Mutex<CriticalSectionRawMutex, Option<Output<'static>>>;
pub static RIGHT_BUTTON_LED: ButtonLEDType = Mutex::new(None);
pub static LEFT_BUTTON_LED: ButtonLEDType = Mutex::new(None);

pub type BuzzerType = Mutex<CriticalSectionRawMutex, Option<Output<'static>>>;
pub static BUZZER: BuzzerType = Mutex::new(None);

pub type RotarySwitchType = Mutex<CriticalSectionRawMutex, Option<Input<'static>>>;
pub static ROTARY_SWITCH_LEFT: RotarySwitchType = Mutex::new(None);
pub static ROTARY_SWITCH_RIGHT: RotarySwitchType = Mutex::new(None);

use esp_hal::peripherals::Peripherals;

type Display = Ssd1306Async<
    I2CInterface<I2c<'static, esp_hal::Async>>,
    DisplaySize128x64,
    ssd1306::mode::BufferedGraphicsModeAsync<DisplaySize128x64>,
>;

/// Initializes peripherals and assigns them to their respective mutexes.
pub async fn init_peripherals(peripherals: Peripherals) -> Display {
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    let pull_up_config = InputConfig::default().with_pull(Pull::Up);
    let output_config_default = OutputConfig::default();

    let right_button = Input::new(peripherals.GPIO11, pull_up_config);

    let left_button = Input::new(peripherals.GPIO5, pull_up_config);

    let right_button_light = Output::new(peripherals.GPIO12, Level::High, output_config_default);
    let left_button_light = Output::new(peripherals.GPIO6, Level::High, output_config_default);

    let buzzer = Output::new(peripherals.GPIO7, Level::Low, output_config_default);

    let rotary_switch_left = Input::new(peripherals.GPIO1, pull_up_config);

    let rotary_switch_right = Input::new(peripherals.GPIO3, pull_up_config);

    {
        *(RIGHT_BUTTON.lock().await) = Some(right_button);
        *(LEFT_BUTTON.lock().await) = Some(left_button);
        *(RIGHT_BUTTON_LED.lock().await) = Some(right_button_light);
        *(LEFT_BUTTON_LED.lock().await) = Some(left_button_light);
        *(BUZZER.lock().await) = Some(buzzer);
        *(ROTARY_SWITCH_LEFT.lock().await) = Some(rotary_switch_left);
        *(ROTARY_SWITCH_RIGHT.lock().await) = Some(rotary_switch_right);
    }

    let i2c_bus: I2c<'_, esp_hal::Async> = I2c::new(
        peripherals.I2C0,
        // I2cConfig is alias of esp_hal::i2c::master::I2c::Config
        I2cConfig::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_scl(peripherals.GPIO9)
    .with_sda(peripherals.GPIO10)
    .into_async();

    let interface = I2CDisplayInterface::new(i2c_bus);
    // initialize the display
    let display = Ssd1306Async::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();

    display
}
