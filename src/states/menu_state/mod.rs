use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};

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
        buzzer::{BUZZER_2K3_CH, BUZZER_400_CH, BuzzerCommand},
        large_display::{
            BACKLIGHT_CH, BacklightCommand, LARGE_DISPLAY_CH,
            LargeDisplayCommand,
        },
        led_shifter::{LED, LED_SHIFTER_CHANNEL, LedCommand},
        mono_display::{
            MONO_DISPLAY_CH, MONO_DISPLAY_LINE_WIDTH,
            MonoDisplayCommand,
        },
        speaker::{SPEAKER_CHANNEL, SpeakerCommand},
        thumbwheel::ThumbwheelHandle,
    },
    input_listener::{Input, InputListener, KillSignal},
    states::{ErrorStateType, MenuState, Stage},
    tasks::all_leds_off,
};

pub mod menu;

pub static LED_SCROLL_INDEX: AtomicUsize = AtomicUsize::new(0);
static RIGHT_ROTARY_DISPLAY_INDEX: AtomicUsize = AtomicUsize::new(0);
static RIGHT_ROTARY_SNAKE_INDEX: AtomicUsize = AtomicUsize::new(0);
static RIGHT_ROTARY_DISPLAY_INITIALIZED: AtomicBool =
    AtomicBool::new(false);
static LAST_RIGHT_THUMBWHEEL_BACKLIGHT: AtomicUsize =
    AtomicUsize::new(usize::MAX);

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

        poll_right_thumbwheel_backlight().await;

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

const THUMBWHEEL_PRINT_THRESHOLD: usize = 16;

