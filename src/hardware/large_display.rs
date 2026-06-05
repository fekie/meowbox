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
use ili9341::ModeState;

use crate::hardware::LargeDisplayType;

pub static BACKLIGHT_CH: Channel<
    CriticalSectionRawMutex,
    BacklightCommand,
    8,
> = Channel::new();

pub static LARGE_DISPLAY_CH: Channel<
    CriticalSectionRawMutex,
    LargeDisplayCommand,
    8,
> = Channel::new();

use esp_hal::gpio::{self, Level};

pub enum BacklightCommand {
    Toggle,
    SetHigh,
    SetLow,
}

pub enum LargeDisplayCommand {
    Clear(u16),
    FillRect {
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        color: u16,
    },
    DisplayOn,
    DisplayOff,
    InvertOn,
    InvertOff,
    SetBrightness(u8),
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
            BacklightCommand::SetHigh => bl_pin.set_high(),
            BacklightCommand::SetLow => bl_pin.set_low(),
        }
    }
}

#[embassy_executor::task]
pub async fn large_display_listener(
    mut display: Option<LargeDisplayType>,
) {
    loop {
        let cmd = LARGE_DISPLAY_CH.receive().await;

        let Some(display) = display.as_mut() else {
            continue;
        };

        let result = match cmd {
            LargeDisplayCommand::Clear(color) => {
                display.clear_screen(color)
            }
            LargeDisplayCommand::FillRect {
                x,
                y,
                width,
                height,
                color,
            } => fill_rect(display, x, y, width, height, color),
            LargeDisplayCommand::DisplayOn => {
                display.display_mode(ModeState::On)
            }
            LargeDisplayCommand::DisplayOff => {
                display.display_mode(ModeState::Off)
            }
            LargeDisplayCommand::InvertOn => {
                display.invert_mode(ModeState::On)
            }
            LargeDisplayCommand::InvertOff => {
                display.invert_mode(ModeState::Off)
            }
            LargeDisplayCommand::SetBrightness(brightness) => {
                display.brightness(brightness)
            }
        };

        if result.is_err() {
            defmt::error!("large display command failed");
        }
    }
}

fn fill_rect(
    display: &mut LargeDisplayType,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    color: u16,
) -> Result<(), ili9341::DisplayError> {
    if width == 0 || height == 0 {
        return Ok(());
    }

    let x1 = x.saturating_add(width - 1);
    let y1 = y.saturating_add(height - 1);
    let pixels = core::iter::repeat(color)
        .take(width as usize * height as usize);

    display.draw_raw_iter(x, y, x1, y1, pixels)
}
