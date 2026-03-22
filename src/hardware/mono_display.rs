use esp_hal::{
    gpio::{Level, Output},
    i2c::master::{Config as I2cConfig, I2c},
    ledc::{
        Ledc, LowSpeed,
        timer::{self, TimerIFace},
    },
    peripherals::{GPIO6, GPIO7, I2C0, LEDC, RMT},
    rmt::{PulseCode, Rmt},
    time::Rate,
};
use esp_hal_smartled::{SmartLedsAdapter, buffer_size};
use ssd1306::{I2CDisplayInterface, Ssd1306Async, prelude::*};
use static_cell::StaticCell;

const I2C_FREQUENCY_KHZ: u32 = 400;

pub type DisplayType = Ssd1306Async<
    I2CInterface<I2c<'static, esp_hal::Async>>,
    DisplaySize128x64,
    ssd1306::mode::BufferedGraphicsModeAsync<DisplaySize128x64>,
>;

// For whatever reason, the compiler requires static here. I am
// assuming something in the code below contains something that is
// 'static.
pub(super) fn init(
    i2c0: I2C0<'static>,
    gpio6: GPIO6<'static>,
    gpio7: GPIO7<'static>,
) -> DisplayType {
    let i2c_bus: I2c<'_, esp_hal::Async> = I2c::new(
        i2c0,
        // I2cConfig is alias of esp_hal::i2c::master::I2c::Config
        I2cConfig::default()
            .with_frequency(Rate::from_khz(I2C_FREQUENCY_KHZ)),
    )
    .unwrap()
    .with_scl(gpio6)
    .with_sda(gpio7)
    .into_async();

    let interface = I2CDisplayInterface::new(i2c_bus);

    // initialize the display
    Ssd1306Async::new(
        interface,
        DisplaySize128x64,
        DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode()
}

use defmt::trace;
#[allow(unused_imports)]
use defmt::{error, info, warn};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel,
};
use embassy_time::{Duration, Timer};
use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};
use heapless::String;
use ssd1306::{
    mode::{BufferedGraphicsModeAsync, TerminalModeAsync},
    prelude::*,
};

pub const MONO_DISPLAY_LINE_WIDTH: usize = 16;

/// A channel to send commands to the display.
pub static MONO_DISPLAY_CH: Channel<
    CriticalSectionRawMutex,
    MonoDisplayCommand,
    20,
> = Channel::new();

// The available commands to send to the display
#[derive(Clone)]
pub enum MonoDisplayCommand {
    /// Usable by Graphics and Terminal
    Init,
    /// Usable by Graphics and Terminal
    Clear,
    /// Usable by Graphics
    SwitchToTerminal,
    /// Usable by Terminal
    SwitchToGraphics,
    /// Write string. Usable by Terminal
    WriteStr(String<MONO_DISPLAY_LINE_WIDTH>),
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
            MonoDisplayCommand::WriteStr(s) => {
                self.cmd_write_str(s).await
            }
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

    async fn cmd_write_str(
        &mut self,
        s: String<MONO_DISPLAY_LINE_WIDTH>,
    ) {
        if let MonoDisplay::Terminal(x) = self {
            match x.write_str(&s).await {
                Ok(()) => trace!("string written to display!"),
                Err(_) => info!("error writing string to display."),
            }
        }
    }
}
