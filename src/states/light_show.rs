use embassy_time::{Duration, Timer};

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

const LIGHT_SHOW_INTERVAL: Duration = Duration::from_millis(500);

impl Meowbox {
    pub(super) async fn tick_light_show(&mut self) {
        let State::LightShow(stage) = self.state else {
            return;
        };

        match stage {
            Stage::Setup => self.setup_light_show().await,
            Stage::Execution => self.execute_light_show().await,
            Stage::Shutdown => self.shutdown_light_show().await,
        }
    }

    async fn setup_light_show(&mut self) {
        MONO_DISPLAY_CH.send(MonoDisplayCommand::Clear).await;
        MONO_DISPLAY_CH
            .send(MonoDisplayCommand::SetDisplayOn(false))
            .await;
        BACKLIGHT_CH.send(BacklightCommand::SetLow).await;
        LARGE_DISPLAY_CH.send(LargeDisplayCommand::DisplayOff).await;
        LED_SHIFTER_CHANNEL.send(LedCommand::SetAllLow).await;
        LED_SHIFTER_CHANNEL
            .send(LedCommand::SetHigh(LED::ButtonLeft))
            .await;

        self.state = State::LightShow(Stage::Execution);
    }

    async fn execute_light_show(&mut self) {
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

        LED_SHIFTER_CHANNEL.send(LedCommand::SetAllHigh).await;
        Timer::after(LIGHT_SHOW_INTERVAL).await;
        LED_SHIFTER_CHANNEL.send(LedCommand::SetAllLow).await;
        LED_SHIFTER_CHANNEL
            .send(LedCommand::SetHigh(LED::ButtonLeft))
            .await;
        Timer::after(LIGHT_SHOW_INTERVAL).await;
    }

    async fn shutdown_light_show(&mut self) {
        LED_SHIFTER_CHANNEL.send(LedCommand::SetAllLow).await;
        BACKLIGHT_CH.send(BacklightCommand::SetLow).await;
        LARGE_DISPLAY_CH.send(LargeDisplayCommand::DisplayOn).await;
        MONO_DISPLAY_CH
            .send(MonoDisplayCommand::SetDisplayOn(true))
            .await;

        self.state = self.next_state.take().unwrap_or(State::Menu(
            Stage::Setup,
            MenuState::default(),
        ));
    }
}
