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

/// A channel to send commands to the display.
pub static MONO_DISPLAY_CH: Channel<
    CriticalSectionRawMutex,
    MonoDisplayCommand,
    20,
> = Channel::new();

// The available commands to send to the display
#[derive(Clone, Copy)]
pub enum MonoDisplayCommand {
    Init,
    Clear,
    SwitchToTerminal,
    SwitchToGraphics,
}

#[embassy_executor::task]
pub async fn display_task(mut display: MonoDisplay) {
    loop {
        let cmd = MONO_DISPLAY_CH.receive().await;

        // do a check to see if the command was to switch operating
        // modes (which requires ownership). Otherwise, execute the
        // command as normal.
        match cmd {
            MonoDisplayCommand::SwitchToGraphics => {
                display = display.to_graphics().await;
            }
            MonoDisplayCommand::SwitchToTerminal => {
                display = display.to_terminal().await;
            }
            _ => display.process_command(cmd).await,
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

    async fn process_command(&mut self, cmd: MonoDisplayCommand) {
        match cmd {
            MonoDisplayCommand::Init => self.cmd_init().await,
            MonoDisplayCommand::Clear => self.cmd_clear().await,
            // MonoDisplayCommand::SwitchToTerminal => match self {
            //     MonoDisplay::Graphics(x) => {
            //         *self =
            //
            // MonoDisplay::Terminal(x.into_terminal_mode())
            //     }
            //     _ => {}
            // },
            // MonoDisplayCommand::SwitchToGraphics => match self {
            //     MonoDisplay::Terminal(x) => {
            //         *self = MonoDisplay::Graphics(
            //             x.into_buffered_graphics_mode(),
            //         )
            //     }
            MonoDisplayCommand::SwitchToGraphics
            | MonoDisplayCommand::SwitchToTerminal => {}
        }
    }
}

impl MonoDisplay {
    async fn cmd_init(&mut self) {
        loop {
            match self {
                MonoDisplay::Terminal(x) => match x.init().await {
                    Ok(_) => {
                        info!("display initialized (terminal)!");
                        break;
                    }
                    Err(_) => {
                        error!("display init failed (terminal)!");
                        Timer::after(Duration::from_millis(200))
                            .await;
                    }
                },
                MonoDisplay::Graphics(x) => match x.init().await {
                    Ok(_) => {
                        info!("display initialized (graphics)!");
                        break;
                    }
                    Err(_) => {
                        error!("display init failed (graphics)!");
                        Timer::after(Duration::from_millis(200))
                            .await;
                    }
                },
            }
        }
    }

    async fn cmd_clear(&mut self) {
        match self {
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
        }
    }
}
