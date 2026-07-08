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

struct PokemonSprite {
    pokemon_id: u16,
    width: u32,
    height: u32,
    delays_ms: &'static [u64],
    offsets: &'static [u32],
    deltas: &'static [u8],
}

static POKEMON_SPRITES: &[PokemonSprite] =
    include!(concat!(env!("OUT_DIR"), "/pokemon_sprites.rs"));

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
    PlayPokemon(u16),
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
    let mut pokemon_frame: Option<(usize, usize)> = None;

    loop {
        let cmd = if let Some((sprite_index, frame_index)) =
            pokemon_frame
        {
            let sprite = &POKEMON_SPRITES[sprite_index];
            let delay =
                Duration::from_millis(sprite.delays_ms[frame_index]);
            match select(
                LARGE_DISPLAY_CH.receive(),
                Timer::after(delay),
            )
            .await
            {
                Either::First(cmd) => cmd,
                Either::Second(()) => {
                    let next_frame =
                        (frame_index + 1) % sprite.delays_ms.len();
                    pokemon_frame = Some((sprite_index, next_frame));
                    if let Some(display) = display.as_mut()
                        && draw_pokemon_frame(
                            display, sprite, next_frame,
                        )
                        .is_err()
                    {
                        defmt::error!("failed to draw Pokemon frame");
                    }
                    continue;
                }
            }
        } else {
            LARGE_DISPLAY_CH.receive().await
        };

        match cmd {
            LargeDisplayCommand::PlayPokemon(pokemon_id) => {
                let Some(sprite_index) =
                    POKEMON_SPRITES.iter().position(|sprite| {
                        sprite.pokemon_id == pokemon_id
                    })
                else {
                    defmt::error!(
                        "no sprite for Pokemon {}",
                        pokemon_id
                    );
                    continue;
                };
                let sprite = &POKEMON_SPRITES[sprite_index];
                pokemon_frame = Some((sprite_index, 0));
                if let Some(display) = display.as_mut() {
                    let result = draw_bars(display).and_then(|_| {
                        draw_pokemon_frame(display, sprite, 0)
                    });
                    if result.is_err() {
                        defmt::error!(
                            "failed to start Pokemon animation"
                        );
                    }
                }
                continue;
            }
            LargeDisplayCommand::StopAnimation => {
                pokemon_frame = None;
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
            LargeDisplayCommand::PlayPokemon(_)
            | LargeDisplayCommand::StopAnimation => unreachable!(),
        };

        if result.is_err() {
            defmt::error!("large display command failed");
        }
    }
}

fn draw_pokemon_frame(
    display: &mut LargeDisplayType,
    sprite: &PokemonSprite,
    frame_index: usize,
) -> Result<(), ili9341::DisplayError> {
    let start = sprite.offsets[frame_index] as usize;
    let end = sprite.offsets[frame_index + 1] as usize;
    let frame = &sprite.deltas[start..end];
    let scale = (240 / sprite.height).min(320 / sprite.width).max(1);
    let origin = Point::new(
        ((240 - sprite.height * scale) / 2) as i32,
        ((320 - sprite.width * scale) / 2) as i32,
    );
    let mut records = frame.chunks_exact(4).peekable();
    while let Some(bytes) = records.next() {
        let encoded_index = u16::from_le_bytes([bytes[0], bytes[1]]);
        let transparent = encoded_index & 0x8000 != 0;
        let index = (encoded_index & 0x7fff) as u32;
        let color = u16::from_le_bytes([bytes[2], bytes[3]]);
        let source_x = index % sprite.width;
        let source_y = index / sprite.width;
        let mut run = 1;

        while let Some(next) = records.peek() {
            let next_encoded = u16::from_le_bytes([next[0], next[1]]);
            let next_transparent = next_encoded & 0x8000 != 0;
            let next_index = (next_encoded & 0x7fff) as u32;
            let next_color = u16::from_le_bytes([next[2], next[3]]);
            if next_index != index + run
                || next_index / sprite.width != source_y
                || next_transparent != transparent
                || (!transparent && next_color != color)
            {
                break;
            }
            records.next();
            run += 1;
        }

        let x = (origin.x as u32 + source_y * scale) as u16;
        let y = (origin.y as u32
            + (sprite.width - source_x - run) * scale)
            as u16;
        let width = scale as u16;
        let height = (run * scale) as u16;
        if transparent {
            fill_bars_rect(display, x, y, width, height)?;
        } else {
            fill_rect(display, x, y, width, height, color)?;
        }
    }

    Ok(())
}

const BAR_WIDTH: u16 = 16;
const BAR_GRAY: u16 = 0x8410;

fn draw_bars(
    display: &mut LargeDisplayType,
) -> Result<(), ili9341::DisplayError> {
    display.clear_screen(0)?;
    for x in (0..240).step_by((BAR_WIDTH * 2) as usize) {
        fill_rect(
            display,
            x,
            0,
            BAR_WIDTH.min(240 - x),
            320,
            BAR_GRAY,
        )?;
    }
    Ok(())
}

fn fill_bars_rect(
    display: &mut LargeDisplayType,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
) -> Result<(), ili9341::DisplayError> {
    let x_end = x + width;
    let mut bar_x = x;
    while bar_x < x_end {
        let segment_width =
            (BAR_WIDTH - bar_x % BAR_WIDTH).min(x_end - bar_x);
        let color = if (bar_x / BAR_WIDTH) % 2 == 0 {
            BAR_GRAY
        } else {
            0
        };
        fill_rect(display, bar_x, y, segment_width, height, color)?;
        bar_x += segment_width;
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
