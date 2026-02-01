use defmt::info;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex,
};
use embassy_time::{Duration, Timer};
use esp_hal::{
    Config,
    clock::CpuClock,
    gpio::{
        DriveMode, Input, InputConfig, Level, Output, OutputConfig,
        Pull,
    },
    i2c::master::{Config as I2cConfig, I2c},
    ledc::{
        Ledc, LowSpeed,
        channel::{self, ChannelIFace},
        timer::{self, TimerIFace},
    },
    time::Rate,
    timer::timg::TimerGroup,
};
use ssd1306::{I2CDisplayInterface, Ssd1306Async, prelude::*};

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

//pub type RotaryLineType = Mutex<CriticalSectionRawMutex,
// Option<Input<'static>>>; pub static ROTARY_RIGHT_A: RotaryLineType
// = Mutex::new(None); pub static ROTARY_RIGHT_B: RotaryLineType =
// Mutex::new(None);

pub type LEDType =
    Mutex<CriticalSectionRawMutex, Option<Output<'static>>>;
pub static RED_LED: LEDType = Mutex::new(None);
pub static GREEN_LED: LEDType = Mutex::new(None);
pub static BLUE_LED: LEDType = Mutex::new(None);
pub static YELLOW_LED: LEDType = Mutex::new(None);
pub static WHITE_LED: LEDType = Mutex::new(None);

pub static LED_ARRAY: [&'static LEDType; 5] =
    [&RED_LED, &GREEN_LED, &BLUE_LED, &YELLOW_LED, &WHITE_LED];

use esp_hal::peripherals::Peripherals;

pub type Display = Ssd1306Async<
    I2CInterface<I2c<'static, esp_hal::Async>>,
    DisplaySize128x64,
    ssd1306::mode::BufferedGraphicsModeAsync<DisplaySize128x64>,
>;

pub struct NonMutexPeripherals {
    pub display: Display,
    pub left_rotary_a: Input<'static>,
    pub left_rotary_b: Input<'static>,
    pub right_rotary_a: Input<'static>,
    pub right_rotary_b: Input<'static>,
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

    let right_button = Input::new(peripherals.GPIO14, pull_up_config);

    let left_button = Input::new(peripherals.GPIO4, pull_up_config);

    let left_button_light = Output::new(
        peripherals.GPIO15,
        Level::Low,
        output_config_default,
    );
    let right_button_light = Output::new(
        peripherals.GPIO9,
        Level::Low,
        output_config_default,
    );

    let buzzer = Output::new(
        peripherals.GPIO12,
        Level::Low,
        output_config_default,
    );

    let rotary_switch_left =
        Input::new(peripherals.GPIO16, pull_up_config);

    let rotary_switch_right =
        Input::new(peripherals.GPIO10, pull_up_config);

    let red_led = Output::new(
        peripherals.GPIO6,
        Level::Low,
        output_config_default,
    );
    let green_led = Output::new(
        peripherals.GPIO2,
        Level::Low,
        output_config_default,
    );
    let blue_led = Output::new(
        peripherals.GPIO21,
        Level::Low,
        output_config_default,
    );
    let yellow_led = Output::new(
        peripherals.GPIO11,
        Level::Low,
        output_config_default,
    );
    let white_led = Output::new(
        peripherals.GPIO7,
        Level::Low,
        output_config_default,
    );

    let pbuzzer_top_left = Output::new(
        peripherals.GPIO1,
        Level::High,
        output_config_default,
    );
    // let pbuzzer_top_right = Output::new(
    //     peripherals.GPIO20,
    //     Level::Low,
    //     output_config_default,
    // );
    let pbuzzer_bottom_left = Output::new(
        peripherals.GPIO5,
        Level::Low,
        output_config_default,
    );
    let pbuzzer_bottom_right = Output::new(
        peripherals.GPIO13,
        Level::Low,
        output_config_default,
    );

    Timer::after(Duration::from_millis(500)).await;

    let left_rotary_a =
        Input::new(peripherals.GPIO42, pull_up_config);
    let left_rotary_b =
        Input::new(peripherals.GPIO41, pull_up_config);

    let right_rotary_a =
        Input::new(peripherals.GPIO3, pull_up_config);
    let right_rotary_b =
        Input::new(peripherals.GPIO46, pull_up_config);

    Timer::after(Duration::from_millis(500)).await;

    {
        *(RIGHT_BUTTON.lock().await) = Some(right_button);
        *(LEFT_BUTTON.lock().await) = Some(left_button);
        *(RIGHT_BUTTON_LED.lock().await) = Some(right_button_light);
        *(LEFT_BUTTON_LED.lock().await) = Some(left_button_light);
        *(BUZZER.lock().await) = Some(buzzer);
        *(ROTARY_SWITCH_LEFT.lock().await) = Some(rotary_switch_left);
        *(ROTARY_SWITCH_RIGHT.lock().await) =
            Some(rotary_switch_right);
        *(PBUZZER_TOP_LEFT.lock().await) = Some(pbuzzer_top_left);
        //*(PBUZZER_TOP_RIGHT.lock().await) = Some(pbuzzer_top_right);
        *(PBUZZER_BOTTOM_LEFT.lock().await) =
            Some(pbuzzer_bottom_left);
        // *(PBUZZER_BOTTOM_RIGHT.lock().await) =
        //     Some(pbuzzer_bottom_right);

        *(RED_LED.lock().await) = Some(red_led);
        *(GREEN_LED.lock().await) = Some(green_led);
        *(BLUE_LED.lock().await) = Some(blue_led);
        *(YELLOW_LED.lock().await) = Some(yellow_led);
        *(WHITE_LED.lock().await) = Some(white_led);
    }

    let i2c_bus: I2c<'_, esp_hal::Async> = I2c::new(
        peripherals.I2C0,
        // I2cConfig is alias of esp_hal::i2c::master::I2c::Config
        I2cConfig::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_scl(peripherals.GPIO17)
    .with_sda(peripherals.GPIO18)
    .into_async();

    let interface = I2CDisplayInterface::new(i2c_bus);
    // initialize the display
    let display = Ssd1306Async::new(
        interface,
        DisplaySize128x64,
        DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode();

    let ledc = Ledc::new(peripherals.LEDC);

    let mut timer = ledc.timer::<LowSpeed>(timer::Number::Timer0);
    timer
        .configure(timer::config::Config {
            duty: timer::config::Duty::Duty10Bit,
            clock_source: timer::LSClockSource::APBClk,
            frequency: Rate::from_hz(2000),
        })
        .unwrap();

    let buzzer_pin = peripherals.GPIO20;

    let mut channel =
        ledc.channel(channel::Number::Channel0, buzzer_pin);

    channel
        .configure(channel::config::Config {
            timer: &timer,
            duty_pct: 50,
            drive_mode: DriveMode::PushPull,
        })
        .unwrap();

    channel.set_duty(50).unwrap();

    Timer::after(Duration::from_millis(5000)).await;

    NonMutexPeripherals {
        display,
        left_rotary_a,
        left_rotary_b,
        right_rotary_a,
        right_rotary_b,
    }
}
