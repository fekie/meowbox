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
    peripherals::{FLASH, Peripherals},
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
