use core::sync::atomic::{AtomicU32, Ordering::Relaxed};

use embassy_time::Instant;
use heapless::String;

use super::{MenuState, Meowbox, Stage, State};
use crate::{
    hardware::{
        large_display::{
            BACKLIGHT_CH, BacklightCommand, LARGE_DISPLAY_CH,
            LargeDisplayCommand,
        },
        led_shifter::{LED, LED_SHIFTER_CHANNEL, LedCommand},
        mono_display::{MONO_DISPLAY_CH, MonoDisplayCommand},
        speaker::{CRIES, SPEAKER_CHANNEL, SpeakerCommand},
    },
    input_listener::{Input, InputListener},
};

static RANDOM_STATE: AtomicU32 = AtomicU32::new(0x9e37_79b9);

impl Meowbox {
    pub(super) async fn tick_cries(&mut self) {
        let State::Cries(stage) = self.state else {
            return;
        };

        match stage {
            Stage::Setup => self.setup_cries().await,
            Stage::Execution => self.execute_cries().await,
            Stage::Shutdown => self.shutdown_cries().await,
        }
    }

    async fn setup_cries(&mut self) {
        LED_SHIFTER_CHANNEL.send(LedCommand::SetAllLow).await;
        LED_SHIFTER_CHANNEL
            .send(LedCommand::SetHigh(LED::ButtonLeft))
            .await;
        LED_SHIFTER_CHANNEL
            .send(LedCommand::SetHigh(LED::ButtonRight))
            .await;

        MONO_DISPLAY_CH
            .send(MonoDisplayCommand::SwitchToTerminal)
            .await;
        MONO_DISPLAY_CH
            .send(MonoDisplayCommand::SetDisplayOn(true))
            .await;
        MONO_DISPLAY_CH.send(MonoDisplayCommand::Clear).await;

        LARGE_DISPLAY_CH.send(LargeDisplayCommand::DisplayOn).await;
        BACKLIGHT_CH.send(BacklightCommand::SetHigh).await;
        LARGE_DISPLAY_CH
            .send(LargeDisplayCommand::PlayVictini)
            .await;

        self.state = State::Cries(Stage::Execution);
    }

    async fn execute_cries(&mut self) {
        if InputListener::take_input(Input::ButtonLeft, true)
            .ok()
            .flatten()
            .is_some()
        {
            self.next_state =
                Some(State::Menu(Stage::Setup, MenuState::default()));
            self.needs_to_shutdown = true;
            return;
        }

        if InputListener::take_input(Input::ButtonRight, true)
            .ok()
            .flatten()
            .is_some()
        {
            let cry_index = random_cry_index();
            let cry = &CRIES[cry_index];
            let filename = String::try_from(cry.filename).unwrap();

            MONO_DISPLAY_CH.send(MonoDisplayCommand::Clear).await;
            MONO_DISPLAY_CH
                .send(MonoDisplayCommand::WriteStr(filename))
                .await;
            SPEAKER_CHANNEL
                .send(SpeakerCommand::PlayPcm(cry.samples))
                .await;
        }
    }

    async fn shutdown_cries(&mut self) {
        LARGE_DISPLAY_CH
            .send(LargeDisplayCommand::StopAnimation)
            .await;
        LED_SHIFTER_CHANNEL.send(LedCommand::SetAllLow).await;
        self.state = self.next_state.take().unwrap_or(State::Menu(
            Stage::Setup,
            MenuState::default(),
        ));
    }
}

fn random_cry_index() -> usize {
    let time = Instant::now().as_ticks() as u32;
    let mut state = RANDOM_STATE.load(Relaxed) ^ time;
    state ^= state << 13;
    state ^= state >> 17;
    state ^= state << 5;
    RANDOM_STATE.store(state, Relaxed);
    state as usize % CRIES.len()
}
