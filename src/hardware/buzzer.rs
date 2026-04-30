use defmt::warn;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel,
};
use embassy_time::{Duration, Timer};
use esp_hal::gpio;

use crate::settings;

pub static BUZZER_2K3_CH: Channel<
    CriticalSectionRawMutex,
    BuzzerCommand,
    32,
> = Channel::new();

pub static BUZZER_400_CH: Channel<
    CriticalSectionRawMutex,
    BuzzerCommand,
    32,
> = Channel::new();

const CLICK_DURATION: Duration = Duration::from_micros(1500);

pub enum BuzzerCommand {
    Play(Duration),
    /// Make a clicking sound
    Click,
}

#[embassy_executor::task]
pub async fn buzzer_2k3_listener(
    mut buzzer_2k3: gpio::Output<'static>,
) {
    loop {
        let cmd = BUZZER_2K3_CH.receive().await;

        if settings::MUTE_SOUNDS {
            warn!("Sounds are muted.");
            continue;
        }

        match cmd {
            BuzzerCommand::Play(duration) => {
                play(&mut buzzer_2k3, duration).await;
            }
            BuzzerCommand::Click => {
                play(&mut buzzer_2k3, CLICK_DURATION).await;
            }
        }
    }
}

#[embassy_executor::task]
pub async fn buzzer_400_listener(
    mut buzzer_2k3: gpio::Output<'static>,
) {
    loop {
        let cmd = BUZZER_400_CH.receive().await;

        if settings::MUTE_SOUNDS {
            warn!("Sounds are muted.");
            continue;
        }

        match cmd {
            BuzzerCommand::Play(duration) => {
                play(&mut buzzer_2k3, duration).await;
            }
            BuzzerCommand::Click => {
                play(&mut buzzer_2k3, CLICK_DURATION).await;
            }
        }
    }
}

async fn play(
    buzzer: &mut gpio::Output<'static>,
    duration: Duration,
) {
    buzzer.set_high();
    Timer::after(duration).await;
    buzzer.set_low();
}
