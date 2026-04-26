use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel,
};
use embassy_time::{Duration, Timer};
use esp_hal::gpio;

pub static BUZZER_CH: Channel<
    CriticalSectionRawMutex,
    BuzzerCommand,
    8,
> = Channel::new();

pub enum BuzzerCommand {
    Play(Duration),
}

#[embassy_executor::task]
pub async fn buzzer_listener(mut buzzer_2k3: gpio::Output<'static>) {
    loop {
        let cmd = BUZZER_CH.receive().await;

        match cmd {
            BuzzerCommand::Play(duration) => {
                play(&mut buzzer_2k3, duration).await;
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
