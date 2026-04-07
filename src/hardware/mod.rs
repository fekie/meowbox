#[allow(unused_imports)]
use defmt::{error, info, warn};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex,
};
use embassy_time::{Duration, Timer};
use esp_hal::{
    clock::CpuClock,
    gpio::{
        Input, InputConfig, Io, Level, Output, OutputConfig,
        OutputSignal, Pull, interconnect::PeripheralOutput,
    },
    i2c::master::{Config as I2cConfig, I2c},
    i2s::master::{Config, DataFormat, I2s},
    ledc::{
        Ledc, LowSpeed,
        timer::{self, TimerIFace},
    },
    peripherals::{FLASH, Peripherals},
    rmt::{PulseCode, Rmt},
    time::Rate,
    timer::timg::TimerGroup,
};
use esp_hal_smartled::{SmartLedsAdapter, buffer_size};
#[allow(unused_imports)]
use esp_println::println;
use smart_leds::{RGB8, SmartLedsWrite};
use ssd1306::{I2CDisplayInterface, Ssd1306Async, prelude::*};
use static_cell::StaticCell;

pub mod leds;
pub mod mono_display;
pub mod neopixel;
pub mod speaker;

use mono_display::MonoDisplay;

pub type ButtonType =
    Mutex<CriticalSectionRawMutex, Option<Input<'static>>>;
pub static RIGHT_BUTTON: ButtonType = Mutex::new(None);
pub static LEFT_BUTTON: ButtonType = Mutex::new(None);

pub type ButtonLEDType =
    Mutex<CriticalSectionRawMutex, Option<Output<'static>>>;
pub static RIGHT_BUTTON_LED: ButtonLEDType = Mutex::new(None);
pub static LEFT_BUTTON_LED: ButtonLEDType = Mutex::new(None);

pub type BuzzerType =
    Mutex<CriticalSectionRawMutex, Option<Output<'static>>>;
pub static BUZZER: BuzzerType = Mutex::new(None);

pub type PBuzzerType =
    Mutex<CriticalSectionRawMutex, Option<Output<'static>>>;
pub static PBUZZER_TOP_LEFT: PBuzzerType = Mutex::new(None);
pub static PBUZZER_TOP_RIGHT: PBuzzerType = Mutex::new(None);
pub static PBUZZER_BOTTOM_LEFT: PBuzzerType = Mutex::new(None);
pub static PBUZZER_BOTTOM_RIGHT: PBuzzerType = Mutex::new(None);

pub type RotarySwitchType =
    Mutex<CriticalSectionRawMutex, Option<Input<'static>>>;
pub static ROTARY_SWITCH_LEFT: RotarySwitchType = Mutex::new(None);
pub static ROTARY_SWITCH_RIGHT: RotarySwitchType = Mutex::new(None);

pub type LEDType =
    Mutex<CriticalSectionRawMutex, Option<Output<'static>>>;
pub static RED_LED: LEDType = Mutex::new(None);
pub static GREEN_LED: LEDType = Mutex::new(None);
pub static BLUE_LED: LEDType = Mutex::new(None);
pub static YELLOW_LED: LEDType = Mutex::new(None);
pub static WHITE_LED: LEDType = Mutex::new(None);

