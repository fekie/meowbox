use core::sync::atomic::{AtomicU32, Ordering::Relaxed};

use embassy_time::Instant;

use super::{MenuState, Meowbox, Stage, State};
use crate::{
    hardware::{
        led_shifter::{LED, LED_SHIFTER_CHANNEL, LedCommand},
        speaker::{CRIES_PCM, SPEAKER_CHANNEL, SpeakerCommand},
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
            let cry = CRIES_PCM[random_cry_index()];
            SPEAKER_CHANNEL.send(SpeakerCommand::PlayPcm(cry)).await;
        }
    }

    async fn shutdown_cries(&mut self) {
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
    state as usize % CRIES_PCM.len()
}
