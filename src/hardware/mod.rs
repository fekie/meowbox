#[allow(unused_imports)]
use defmt::{error, info, warn};
use display_interface_spi::SPIInterface;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex,
};
use embassy_time::{Duration, Timer};
use embedded_hal::pwm::{ErrorType, SetDutyCycle};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
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
use ili9341::{DisplaySize240x320, Ili9341, Orientation};
use lcd_ili9341_spi::{Lcd, LcdOrientation, rgb_to_u16};
use smart_leds::{RGB8, SmartLedsWrite};
use ssd1306::{I2CDisplayInterface, Ssd1306Async, prelude::*};
use static_cell::StaticCell;

pub mod buttons;
pub mod buzzer;
pub mod large_display;
pub mod led_shifter;
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
//pub static RIGHT_BUTTON_LED: ButtonLEDType = Mutex::new(None);
//pub static LEFT_BUTTON_LED: ButtonLEDType = Mutex::new(None);

pub type BuzzerType =
    Mutex<CriticalSectionRawMutex, Option<Output<'static>>>;
//pub static BUZZER_400: BuzzerType = Mutex::new(None);
//pub static BUZZER_2K3: BuzzerType = Mutex::new(None);

pub type RotarySwitchType =
    Mutex<CriticalSectionRawMutex, Option<Input<'static>>>;
pub static ROTARY_SWITCH_LEFT: RotarySwitchType = Mutex::new(None);
pub static ROTARY_SWITCH_RIGHT: RotarySwitchType = Mutex::new(None);

use esp_hal::{
    self,
    delay::Delay,
    i2c::{self},
    main,
    spi::{self, master::Spi},
};

// pub type LEDType =
//     Mutex<CriticalSectionRawMutex, Option<Output<'static>>>;
// pub static RED_LED: LEDType = Mutex::new(None);
// pub static GREEN_LED: LEDType = Mutex::new(None);
// pub static BLUE_LED: LEDType = Mutex::new(None);
// pub static YELLOW_LED: LEDType = Mutex::new(None);
// pub static WHITE_LED: LEDType = Mutex::new(None);

// pub static LED_ARRAY: [&'static LEDType; 5] =
//     [&RED_LED, &GREEN_LED, &BLUE_LED, &YELLOW_LED, &WHITE_LED];

pub type MonoDisplayType =
    Mutex<CriticalSectionRawMutex, Option<MonoDisplay>>;
// pub static DISPLAY: MonoDisplayType = Mutex::new(None);
static BAR: static_cell::StaticCell<MonoDisplayType> =
    static_cell::StaticCell::new();

pub type LedShifterType =
    adv_shift_registers::AdvancedShiftRegister<2, Output<'static>>;

pub type LargeDisplayType = Ili9341<
    SPIInterface<
        ExclusiveDevice<
            Spi<'static, esp_hal::Blocking>,
            Output<'static>,
            NoDelay,
        >,
        Output<'static>,
    >,
    Output<'static>,
>;

