use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex,
};
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
    peripherals::{
        FLASH, GPIO9, GPIO11, GPIO14, GPIO15, GPIO41, Peripherals,
    },
    rmt::{PulseCode, Rmt},
    time::Rate,
    timer::timg::TimerGroup,
};

type LEDType =
    Mutex<CriticalSectionRawMutex, Option<Output<'static>>>;
static RED_LED: LEDType = Mutex::new(None);
static GREEN_LED: LEDType = Mutex::new(None);
static BLUE_LED: LEDType = Mutex::new(None);
static YELLOW_LED: LEDType = Mutex::new(None);
static WHITE_LED: LEDType = Mutex::new(None);

static LED_ARRAY: [&'static LEDType; 5] =
    [&RED_LED, &GREEN_LED, &BLUE_LED, &YELLOW_LED, &WHITE_LED];

pub(super) async fn init(
    // red
    gpio41: GPIO41<'static>,
    // green
    gpio15: GPIO15<'static>,
    // blue
    gpio14: GPIO14<'static>,
    // yellow
    gpio9: GPIO9<'static>,
    // white
    gpio11: GPIO11<'static>,
) {
    let output_config_default = OutputConfig::default();

    let red_led =
        Output::new(gpio41, Level::Low, output_config_default);

    let green_led =
        Output::new(gpio15, Level::Low, output_config_default);
    let blue_led =
        Output::new(gpio14, Level::Low, output_config_default);
    let yellow_led =
        Output::new(gpio9, Level::Low, output_config_default);
    let white_led =
        Output::new(gpio11, Level::Low, output_config_default);

    {
        *(RED_LED.lock().await) = Some(red_led);
        *(GREEN_LED.lock().await) = Some(green_led);
        *(BLUE_LED.lock().await) = Some(blue_led);
        *(YELLOW_LED.lock().await) = Some(yellow_led);
        *(WHITE_LED.lock().await) = Some(white_led);
    }
}
