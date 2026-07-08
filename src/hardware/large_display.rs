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

use embassy_futures::select::{Either, select};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel,
};
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_10X20},
    pixelcolor::Rgb565,
    prelude::*,
    text::Text,
};
use ili9341::ModeState;

use crate::hardware::LargeDisplayType;

include!(concat!(env!("OUT_DIR"), "/victini.rs"));

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
    SetBrightness(u8),
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
    DrawText90 {
        text: &'static str,
        color: u16,
        scale: u32,
    },
    PlayVictini,
    StopAnimation,
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
            BacklightCommand::SetBrightness(brightness) => {
                if brightness > 0 {
                    bl_pin.set_high();
                } else {
                    bl_pin.set_low();
                }
            }
        }
    }
}

#[embassy_executor::task]
pub async fn large_display_listener(
    mut display: Option<LargeDisplayType>,
) {
    let mut victini_frame = None;

    loop {
        let cmd = if let Some(frame_index) = victini_frame {
            let delay =
                Duration::from_millis(VICTINI_DELAYS_MS[frame_index]);
            match select(
                LARGE_DISPLAY_CH.receive(),
                Timer::after(delay),
            )
            .await
            {
                Either::First(cmd) => cmd,
                Either::Second(()) => {
                    let next_frame =
                        (frame_index + 1) % VICTINI_DELAYS_MS.len();
                    victini_frame = Some(next_frame);
                    if let Some(display) = display.as_mut()
                        && draw_victini_frame(display, next_frame)
                            .is_err()
                    {
                        defmt::error!("failed to draw Victini frame");
                    }
                    continue;
                }
            }
        } else {
            LARGE_DISPLAY_CH.receive().await
        };

        match cmd {
            LargeDisplayCommand::PlayVictini => {
                victini_frame = Some(0);
                if let Some(display) = display.as_mut() {
                    let result = display
                        .clear_screen(0)
                        .and_then(|_| draw_victini_frame(display, 0));
                    if result.is_err() {
                        defmt::error!(
                            "failed to start Victini animation"
                        );
                    }
                }
                continue;
            }
            LargeDisplayCommand::StopAnimation => {
                victini_frame = None;
                continue;
            }
            _ => {}
        }

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
            LargeDisplayCommand::DrawText90 {
                text,
                color,
                scale,
            } => {
                let scale = scale.max(1);
                let text_width = text.len() as u32 * 10;
                let origin = Point::new(
                    (240_u32.saturating_sub(20 * scale) / 2) as i32,
                    (320_u32.saturating_sub(text_width * scale) / 2)
                        as i32,
                );
                let mut rotated = RotatedScaledTarget {
                    display,
                    origin,
                    source_size: Size::new(text_width, 20),
                    scale,
                };

                Text::new(
                    text,
                    Point::new(0, 15),
                    MonoTextStyle::new(
                        &FONT_10X20,
                        Rgb565::new(
                            ((color >> 11) & 0x1f) as u8,
                            ((color >> 5) & 0x3f) as u8,
                            (color & 0x1f) as u8,
                        ),
                    ),
                )
                .draw(&mut rotated)
                .map(|_| ())
            }
            LargeDisplayCommand::PlayVictini
            | LargeDisplayCommand::StopAnimation => unreachable!(),
        };

        if result.is_err() {
            defmt::error!("large display command failed");
        }
    }
}

fn draw_victini_frame(
    display: &mut LargeDisplayType,
    frame_index: usize,
) -> Result<(), ili9341::DisplayError> {
    let start = VICTINI_OFFSETS[frame_index] as usize;
    let end = VICTINI_OFFSETS[frame_index + 1] as usize;
    let frame = &VICTINI_DELTAS[start..end];
    let scale =
        (240 / VICTINI_HEIGHT).min(320 / VICTINI_WIDTH).max(1);
    let origin = Point::new(
        ((240 - VICTINI_HEIGHT * scale) / 2) as i32,
        ((320 - VICTINI_WIDTH * scale) / 2) as i32,
    );
    let mut records = frame.chunks_exact(4).peekable();
    while let Some(bytes) = records.next() {
        let index = u16::from_le_bytes([bytes[0], bytes[1]]) as u32;
        let color = u16::from_le_bytes([bytes[2], bytes[3]]);
        let source_x = index % VICTINI_WIDTH;
        let source_y = index / VICTINI_WIDTH;
        let mut run = 1;

        while let Some(next) = records.peek() {
            let next_index =
                u16::from_le_bytes([next[0], next[1]]) as u32;
            let next_color = u16::from_le_bytes([next[2], next[3]]);
            if next_index != index + run
                || next_index / VICTINI_WIDTH != source_y
                || next_color != color
            {
                break;
            }
            records.next();
            run += 1;
        }

        fill_rect(
            display,
            (origin.x as u32 + source_y * scale) as u16,
            (origin.y as u32
                + (VICTINI_WIDTH - source_x - run) * scale)
                as u16,
            scale as u16,
            (run * scale) as u16,
            color,
        )?;
    }

    Ok(())
}

struct RotatedScaledTarget<'a> {
    display: &'a mut LargeDisplayType,
    origin: Point,
    source_size: Size,
    scale: u32,
}

impl DrawTarget for RotatedScaledTarget<'_> {
    type Color = Rgb565;
    type Error = ili9341::DisplayError;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let origin = self.origin;
        let source_width = self.source_size.width as i32;
        let scale = self.scale as i32;

        self.display.draw_iter(pixels.into_iter().flat_map(
            move |Pixel(point, color)| {
                let x = origin.x + point.y * scale;
                let y =
                    origin.y + (source_width - 1 - point.x) * scale;

                (0..scale).flat_map(move |dx| {
                    (0..scale).map(move |dy| {
                        Pixel(Point::new(x + dx, y + dy), color)
                    })
                })
            },
        ))
    }
}

impl OriginDimensions for RotatedScaledTarget<'_> {
    fn size(&self) -> Size {
        self.source_size
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
