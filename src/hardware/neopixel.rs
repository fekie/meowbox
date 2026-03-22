use esp_hal::{
    gpio::{Level, Output},
    ledc::{
        Ledc, LowSpeed,
        timer::{self, TimerIFace},
    },
    peripherals::{GPIO48, LEDC, RMT},
    rmt::{PulseCode, Rmt},
    time::Rate,
};
use esp_hal_smartled::{SmartLedsAdapter, buffer_size};
use static_cell::StaticCell;

const RMT_BUFFER_SIZE: usize = 1;
// Honestly have no idea what this does, but 25 is an arbitrary number
// and it works.
const SMART_LEDS_ADAPTER_BUFFER_SIZE: usize = 25;
const NEOPIXEL_PULSE_SPEED_MHZ: u32 = 80;

// The buffer for the control code sequence used by the neopixel.
static RMT_BUFFER: StaticCell<
    [PulseCode; buffer_size(RMT_BUFFER_SIZE)],
> = StaticCell::new();

pub(super) fn init<'a>(
    ledc: LEDC<'a>,
    rmt: RMT<'a>,
    gpio48: GPIO48<'a>,
    output_config_default: esp_hal::gpio::OutputConfig,
) -> SmartLedsAdapter<'a, SMART_LEDS_ADAPTER_BUFFER_SIZE> {
    let ledc_t = Ledc::new(ledc);

    let mut timer = ledc_t.timer::<LowSpeed>(timer::Number::Timer0);
    timer
        .configure(timer::config::Config {
            duty: timer::config::Duty::Duty10Bit,
            clock_source: timer::LSClockSource::APBClk,
            frequency: Rate::from_hz(2000),
        })
        .unwrap();

    // rmt stands for remote control transceiver. More information
    // about RMT is located at https://docs.espressif.com/projects/esp-idf/en/latest/esp32s3/api-reference/peripherals/rmt.html
    let rmt = Rmt::new(rmt, Rate::from_mhz(NEOPIXEL_PULSE_SPEED_MHZ))
        .unwrap();

    let neopixel_pin =
        Output::new(gpio48, Level::Low, output_config_default);

    let rmt_buffer = RMT_BUFFER.init(
        [PulseCode::end_marker(); buffer_size(RMT_BUFFER_SIZE)],
    );

    // channels 0 and 1 are for sending, channels 2 and 3 are for
    // sending
    SmartLedsAdapter::new(rmt.channel0, neopixel_pin, rmt_buffer)
}
