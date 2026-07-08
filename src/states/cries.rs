use heapless::String;
use rotary_encoder_embedded::Direction;

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

const CRIES_VOLUME_MULTIPLIER: f32 = 0.75;

impl Meowbox {
    pub(super) async fn tick_cries(&mut self) {
        let State::Cries(stage, cry_index) = self.state else {
            return;
        };

        match stage {
            Stage::Setup => self.setup_cries(cry_index).await,
            Stage::Execution => self.execute_cries(cry_index).await,
            Stage::Shutdown => self.shutdown_cries().await,
        }
    }

    async fn setup_cries(&mut self, cry_index: usize) {
        LED_SHIFTER_CHANNEL.send(LedCommand::SetAllLow).await;
        LED_SHIFTER_CHANNEL
            .send(LedCommand::SetHigh(LED::ButtonLeft))
            .await;
        LED_SHIFTER_CHANNEL
            .send(LedCommand::SetHigh(LED::ButtonRight))
            .await;
        LED_SHIFTER_CHANNEL
            .send(LedCommand::SetHigh(LED::DpadLeft))
            .await;
        LED_SHIFTER_CHANNEL
            .send(LedCommand::SetHigh(LED::DpadRight))
            .await;

        MONO_DISPLAY_CH
            .send(MonoDisplayCommand::SwitchToTerminal)
            .await;
        MONO_DISPLAY_CH
            .send(MonoDisplayCommand::SetDisplayOn(true))
            .await;
        let _ = InputListener::take_input(Input::DpadLeft, true);
        let _ = InputListener::take_input(Input::DpadRight, true);
        let _ = InputListener::take_input(
            Input::RotaryEncoderRotateLeft(Direction::Clockwise),
            true,
        );
        let _ = InputListener::take_input(
            Input::RotaryEncoderRotateLeft(Direction::Anticlockwise),
            true,
        );
        let _ = InputListener::take_input(
            Input::RotaryEncoderRotateRight(Direction::Clockwise),
            true,
        );
        let _ = InputListener::take_input(
            Input::RotaryEncoderRotateRight(Direction::Anticlockwise),
            true,
        );
        LARGE_DISPLAY_CH.send(LargeDisplayCommand::DisplayOn).await;
        BACKLIGHT_CH.send(BacklightCommand::SetHigh).await;
        show_cry(cry_index).await;

        self.state = State::Cries(Stage::Execution, cry_index);
    }

    async fn execute_cries(&mut self, cry_index: usize) {
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
            let cry = &CRIES[cry_index];
            SPEAKER_CHANNEL
                .send(SpeakerCommand::PlayPcmWithVolume {
                    samples: cry.samples,
                    volume_multiplier: CRIES_VOLUME_MULTIPLIER,
                })
                .await;
        }

        let previous = take_total(Input::DpadLeft)
            + take_total(Input::RotaryEncoderRotateLeft(
                Direction::Anticlockwise,
            ))
            + take_total(Input::RotaryEncoderRotateRight(
                Direction::Anticlockwise,
            ));
        let next = take_total(Input::DpadRight)
            + take_total(Input::RotaryEncoderRotateLeft(
                Direction::Clockwise,
            ))
            + take_total(Input::RotaryEncoderRotateRight(
                Direction::Clockwise,
            ));
        if previous != 0 || next != 0 {
            let count = CRIES.len();
            let next_index = (cry_index + next % count + count
                - previous % count)
                % count;
            self.state = State::Cries(Stage::Execution, next_index);
            show_cry(next_index).await;
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

fn take_total(input: Input) -> usize {
    InputListener::take_input(input, true)
        .ok()
        .flatten()
        .unwrap_or(0) as usize
}

async fn show_cry(cry_index: usize) {
    let cry = &CRIES[cry_index];
    let filename = String::try_from(cry.filename).unwrap();

    MONO_DISPLAY_CH.send(MonoDisplayCommand::Clear).await;
    MONO_DISPLAY_CH
        .send(MonoDisplayCommand::WriteStr(filename))
        .await;
    LARGE_DISPLAY_CH
        .send(LargeDisplayCommand::PlayPokemon(cry.pokemon_id))
        .await;
}
