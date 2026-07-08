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
    },
    input_listener::{Input, InputListener},
};

const BLACK: u16 = 0x0000;
const RED: u16 = 0xf800;

impl Meowbox {
    pub(super) async fn tick_unimplemented(&mut self) {
        let State::Unimplemented(stage) = self.state else {
            return;
        };

        match stage {
            Stage::Setup => self.setup_unimplemented().await,
            Stage::Execution => self.execute_unimplemented().await,
            Stage::Shutdown => self.shutdown_unimplemented().await,
        }
    }

    async fn setup_unimplemented(&mut self) {
        LED_SHIFTER_CHANNEL.send(LedCommand::SetAllLow).await;
        LED_SHIFTER_CHANNEL
            .send(LedCommand::SetHigh(LED::ButtonLeft))
            .await;

        MONO_DISPLAY_CH
            .send(MonoDisplayCommand::SwitchToTerminal)
            .await;
        MONO_DISPLAY_CH
            .send(MonoDisplayCommand::SetDisplayOn(true))
            .await;
        MONO_DISPLAY_CH.send(MonoDisplayCommand::Clear).await;
        MONO_DISPLAY_CH
            .send(MonoDisplayCommand::WriteStr(
                String::try_from("UNIMPLEMENTED").unwrap(),
            ))
            .await;

        LARGE_DISPLAY_CH.send(LargeDisplayCommand::DisplayOn).await;
        LARGE_DISPLAY_CH
            .send(LargeDisplayCommand::Clear(BLACK))
            .await;
        LARGE_DISPLAY_CH
            .send(LargeDisplayCommand::DrawText90 {
                text: "UNIMPLEMENTED",
                color: RED,
                scale: 2,
            })
            .await;
        BACKLIGHT_CH.send(BacklightCommand::SetHigh).await;

        self.state = State::Unimplemented(Stage::Execution);
    }

    async fn execute_unimplemented(&mut self) {
        if InputListener::take_input(Input::ButtonLeft, true)
            .ok()
            .flatten()
            .is_some()
        {
            self.next_state =
                Some(State::Menu(Stage::Setup, MenuState::default()));
            self.needs_to_shutdown = true;
        }
    }

    async fn shutdown_unimplemented(&mut self) {
        LED_SHIFTER_CHANNEL.send(LedCommand::SetAllLow).await;
        BACKLIGHT_CH.send(BacklightCommand::SetLow).await;

        self.state = self.next_state.take().unwrap_or(State::Menu(
            Stage::Setup,
            MenuState::default(),
        ));
    }
}
