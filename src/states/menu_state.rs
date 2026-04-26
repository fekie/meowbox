use defmt::{dbg, info};
use embassy_time::{Duration, Timer};
use esp_println::println;
use heapless::String;

use super::{Meowbox, State};
use crate::{
    hardware::{
        buzzer::{BUZZER_CH, BuzzerCommand},
        led_shifter::{LED, LED_SHIFTER_CHANNEL, LedCommand},
        mono_display::{
            MONO_DISPLAY_CH, MONO_DISPLAY_LINE_WIDTH,
            MonoDisplayCommand,
        },
    },
    input_listener::{Input, InputListener},
    menu::{MenuGeneralItem, MenuProgram, MenuStatusHandle},
    states::{ErrorStateType, MenuState, Stage},
    tasks::all_leds_off,
};

// Light Ring
impl Meowbox {
    pub(super) async fn tick_menu_state(&mut self) {
        if let State::Menu(stage, _) = self.state {
            match stage {
                Stage::Setup => self.setup_menu_state().await,
                Stage::Execution => self.execute_menu_state().await,
                Stage::Shutdown => self.shutdown_menu_state().await,
            }
        }
    }

    async fn setup_menu_state(&mut self) {
        LED_SHIFTER_CHANNEL
            .send(LedCommand::SetHigh(LED::ButtonRight))
            .await;

        LED_SHIFTER_CHANNEL
            .send(LedCommand::SetHigh(LED::AmberLeft))
            .await;

        MONO_DISPLAY_CH
            .send(MonoDisplayCommand::SwitchToTerminal)
            .await;

        MONO_DISPLAY_CH.send(MonoDisplayCommand::Init).await;

        MONO_DISPLAY_CH.send(MonoDisplayCommand::Clear).await;

        // do blank line
        MONO_DISPLAY_CH
            .send(MonoDisplayCommand::WriteStr(
                String::try_from(" \n").unwrap(),
            ))
            .await;

        // display the menu

        //test_text_on_each_line().await;

        // turn all leds off and go to next state
        //all_leds_off().await;
        self.state =
            State::Menu(Stage::Execution, MenuState::default());
    }

    async fn execute_menu_state(&mut self) {
        if let State::Menu(_, _) = &mut self.state {
            // Display stuff here
            //info!("display menu");
        }

        let left_rotary_encoder_pressed = InputListener::take_input(
            Input::RotaryEncoderPressLeft,
            true,
        )
        .unwrap()
        .unwrap_or_default()
            != 0;

        if left_rotary_encoder_pressed {
            //println!("AAAA WHY THIS TRIGGER");
            BUZZER_CH
                .send(BuzzerCommand::Play(Duration::from_millis(50)))
                .await;
        }

        let menu_status_handle = MenuStatusHandle::new();

        // if it doesnt need an update then return early
        if !menu_status_handle.needs_update() {
            return;
        }

        // set it to no longer need update
        menu_status_handle.set_needs_update(false);

        let scroll = menu_status_handle.scroll();

        MONO_DISPLAY_CH.send(MonoDisplayCommand::Clear).await;

        MONO_DISPLAY_CH
            .send(MonoDisplayCommand::WriteStr(
                String::try_from(" \n").unwrap(),
            ))
            .await;

        // let menu_offset = CURRENT_MENU_SCROLL
        //     .load(core::sync::atomic::Ordering::SeqCst);

        for general_item in self
            .resources
            .menu_resoures
            .menu_tree
            .layer_0
            .iter()
            .skip(scroll)
            // we only show the first 7, because this is all that fits
            // on the screen
            .take(7)
        {
            // let name = match general_item {
            //     MenuGeneralItem::MenuProgram(x) => {
            //         String::from(x.as_str())
            //     }
            //     MenuGeneralItem::MenuFolder(x) => x.to_string(),
            // };

            let name: String<MONO_DISPLAY_LINE_WIDTH> =
                match general_item {
                    MenuGeneralItem::MenuProgram(x) => {
                        let mut combined: String<16> = String::new();
                        combined.push_str("#").unwrap();
                        combined.push_str(&x.as_str()).unwrap();
                        combined
                    }
                    MenuGeneralItem::MenuFolder(x) => {
                        let mut combined: String<16> = String::new();
                        combined.push_str("/").unwrap();
                        combined.push_str(&x.as_str()).unwrap();
                        combined
                        //String::try_from(x.as_str()).unwrap()
                    }
                };

            MONO_DISPLAY_CH
                .send(MonoDisplayCommand::WriteStr(name))
                .await;

            MONO_DISPLAY_CH
                .send(MonoDisplayCommand::WriteStr(
                    String::try_from(" \n").unwrap(),
                ))
                .await;

            // MONO_DISPLAY_CH
            //     .send(MonoDisplayCommand::WriteStr(
            //         String::try_from("aaaa▓█▄▀│").unwrap(),
            //     ))
            //     .await;
        }

        Timer::after(Duration::from_millis(1)).await;
    }

    /// This method is called if the state is in shutdown. Shutdown
    /// is only started when an item exists in next_state.
    async fn shutdown_menu_state(&mut self) {
        // TODO: turn all lights off
        //all_leds_off().await;

        self.state = match self.next_state.take() {
            Some(x) => x,
            None => State::ErrorState(
                ErrorStateType::NextStateNotSpecified,
            ),
        }
    }
}

#[allow(dead_code)]
async fn test_text_on_each_line() {
    // change display to terminal (and waits for it to happen)
    MONO_DISPLAY_CH
        .send(MonoDisplayCommand::SwitchToTerminal)
        .await;

    // then init
    MONO_DISPLAY_CH.send(MonoDisplayCommand::Init).await;

    MONO_DISPLAY_CH.send(MonoDisplayCommand::Clear).await;

    // send a string to screen
    let s: String<MONO_DISPLAY_LINE_WIDTH> =
        String::try_from("meowbox").unwrap();
    MONO_DISPLAY_CH.send(MonoDisplayCommand::WriteStr(s)).await;

    MONO_DISPLAY_CH
        .send(MonoDisplayCommand::WriteStr(
            String::try_from(" \non!").unwrap(),
        ))
        .await;

    MONO_DISPLAY_CH
        .send(MonoDisplayCommand::WriteStr(
            String::try_from(" \naon!").unwrap(),
        ))
        .await;

    MONO_DISPLAY_CH
        .send(MonoDisplayCommand::WriteStr(
            String::try_from(" \nbon!").unwrap(),
        ))
        .await;
    MONO_DISPLAY_CH
        .send(MonoDisplayCommand::WriteStr(
            String::try_from(" \ncon!").unwrap(),
        ))
        .await;
    MONO_DISPLAY_CH
        .send(MonoDisplayCommand::WriteStr(
            String::try_from(" \ndon!").unwrap(),
        ))
        .await;
    MONO_DISPLAY_CH
        .send(MonoDisplayCommand::WriteStr(
            String::try_from(" \neon!").unwrap(),
        ))
        .await;
    MONO_DISPLAY_CH
        .send(MonoDisplayCommand::WriteStr(
            String::try_from(" \nfon!").unwrap(),
        ))
        .await;
    MONO_DISPLAY_CH
        .send(MonoDisplayCommand::WriteStr(
            String::try_from(" \ngon!").unwrap(),
        ))
        .await;
}