struct DummyPwm(Output<'static>);

impl ErrorType for DummyPwm {
    type Error = core::convert::Infallible;
}

impl SetDutyCycle for DummyPwm {
    fn max_duty_cycle(&self) -> u16 {
        255
    }

    fn set_duty_cycle(
        &mut self,
        duty: u16,
    ) -> Result<(), Self::Error> {
        if duty > 0 {
            self.0.set_high();
        } else {
            self.0.set_low();
        }
        Ok(())
    }
}

pub struct NonMutexPeripherals {
    pub mono_display: mono_display::DisplayType,
    pub left_rotary_a: Input<'static>,
    pub left_rotary_b: Input<'static>,
    pub right_rotary_a: Input<'static>,
    pub right_rotary_b: Input<'static>,
    pub flash: FLASH<'static>,
    // it has a buffer size of one because there is only one neopixel
    //pub neopixel: SmartLedsAdapter<'static, 25>,
    //pub i2s_speaker: I2s<'static, esp_hal::Async>,
    pub shifter: LedShifterType,
    //pub large_display: LargeDisplayType,
    pub buzzer_2k3: Output<'static>,
    pub left_button: Input<'static>,
    pub right_button: Input<'static>,
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

    let left_button = Input::new(peripherals.GPIO3, pull_up_config);

    let right_button = Input::new(peripherals.GPIO4, pull_up_config);

    // let left_button_light = Output::new(
    //     peripherals.GPIO4,
    //     Level::Low,
    //     output_config_default,
    // );
    // let right_button_light = Output::new(
    //     peripherals.GPIO12,
    //     Level::Low,
    //     output_config_default,
    // );

    let buzzer_2k3 = Output::new(
        peripherals.GPIO16,
        Level::Low,
        output_config_default,
    );

    let rotary_switch_left =
        Input::new(peripherals.GPIO13, pull_up_config);

    let rotary_switch_right =
        Input::new(peripherals.GPIO14, pull_up_config);

    let reg_rclk = Output::new(
        peripherals.GPIO0,
        Level::Low,
        output_config_default,
    );

    let reg_ser = Output::new(
        peripherals.GPIO1,
        Level::Low,
        output_config_default,
    );

    let reg_srclk = Output::new(
        peripherals.GPIO2,
        Level::Low,
        output_config_default,
    );

    let shifter: adv_shift_registers::AdvancedShiftRegister<
        2,
        Output<'_>,
    > = adv_shift_registers::AdvancedShiftRegister::new(
        reg_ser, reg_srclk, reg_rclk, 0,
    );

    let left_rotary_a = Input::new(peripherals.GPIO9, pull_up_config);
    let left_rotary_b =
        Input::new(peripherals.GPIO11, pull_up_config);

    let right_rotary_a =
        Input::new(peripherals.GPIO10, pull_up_config);
    let right_rotary_b =
        Input::new(peripherals.GPIO12, pull_up_config);

    {
        //*(RIGHT_BUTTON.lock().await) = Some(right_button);
        //*(LEFT_BUTTON.lock().await) = Some(left_button);
        //*(RIGHT_BUTTON_LED.lock().await) = Some(right_button_light);
        //*(LEFT_BUTTON_LED.lock().await) = Some(left_button_light);
        //*(BUZZER_400.lock().await) = Some(buzzer_400);
        //*(BUZZER_2K3.lock().await) = Some(buzzer_2k3);
        *(ROTARY_SWITCH_LEFT.lock().await) = Some(rotary_switch_left);
        *(ROTARY_SWITCH_RIGHT.lock().await) =
            Some(rotary_switch_right);

        // *(RED_LED.lock().await) = Some(red_led);
        // *(GREEN_LED.lock().await) = Some(green_led);
        // *(BLUE_LED.lock().await) = Some(blue_led);
        // *(YELLOW_LED.lock().await) = Some(yellow_led);
        // *(WHITE_LED.lock().await) = Some(white_led);
    }

    // Uncomment this after led handle is done
    // leds::init(
    //     peripherals.GPIO41,
    //     peripherals.GPIO15,
    //     peripherals.GPIO14,
    //     peripherals.GPIO9,
    //     peripherals.GPIO11,
    // ).await;

    // let display = mono_display::init(
    //     peripherals.I2C0,
    //     peripherals.GPIO6,
    //     peripherals.GPIO7,
    // );

    let flash = peripherals.FLASH;

    // let neopixel = neopixel::init(
    //     peripherals.LEDC,
    //     peripherals.RMT,
    //     peripherals.GPIO38,
    //     output_config_default,
    // );

    let mono_display = mono_display::init(
        peripherals.I2C0,
        peripherals.GPIO35,
        peripherals.GPIO21,
    );

    // let i2s_speaker = speaker::init(
    //     peripherals.I2S0,
    //     peripherals.DMA_CH0,
    //     peripherals.GPIO37,
    //     peripherals.GPIO38,
    //     peripherals.GPIO39,
    //     peripherals.GPIO40,
    // );

    //Timer::after(Duration::from_millis(1000)).await;

    // const SPI_FREQUENCY: Rate = Rate::from_mhz(20);

    // let miso = peripherals.GPIO45;
    // let mosi = peripherals.GPIO38;
    // let sclk = peripherals.GPIO36;
    // let cs = peripherals.GPIO48;
    // let spi = Spi::new(
    //     peripherals.SPI2,
    //     spi::master::Config::default()
    //         .with_frequency(SPI_FREQUENCY)
    //         .with_mode(esp_hal::spi::Mode::_0),
    // )
    // .unwrap()
    // .with_sck(sclk)
    // //.with_miso(miso) // order matters, apparently
    // .with_mosi(mosi);
    // //.with_cs(cs);

    // let rst = peripherals.GPIO37;

    // let dc = peripherals.GPIO47;

    // let cs = Output::new(cs, Level::High, OutputConfig::default());

    // let spi_device = ExclusiveDevice::new_no_delay(spi,
    // cs).unwrap();

    // let rst = Output::new(rst, Level::Low,
    // OutputConfig::default());

    // let dc = Output::new(dc, Level::Low, OutputConfig::default());

    // let interface = SPIInterface::new(spi_device, dc);

    // let large_display: LargeDisplayType = Ili9341::new(
    //     interface,
    //     rst,
    //     &mut Delay::new(),
    //     Orientation::Portrait,
    //     DisplaySize240x320,
    // )
    // .unwrap();

    let miso = peripherals.GPIO45;
    let mosi = peripherals.GPIO38;
    let sclk = peripherals.GPIO36;
    let cs = peripherals.GPIO48;

    let dc = Output::new(
        peripherals.GPIO47,
        Level::Low,
        OutputConfig::default(),
    );
    let rst = Output::new(
        peripherals.GPIO37,
        Level::Low,
        OutputConfig::default(),
    );

    // --- SPI ---
    let spi = Spi::new(
        peripherals.SPI2,
        spi::master::Config::default()
            .with_frequency(Rate::from_mhz(10))
            .with_mode(esp_hal::spi::Mode::_0),
    )
    .unwrap()
    .with_sck(sclk)
    .with_mosi(mosi)
    .with_miso(miso)
    .with_cs(cs);

    // --- backlight ---
    let bl_pin = Output::new(
        peripherals.GPIO46,
        Level::Low,
        OutputConfig::default(),
    );
    let bl = DummyPwm(bl_pin);

    // --- LCD ---
    //let mut lcd = Lcd::new(spi, dc, rst, bl)
    //  .with_orientation(LcdOrientation::Rotate0);

    //Timer::after_secs(10).await;

    let mut delay = Delay::new();

    //lcd.init(&mut delay).unwrap();
    //lcd.set_backlight(255).unwrap();

    //lcd.clear(0x0000).unwrap();
    //lcd.fill_rect(10, 10, 50, 50, rgb_to_u16(255, 0, 0));

    NonMutexPeripherals {
        mono_display,
        left_rotary_a,
        left_rotary_b,
        right_rotary_a,
        right_rotary_b,
        flash, //simple_speaker,
        //neopixel,
        shifter, /*i2s_speaker,
                  *large_display, */
        buzzer_2k3,
        left_button,
        right_button,
    }
}
