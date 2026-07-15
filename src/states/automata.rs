use core::fmt::Write;

use embassy_time::{Duration, Timer};
use heapless::String;
use rotary_encoder_embedded::Direction;

use super::{AutomataState, MenuState, Meowbox, Stage, State};
use crate::{
    hardware::{
        buzzer::{BUZZER_2K3_CH, BuzzerCommand},
        large_display::{
            BACKLIGHT_CH, BacklightCommand, LARGE_DISPLAY_CH,
            LargeDisplayCommand,
        },
        led_shifter::{LED, LED_SHIFTER_CHANNEL, LedCommand},
        mono_display::{
            MONO_DISPLAY_CH, MONO_DISPLAY_LINE_WIDTH,
            MonoDisplayCommand,
        },
    },
    input_listener::{Input, InputListener},
};

const BLACK: u16 = 0x0000;
const WHITE: u16 = 0xffff;
const CELL_SIZE: u16 = 4;
const AUTOMATA_COLUMNS: usize = 60;
const AUTOMATA_ROWS: usize = 80;
const INPUT_LED_TIME: Duration = Duration::from_millis(100);

impl Meowbox {
    pub(super) async fn tick_automata(&mut self) {
        let State::Automata(stage, automata_state) = self.state
        else {
            return;
        };

        match stage {
            Stage::Setup => self.setup_automata(automata_state).await,
            Stage::Execution => {
                self.execute_automata(automata_state).await
            }
            Stage::Shutdown => self.shutdown_automata().await,
        }
    }

    async fn setup_automata(
        &mut self,
        automata_state: AutomataState,
    ) {
        LED_SHIFTER_CHANNEL.send(LedCommand::SetAllLow).await;
        LED_SHIFTER_CHANNEL
            .send(LedCommand::SetHigh(LED::ButtonLeft))
            .await;
        LED_SHIFTER_CHANNEL
            .send(LedCommand::SetHigh(LED::DpadLeft))
            .await;
        LED_SHIFTER_CHANNEL
            .send(LedCommand::SetHigh(LED::DpadRight))
            .await;
        LED_SHIFTER_CHANNEL
            .send(LedCommand::SetHigh(LED::AmberLeft))
            .await;
        LED_SHIFTER_CHANNEL
            .send(LedCommand::SetHigh(LED::AmberRight))
            .await;

        MONO_DISPLAY_CH
            .send(MonoDisplayCommand::SwitchToTerminal)
            .await;
        MONO_DISPLAY_CH.send(MonoDisplayCommand::Init).await;
        MONO_DISPLAY_CH
            .send(MonoDisplayCommand::SetDisplayOn(true))
            .await;
        write_rule_label(automata_state.rule).await;

        LARGE_DISPLAY_CH.send(LargeDisplayCommand::DisplayOn).await;
        BACKLIGHT_CH.send(BacklightCommand::SetHigh).await;
        draw_rule(automata_state.rule).await;

        self.state =
            State::Automata(Stage::Execution, automata_state);
    }

    async fn execute_automata(
        &mut self,
        mut automata_state: AutomataState,
    ) {
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

        let delta = take_rule_delta();

        if delta == 0 {
            Timer::after(Duration::from_millis(10)).await;
            return;
        }

        automata_state.rule = (automata_state.rule as i16 + delta)
            .rem_euclid(256) as u8;
        self.state =
            State::Automata(Stage::Execution, automata_state);

        write_rule_label(automata_state.rule).await;
        draw_rule(automata_state.rule).await;
        flash_input_led(delta).await;

        let click_count = delta.unsigned_abs();
        for _ in 0..click_count {
            BUZZER_2K3_CH.send(BuzzerCommand::Click).await;
        }
    }

    async fn shutdown_automata(&mut self) {
        LED_SHIFTER_CHANNEL.send(LedCommand::SetAllLow).await;
        BACKLIGHT_CH.send(BacklightCommand::SetLow).await;

        self.state = self.next_state.take().unwrap_or(State::Menu(
            Stage::Setup,
            MenuState::default(),
        ));
    }
}

fn take_rule_delta() -> i16 {
    let left_cw = take_input_count(Input::RotaryEncoderRotateLeft(
        Direction::Clockwise,
    ));
    let left_ccw = take_input_count(Input::RotaryEncoderRotateLeft(
        Direction::Anticlockwise,
    ));
    let right_cw = take_input_count(Input::RotaryEncoderRotateRight(
        Direction::Clockwise,
    ));
    let right_ccw = take_input_count(
        Input::RotaryEncoderRotateRight(Direction::Anticlockwise),
    );
    let dpad_left = take_input_count(Input::DpadLeft);
    let dpad_right = take_input_count(Input::DpadRight);

    (left_cw + right_cw + dpad_right) as i16
        - (left_ccw + right_ccw + dpad_left) as i16
}

fn take_input_count(input: Input) -> u16 {
    InputListener::take_input(input, true)
        .ok()
        .flatten()
        .unwrap_or_default()
}

async fn flash_input_led(delta: i16) {
    let led = if delta > 0 {
        LED::DpadRight
    } else {
        LED::DpadLeft
    };

    LED_SHIFTER_CHANNEL
        .send(LedCommand::TemporaryToggle(led, INPUT_LED_TIME))
        .await;
}

async fn write_rule_label(rule: u8) {
    let mut label: String<MONO_DISPLAY_LINE_WIDTH> = String::new();
    write!(label, "rule {}", rule).unwrap();

    MONO_DISPLAY_CH.send(MonoDisplayCommand::Clear).await;
    MONO_DISPLAY_CH
        .send(MonoDisplayCommand::WriteStr(label))
        .await;
}

async fn draw_rule(rule: u8) {
    let mut previous = [false; AUTOMATA_COLUMNS];
    let mut current = [false; AUTOMATA_COLUMNS];

    previous[AUTOMATA_COLUMNS / 2] = true;

    LARGE_DISPLAY_CH
        .send(LargeDisplayCommand::Clear(WHITE))
        .await;
    draw_row(0, &previous).await;

    for row in 1..AUTOMATA_ROWS {
        for column in 0..AUTOMATA_COLUMNS {
            let left = column
                .checked_sub(1)
                .map(|index| previous[index])
                .unwrap_or(false);
            let center = previous[column];
            let right =
                previous.get(column + 1).copied().unwrap_or(false);

            current[column] = next_cell(rule, left, center, right);
        }

        draw_row(row, &current).await;
        previous = current;
        current = [false; AUTOMATA_COLUMNS];
    }
}

async fn draw_row(row: usize, cells: &[bool; AUTOMATA_COLUMNS]) {
    let mut column = 0;

    while column < AUTOMATA_COLUMNS {
        if !cells[column] {
            column += 1;
            continue;
        }

        let start = column;
        while column < AUTOMATA_COLUMNS && cells[column] {
            column += 1;
        }

        let run_len = column - start;
        LARGE_DISPLAY_CH
            .send(LargeDisplayCommand::FillRect {
                x: start as u16 * CELL_SIZE,
                y: row as u16 * CELL_SIZE,
                width: run_len as u16 * CELL_SIZE,
                height: CELL_SIZE,
                color: BLACK,
            })
            .await;
    }
}

fn next_cell(
    rule: u8,
    left: bool,
    center: bool,
    right: bool,
) -> bool {
    let neighborhood =
        ((left as u8) << 2) | ((center as u8) << 1) | right as u8;

    ((rule >> neighborhood) & 1) != 0
}
