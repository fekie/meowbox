use core::sync::atomic::{AtomicUsize, Ordering::SeqCst};

use defmt::{dbg, info};
use embassy_time::{Duration, Timer};
use esp_println::println;
use heapless::String;
use menu::{MenuGeneralItem, MenuProgram, MenuStatusHandle};
use rotary_encoder_embedded::Direction;

use super::{Meowbox, State};
use crate::{
    hardware::{
        buttons::DPAD_DEBOUNCE,
        buzzer::{BUZZER_CH, BuzzerCommand},
        led_shifter::{LED, LED_SHIFTER_CHANNEL, LedCommand},
        mono_display::{
            MONO_DISPLAY_CH, MONO_DISPLAY_LINE_WIDTH,
            MonoDisplayCommand,
        },
        speaker::{SPEAKER_CHANNEL, SpeakerCommand},
    },
    input_listener::{Input, InputListener, KillSignal},
    states::{ErrorStateType, MenuState, Stage},
    tasks::all_leds_off,
};

pub mod menu;

pub static LED_SCROLL_INDEX: AtomicUsize = AtomicUsize::new(0);

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

        LED_SHIFTER_CHANNEL
            .send(LedCommand::SetHigh(LED::DpadBottom))
            .await;

        LED_SHIFTER_CHANNEL
            .send(LedCommand::SetHigh(LED::DpadTop))
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

        let _ = handle_inputs().await;

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

        let mut current_items = self
            .resources
            .menu_resoures
            .menu_tree
            .layer_0
            .iter()
            .skip(scroll)
            // we only show the first 7, because this is all that fits
            // on the screen
            .take(7)
            .peekable();

        let top = **current_items.peek().unwrap();

        for general_item in current_items {
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

/// Returns Err(KillSignal) if the kill signal was given
async fn handle_inputs() -> Result<(), KillSignal> {
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

    let left_rotary_encoder_cw = InputListener::take_input(
        Input::RotaryEncoderRotateLeft(Direction::Clockwise),
        true,
    )?
    .unwrap_or_default();

    let dpad_bottom =
        InputListener::take_input(Input::DpadBottom, true)?
            .unwrap_or_default();

    let scroll_down_amount = left_rotary_encoder_cw + dpad_bottom;

    for _ in 0..left_rotary_encoder_cw {
        menu_scroll_down().await;

        LED_SHIFTER_CHANNEL
            .send(LedCommand::TemporaryToggle(
                LED::AmberLeft,
                Duration::from_millis(200),
            ))
            .await;
    }

    for _ in 0..dpad_bottom {
        menu_scroll_down().await;

        LED_SHIFTER_CHANNEL
            .send(LedCommand::TemporaryToggle(
                LED::DpadBottom,
                Duration::from_millis(200),
            ))
            .await;
    }

    let left_rotary_encoder_ccw = InputListener::take_input(
        Input::RotaryEncoderRotateLeft(Direction::Anticlockwise),
        true,
    )?
    .unwrap_or_default();

    let dpad_top = InputListener::take_input(Input::DpadTop, true)?
        .unwrap_or_default();

    let scroll_up_amount = left_rotary_encoder_ccw + dpad_top;

    for _ in 0..left_rotary_encoder_ccw {
        menu_scroll_up().await;
        LED_SHIFTER_CHANNEL
            .send(LedCommand::TemporaryToggle(
                LED::AmberLeft,
                Duration::from_millis(200),
            ))
            .await;
    }

    for _ in 0..dpad_top {
        menu_scroll_up().await;

        LED_SHIFTER_CHANNEL
            .send(LedCommand::TemporaryToggle(
                LED::DpadTop,
                Duration::from_millis(200),
            ))
            .await;
    }

    let total_changes = left_rotary_encoder_cw
        + dpad_bottom
        + left_rotary_encoder_ccw
        + dpad_top;
    for _ in 0..total_changes {
        BUZZER_CH.send(BuzzerCommand::Click).await;
    }

    let old_led_scroll_index =
        LED_SCROLL_INDEX.load(core::sync::atomic::Ordering::SeqCst);

    let new_led_scroll_index = (old_led_scroll_index as i8
        + scroll_down_amount as i8
        - scroll_up_amount as i8)
        .rem_euclid(6) as usize;

    update_led_scroll_bar(old_led_scroll_index, new_led_scroll_index)
        .await;

    LED_SCROLL_INDEX.store(new_led_scroll_index, SeqCst);

    let button_left =
        InputListener::take_input(Input::ButtonLeft, true)?;

    if button_left.is_some() {
        SPEAKER_CHANNEL
            .send(SpeakerCommand::Sine440Hz(Duration::from_secs(5)))
            .await;
        println!("hit left button");
    }

    let button_right =
        InputListener::take_input(Input::ButtonRight, true)?;

    if button_right.is_some() {
        println!("hit right button");
    }

    Ok(())
}

/// Converts an index to an LED. A mapping is created here because
/// the hardware mappings are not neccessarily in order.
const LED_SCROLL_BAR_MAPPING: [LED; 6] = [
    LED::Red,
    LED::Orange,
    LED::YellowCenter,
    LED::Green,
    LED::Blue,
    LED::White,
];

const LED_SCROLL_BAR_TOGGLE_TIME: Duration =
    Duration::from_millis(100);

async fn update_led_scroll_bar(
    old_bar_index: usize,
    new_bar_index: usize,
) {
    if old_bar_index == new_bar_index {
        return;
    }

    let led = LED_SCROLL_BAR_MAPPING[new_bar_index];

    LED_SHIFTER_CHANNEL
        .send(LedCommand::TemporaryToggle(
            led,
            LED_SCROLL_BAR_TOGGLE_TIME,
        ))
        .await;
}

// Scrolls down the menu by 1 (which increments the scroll offset)
async fn menu_scroll_down() {
    let menu_status_handle = MenuStatusHandle::new();

    let mut scroll = menu_status_handle.scroll();
    scroll = (scroll + 1) % menu_status_handle.current_layer_size();
    menu_status_handle.set_scroll(scroll);
    menu_status_handle.set_needs_update(true);

    LED_SHIFTER_CHANNEL
        .send(LedCommand::TemporaryToggle(
            LED::AmberLeft,
            Duration::from_millis(10),
        ))
        .await;
}

// Scrolls up the menu by 1 (which decrements the scroll offset)
async fn menu_scroll_up() {
    let menu_status_handle = MenuStatusHandle::new();

    let mut scroll = menu_status_handle.scroll();
    if scroll == 0 {
        scroll = menu_status_handle.current_layer_size() - 1;
    } else {
        scroll -= 1;
    }
    menu_status_handle.set_scroll(scroll);
    menu_status_handle.set_needs_update(true);
}
