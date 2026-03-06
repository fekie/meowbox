use core::sync::atomic::{
    AtomicBool, AtomicU8, AtomicU16, Ordering::SeqCst,
};

use embassy_executor::task;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel,
};
use embassy_time::{Duration, Timer};
use esp_hal_smartled::SmartLedsAdapter;
use smart_leds::{RGB8, SmartLedsWrite};

pub static NEOPIXEL_CH: Channel<
    CriticalSectionRawMutex,
    NeopixelCommand,
    5,
> = Channel::new();

pub enum NeopixelCommand {
    /// Sets the hue, sets the pixel to the color
    /// if the pixel is currently on
    SetHue(u16),
    ActivateWithHB {
        hue: u16,
        brightness: u8,
    },
    /// Cycle through all the hues, with 1ms inbetween.
    /// The value is the amount of full cycles to do.
    CycleAllHues(u8),
}

// Updated when a new command is sent.
pub static CURRENT_HUE: AtomicU16 = AtomicU16::new(0);
pub static CURRENT_BRIGHTNESS: AtomicU8 = AtomicU8::new(0);
pub static NEOPIXEL_CURRENTLY_ON: AtomicBool = AtomicBool::new(false);

/// Contains controls for the neopixel. No data is stored inside the
/// struct.
#[derive(Clone, Copy)]
pub struct NeoPixelHandle {}

impl NeoPixelHandle {
    pub fn new() -> Self {
        Self {}
    }

    pub fn current_hue(&self) -> u16 {
        CURRENT_HUE.load(SeqCst)
    }

    pub fn current_brightness(&self) -> u8 {
        CURRENT_BRIGHTNESS.load(SeqCst)
    }

    pub fn neopixel_currently_on(&self) -> bool {
        NEOPIXEL_CURRENTLY_ON.load(SeqCst)
    }
}

impl NeoPixelHandle {
    /// Increment neopixel hue with a positive or negative value.
    /// Final values will be normalized to fit 0-360
    pub async fn increment_neopixel_hue(&self, amount: i32) {
        let current_hue = self.current_hue() as i32;

        let hue = (current_hue + amount).rem_euclid(360) as u16;

        NEOPIXEL_CH.send(NeopixelCommand::SetHue(hue)).await
    }

    pub async fn activate_with_hb(&self, hue: u16, brightness: u8) {
        NEOPIXEL_CH
            .send(NeopixelCommand::ActivateWithHB {
                hue,
                brightness: brightness,
            })
            .await;
    }

    pub async fn cycle_all_hues(&self, cycles: u8) {
        NEOPIXEL_CH
            .send(NeopixelCommand::CycleAllHues(cycles))
            .await;
    }
}

#[task]
pub async fn neopixel_command_listener(
    mut neopixel: SmartLedsAdapter<'static, 25>,
) {
    // TODO: basically make the buzzer beeping a separate task, that
    // waits for a message on a channel
    loop {
        let cmd = NEOPIXEL_CH.receive().await;

        match cmd {
            NeopixelCommand::ActivateWithHB { hue, brightness } => {
                CURRENT_HUE.store(hue, SeqCst);
                CURRENT_BRIGHTNESS.store(brightness, SeqCst);
                NEOPIXEL_CURRENTLY_ON.store(true, SeqCst);

                let rgb = hue_to_rgb(hue, brightness);

                neopixel.write([rgb]).unwrap();
            }
            NeopixelCommand::SetHue(hue) => {
                CURRENT_HUE.store(hue, SeqCst);

                if NEOPIXEL_CURRENTLY_ON.load(SeqCst) {
                    let rgb = hue_to_rgb(
                        hue,
                        CURRENT_BRIGHTNESS.load(SeqCst),
                    );
                    neopixel.write([rgb]).unwrap();
                }
            }
            NeopixelCommand::CycleAllHues(cycles) => {
                let mut hue = CURRENT_HUE.load(SeqCst);

                for _ in 0..cycles {
                    for _ in 0..360 {
                        hue = (hue + 1) % 360;

                        CURRENT_HUE.store(hue, SeqCst);

                        let rgb = hue_to_rgb(
                            hue,
                            CURRENT_BRIGHTNESS.load(SeqCst),
                        );
                        neopixel.write([rgb]).unwrap();

                        Timer::after(Duration::from_millis(2)).await;
                    }
                }
            }
        }

        //button.lock().await.as_mut().unwrap().wait_for_low().await;
        //led.lock().await.as_mut().unwrap().set_low();

        //info!("right button triggered");
    }
}

/// Convert hue (0-360) and brightness (0-255)
/// to RGB.
pub fn hue_to_rgb(hue: u16, brightness: u8) -> RGB8 {
    // normalize hue
    let hue = hue % 360;

    let v = brightness as u16;

    let region = hue / 60;
    let remainder = hue % 60;

    let f = remainder * 255 / 60;

    let p = 0u8;
    let q = (v * (255 - f) / 255) as u8;
    let t = (v * f / 255) as u8;

    match region {
        0 => RGB8 {
            r: brightness,
            g: t,
            b: p,
        },
        1 => RGB8 {
            r: q,
            g: brightness,
            b: p,
        },
        2 => RGB8 {
            r: p,
            g: brightness,
            b: t,
        },
        3 => RGB8 {
            r: p,
            g: q,
            b: brightness,
        },
        4 => RGB8 {
            r: t,
            g: p,
            b: brightness,
        },
        _ => RGB8 {
            r: brightness,
            g: p,
            b: q,
        },
    }
}
