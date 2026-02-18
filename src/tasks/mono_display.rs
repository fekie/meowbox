#[allow(unused_imports)]
use defmt::{error, info, warn};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel,
};
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::{Point, *},
    text::{Baseline, Text},
};
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use ssd1306::{
    I2CDisplayInterface, Ssd1306Async,
    mode::{BufferedGraphicsModeAsync, TerminalModeAsync},
    prelude::*,
};

// The available commands
pub enum MonoDisplayCommand {
    Init,
    Clear,
    SwitchToTerminal,
    SwitchToGraphics,
}

static MONO_DISPLAY_CH: Channel<
    CriticalSectionRawMutex,
    MonoDisplayCommand,
    20,
> = Channel::new();

#[embassy_executor::task]
async fn display_task(mut display: MonoDisplay) {
    loop {
        match MONO_DISPLAY_CH.receive().await {
            MonoDisplayCommand::Init => loop {
                match &mut display {
                    MonoDisplay::Terminal(x) => {
                        match x.init().await {
                            Ok(_) => {
                                info!(
                                    "display initialized (terminal)!"
                                );
                                break;
                            }
                            Err(_) => {
                                error!(
                                    "display init failed (terminal)!"
                                );
                                Timer::after(Duration::from_millis(
                                    200,
                                ))
                                .await;
                            }
                        }
                    }
                    MonoDisplay::Graphics(x) => {
                        match x.init().await {
                            Ok(_) => {
                                info!(
                                    "display initialized (graphics)!"
                                );
                                break;
                            }
                            Err(_) => {
                                error!(
                                    "display init failed (graphics)!"
                                );
                                Timer::after(Duration::from_millis(
                                    200,
                                ))
                                .await;
                            }
                        }
                    }
                }
            },
            MonoDisplayCommand::Clear => match &mut display {
                MonoDisplay::Terminal(x) => {
                    if x.clear().await.is_err() {
                        info!("error on clear");
                    }
                }
                MonoDisplay::Graphics(x) => {
                    if x.clear(BinaryColor::Off).is_err() {
                        info!("error on clear");
                    }
                    if x.flush().await.is_err() {
                        info!("error on flush");
                    }
                }
            },
            MonoDisplayCommand::SwitchToTerminal => {
                display = match display {
                    MonoDisplay::Graphics(x) => {
                        MonoDisplay::Terminal(x.into_terminal_mode())
                    }
                    other => other,
                }
            }
            MonoDisplayCommand::SwitchToGraphics => {
                display = match display {
                    MonoDisplay::Terminal(x) => {
                        MonoDisplay::Graphics(
                            x.into_buffered_graphics_mode(),
                        )
                    }
                    other => other,
                }
            } //_ => {}
        }
    }
}

pub type GraphicsMonoDisplayType = Ssd1306Async<
    I2CInterface<I2c<'static, esp_hal::Async>>,
    DisplaySize128x64,
    BufferedGraphicsModeAsync<DisplaySize128x64>,
>;

pub type TerminalMonoDisplayType = Ssd1306Async<
    I2CInterface<I2c<'static, esp_hal::Async>>,
    DisplaySize128x64,
    TerminalModeAsync,
>;

/// The struct that represents the 128x64 i2s display. An enum is used
/// as the display needs to be able to switch between terminal
/// and graphics mode while the program is running.
pub enum MonoDisplay {
    Graphics(GraphicsMonoDisplayType),
    Terminal(TerminalMonoDisplayType),
}

impl MonoDisplay {
    pub async fn to_terminal(self) -> Self {
        match self {
            Self::Graphics(d) => {
                Self::Terminal(d.into_terminal_mode())
            }
            other => other,
        }
    }

    pub async fn to_graphics(self) -> Self {
        match self {
            Self::Terminal(d) => {
                Self::Graphics(d.into_buffered_graphics_mode())
            }
            other => other,
        }
    }
}