pub static LED_ARRAY: [&'static LEDType; 5] =
    [&RED_LED, &GREEN_LED, &BLUE_LED, &YELLOW_LED, &WHITE_LED];

pub type MonoDisplayType =
    Mutex<CriticalSectionRawMutex, Option<MonoDisplay>>;
// pub static DISPLAY: MonoDisplayType = Mutex::new(None);
static BAR: static_cell::StaticCell<MonoDisplayType> =
    static_cell::StaticCell::new();

pub struct NonMutexPeripherals {
    pub display: mono_display::DisplayType,
    pub left_rotary_a: Input<'static>,
    pub left_rotary_b: Input<'static>,
    pub right_rotary_a: Input<'static>,
    pub right_rotary_b: Input<'static>,
    pub flash: FLASH<'static>,
    // it has a buffer size of one because there is only one neopixel
    pub neopixel: SmartLedsAdapter<'static, 25>,
    pub i2s_speaker: I2s<'static, esp_hal::Async>,
}

/// Initializes peripherals and assigns them to their respective
/// mutexes.
pub async fn init_peripherals(
    peripherals: Peripherals,
) -> NonMutexPeripherals {
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    Timer::after(Duration::from_millis(500)).await;

    let pull_up_config = InputConfig::default().with_pull(Pull::Up);
    let output_config_default = OutputConfig::default();

    let left_button = Input::new(peripherals.GPIO5, pull_up_config);

    let right_button = Input::new(peripherals.GPIO8, pull_up_config);

    let left_button_light = Output::new(
        peripherals.GPIO4,
        Level::Low,
        output_config_default,
    );
    let right_button_light = Output::new(
        peripherals.GPIO12,
        Level::Low,
        output_config_default,
    );

    let buzzer = Output::new(
        peripherals.GPIO16,
        Level::Low,
        output_config_default,
    );

    let rotary_switch_left =
        Input::new(peripherals.GPIO18, pull_up_config);

    let rotary_switch_right =
        Input::new(peripherals.GPIO17, pull_up_config);

    let red_led = Output::new(
        peripherals.GPIO41,
        Level::Low,
        output_config_default,
    );
    let green_led = Output::new(
        peripherals.GPIO15,
        Level::Low,
        output_config_default,
    );
    let blue_led = Output::new(
        peripherals.GPIO14,
        Level::Low,
        output_config_default,
    );
    let yellow_led = Output::new(
        peripherals.GPIO9,
        Level::Low,
        output_config_default,
    );
    let white_led = Output::new(
        peripherals.GPIO11,
        Level::Low,
        output_config_default,
    );

    let left_rotary_a = Input::new(peripherals.GPIO2, pull_up_config);
    let left_rotary_b =
        Input::new(peripherals.GPIO42, pull_up_config);

    let right_rotary_a =
        Input::new(peripherals.GPIO10, pull_up_config);
    let right_rotary_b =
        Input::new(peripherals.GPIO13, pull_up_config);

    {
        *(RIGHT_BUTTON.lock().await) = Some(right_button);
        *(LEFT_BUTTON.lock().await) = Some(left_button);
        *(RIGHT_BUTTON_LED.lock().await) = Some(right_button_light);
        *(LEFT_BUTTON_LED.lock().await) = Some(left_button_light);
        *(BUZZER.lock().await) = Some(buzzer);
        *(ROTARY_SWITCH_LEFT.lock().await) = Some(rotary_switch_left);
        *(ROTARY_SWITCH_RIGHT.lock().await) =
            Some(rotary_switch_right);

        *(RED_LED.lock().await) = Some(red_led);
        *(GREEN_LED.lock().await) = Some(green_led);
        *(BLUE_LED.lock().await) = Some(blue_led);
        *(YELLOW_LED.lock().await) = Some(yellow_led);
        *(WHITE_LED.lock().await) = Some(white_led);
    }

    // Uncomment this after led handle is done
    // leds::init(
    //     peripherals.GPIO41,
    //     peripherals.GPIO15,
    //     peripherals.GPIO14,
    //     peripherals.GPIO9,
    //     peripherals.GPIO11,
    // ).await;

    let display = mono_display::init(
        peripherals.I2C0,
        peripherals.GPIO6,
        peripherals.GPIO7,
    );

    let flash = peripherals.FLASH;

    let neopixel = neopixel::init(
        peripherals.LEDC,
        peripherals.RMT,
        peripherals.GPIO48,
        output_config_default,
    );

    let i2s_speaker = speaker::init(
        peripherals.I2S0,
        peripherals.DMA_CH0,
        peripherals.GPIO37,
        peripherals.GPIO38,
        peripherals.GPIO39,
        peripherals.GPIO40,
    );

    Timer::after(Duration::from_millis(1000)).await;

    NonMutexPeripherals {
        display,
        left_rotary_a,
        left_rotary_b,
        right_rotary_a,
        right_rotary_b,
        flash, //simple_speaker,
        neopixel,
        i2s_speaker,
    }
}