async fn poll_right_thumbwheel_backlight() {
    let Some(right_thumbwheel) = ThumbwheelHandle::right_raw().await
    else {
        return;
    };

    let backlight = usize::from(right_thumbwheel);
    let last_backlight = LAST_RIGHT_THUMBWHEEL_BACKLIGHT.load(SeqCst);

    if last_backlight != usize::MAX
        && last_backlight.abs_diff(backlight)
            < THUMBWHEEL_PRINT_THRESHOLD
    {
        return;
    }

    LAST_RIGHT_THUMBWHEEL_BACKLIGHT.store(backlight, SeqCst);

    println!("right thumbwheel changed: adc={}", right_thumbwheel);
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
        BUZZER_2K3_CH
            .send(BuzzerCommand::Play(Duration::from_millis(50)))
            .await;
    }

    let right_rotary_encoder_pressed = InputListener::take_input(
        Input::RotaryEncoderPressRight,
        true,
    )
    .unwrap()
    .unwrap_or_default()
        != 0;

    if right_rotary_encoder_pressed {
        //println!("AAAA WHY THIS TRIGGER");
        // BUZZER_400_CH
        //     .send(BuzzerCommand::Play(Duration::from_millis(2000)))
        //     .await;
        BACKLIGHT_CH.send(BacklightCommand::SetHigh).await;
        draw_right_rotary_display_press().await;
    }

    let right_rotary_encoder_cw = InputListener::take_input(
        Input::RotaryEncoderRotateRight(Direction::Clockwise),
        true,
    )?
    .unwrap_or_default();

    let right_rotary_encoder_ccw = InputListener::take_input(
        Input::RotaryEncoderRotateRight(Direction::Anticlockwise),
        true,
    )?
    .unwrap_or_default();

    if right_rotary_encoder_cw != 0 || right_rotary_encoder_ccw != 0 {
        BACKLIGHT_CH.send(BacklightCommand::SetHigh).await;

        move_right_rotary_display_square(
            right_rotary_encoder_cw,
            right_rotary_encoder_ccw,
        )
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
        + dpad_top
        + right_rotary_encoder_cw
        + right_rotary_encoder_ccw;
    for _ in 0..total_changes {
        BUZZER_2K3_CH.send(BuzzerCommand::Click).await;
    }

    let old_led_scroll_index =
        LED_SCROLL_INDEX.load(core::sync::atomic::Ordering::SeqCst);

    let new_led_scroll_index = (old_led_scroll_index as i32
        + scroll_down_amount as i32
        - scroll_up_amount as i32)
        .rem_euclid(6) as usize;

    update_led_scroll_bar(
        old_led_scroll_index,
        new_led_scroll_index,
        scroll_down_amount,
        scroll_up_amount,
    )
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
        BACKLIGHT_CH.send(BacklightCommand::Toggle).await;

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

const LARGE_DISPLAY_BLACK: u16 = 0x0000;
const LARGE_DISPLAY_DIM_GRAY: u16 = 0x39e7;
const RIGHT_ROTARY_DISPLAY_COLORS: [u16; 6] = [
    0xf800, // red
    0xfd20, // orange
    0xffe0, // yellow
    0x07e0, // green
    0x001f, // blue
    0xffff, // white
];
const RIGHT_ROTARY_SNAKE_COLUMNS: usize = 8;
const RIGHT_ROTARY_SNAKE_ROWS: usize = 10;
const RIGHT_ROTARY_SNAKE_LEN: usize =
    RIGHT_ROTARY_SNAKE_COLUMNS * RIGHT_ROTARY_SNAKE_ROWS;
const RIGHT_ROTARY_SNAKE_ORIGIN_X: u16 = 1;
const RIGHT_ROTARY_SNAKE_ORIGIN_Y: u16 = 1;
const RIGHT_ROTARY_SNAKE_CELL_STRIDE_X: u16 = 30;
const RIGHT_ROTARY_SNAKE_CELL_STRIDE_Y: u16 = 32;
const RIGHT_ROTARY_SNAKE_SQUARE_SIZE: u16 = 28;

async fn draw_right_rotary_display_press() {
    let index = (RIGHT_ROTARY_DISPLAY_INDEX.load(SeqCst) + 1)
        % RIGHT_ROTARY_DISPLAY_COLORS.len();

    RIGHT_ROTARY_DISPLAY_INDEX.store(index, SeqCst);
    let snake_index = RIGHT_ROTARY_SNAKE_INDEX.load(SeqCst);

    if RIGHT_ROTARY_DISPLAY_INITIALIZED.swap(true, SeqCst) {
        draw_right_rotary_display_square(
            snake_index,
            RIGHT_ROTARY_DISPLAY_COLORS[index],
        )
        .await;
    } else {
        draw_right_rotary_display(snake_index, index).await;
    }
}

async fn move_right_rotary_display_square(cw: u16, ccw: u16) {
    if cw == 0 && ccw == 0 {
        return;
    }

    if !RIGHT_ROTARY_DISPLAY_INITIALIZED.swap(true, SeqCst) {
        draw_right_rotary_display(
            RIGHT_ROTARY_SNAKE_INDEX.load(SeqCst),
            current_right_rotary_display_color_index(),
        )
        .await;
    }

    let forward_steps = ccw.saturating_sub(cw);
    let backward_steps = cw.saturating_sub(ccw);

    for _ in 0..forward_steps {
        move_right_rotary_display_square_one_step(1).await;
    }

    for _ in 0..backward_steps {
        move_right_rotary_display_square_one_step(-1).await;
    }
}

async fn move_right_rotary_display_square_one_step(direction: i16) {
    let old_index = RIGHT_ROTARY_SNAKE_INDEX.load(SeqCst);
    let new_index = (old_index as i16 + direction)
        .rem_euclid(RIGHT_ROTARY_SNAKE_LEN as i16)
        as usize;

    RIGHT_ROTARY_SNAKE_INDEX.store(new_index, SeqCst);

    redraw_right_rotary_display_square(
        old_index,
        new_index,
        RIGHT_ROTARY_DISPLAY_COLORS
            [current_right_rotary_display_color_index()],
    )
    .await;
}

async fn draw_right_rotary_display(
    snake_index: usize,
    color_index: usize,
) {
    let color = RIGHT_ROTARY_DISPLAY_COLORS[color_index];
    let (snake_x, snake_y) = right_rotary_snake_position(snake_index);

    LARGE_DISPLAY_CH
        .send(LargeDisplayCommand::Clear(LARGE_DISPLAY_BLACK))
        .await;

    draw_right_rotary_snake_track().await;

    LARGE_DISPLAY_CH
        .send(LargeDisplayCommand::FillRect {
            x: snake_x,
            y: snake_y,
            width: RIGHT_ROTARY_SNAKE_SQUARE_SIZE,
            height: RIGHT_ROTARY_SNAKE_SQUARE_SIZE,
            color,
        })
        .await;
}

async fn redraw_right_rotary_display_square(
    old_index: usize,
    new_index: usize,
    color: u16,
) {
    let (old_x, old_y) = right_rotary_snake_position(old_index);
    let (new_x, new_y) = right_rotary_snake_position(new_index);

    LARGE_DISPLAY_CH
        .send(LargeDisplayCommand::FillRect {
            x: old_x,
            y: old_y,
            width: RIGHT_ROTARY_SNAKE_SQUARE_SIZE,
            height: RIGHT_ROTARY_SNAKE_SQUARE_SIZE,
            color: LARGE_DISPLAY_DIM_GRAY,
        })
        .await;

    LARGE_DISPLAY_CH
        .send(LargeDisplayCommand::FillRect {
            x: new_x,
            y: new_y,
            width: RIGHT_ROTARY_SNAKE_SQUARE_SIZE,
            height: RIGHT_ROTARY_SNAKE_SQUARE_SIZE,
            color,
        })
        .await;
}

async fn draw_right_rotary_display_square(index: usize, color: u16) {
    let (x, y) = right_rotary_snake_position(index);

    LARGE_DISPLAY_CH
        .send(LargeDisplayCommand::FillRect {
            x,
            y,
            width: RIGHT_ROTARY_SNAKE_SQUARE_SIZE,
            height: RIGHT_ROTARY_SNAKE_SQUARE_SIZE,
            color,
        })
        .await;
}

async fn draw_right_rotary_snake_track() {
    for index in 0..RIGHT_ROTARY_SNAKE_LEN {
        let (x, y) = right_rotary_snake_position(index);

        LARGE_DISPLAY_CH
            .send(LargeDisplayCommand::FillRect {
                x,
                y,
                width: RIGHT_ROTARY_SNAKE_SQUARE_SIZE,
                height: RIGHT_ROTARY_SNAKE_SQUARE_SIZE,
                color: LARGE_DISPLAY_DIM_GRAY,
            })
            .await;
    }
}

fn right_rotary_snake_position(index: usize) -> (u16, u16) {
    let row = index / RIGHT_ROTARY_SNAKE_COLUMNS;
    let col = index % RIGHT_ROTARY_SNAKE_COLUMNS;
    let snaked_col = if row % 2 == 0 {
        col
    } else {
        RIGHT_ROTARY_SNAKE_COLUMNS - 1 - col
    };

    let x = RIGHT_ROTARY_SNAKE_ORIGIN_X
        + (snaked_col as u16 * RIGHT_ROTARY_SNAKE_CELL_STRIDE_X);
    let y = RIGHT_ROTARY_SNAKE_ORIGIN_Y
        + (row as u16 * RIGHT_ROTARY_SNAKE_CELL_STRIDE_Y);

    (x, y)
}

fn current_right_rotary_display_color_index() -> usize {
    RIGHT_ROTARY_DISPLAY_INDEX.load(SeqCst)
        % RIGHT_ROTARY_DISPLAY_COLORS.len()
}

async fn update_led_scroll_bar(
    old_bar_index: usize,
    new_bar_index: usize,
    scroll_down_amount: u16,
    scroll_up_amount: u16,
) {
    let delta = scroll_down_amount as i32 - scroll_up_amount as i32;

    if delta == 0 {
        return;
    }

    let step_count = match delta.unsigned_abs() as usize
        % LED_SCROLL_BAR_MAPPING.len()
    {
        0 => LED_SCROLL_BAR_MAPPING.len(),
        steps => steps,
    };

    let step_direction = if delta > 0 { 1 } else { -1 };
    let bar_len = LED_SCROLL_BAR_MAPPING.len() as isize;

    for step in 1..=step_count {
        let led_index = (old_bar_index as isize
            + (step as isize * step_direction))
            .rem_euclid(bar_len) as usize;
        let led = LED_SCROLL_BAR_MAPPING[led_index];

        LED_SHIFTER_CHANNEL
            .send(LedCommand::TemporarySetHigh(
                led,
                LED_SCROLL_BAR_TOGGLE_TIME,
            ))
            .await;
    }

    debug_assert_eq!(
        new_bar_index,
        (old_bar_index as isize
            + (step_count as isize * step_direction))
            .rem_euclid(bar_len) as usize
    );
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
