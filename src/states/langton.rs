use core::fmt::Write;

use embassy_time::{Duration, Timer};
use heapless::String;
use rotary_encoder_embedded::Direction;

use super::{
    LangtonDirection, LangtonState, MenuState, Meowbox, Stage, State,
};
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
const LANGTON_COLUMNS: usize = 60;
const LANGTON_ROWS: usize = 80;
const LANGTON_STEPS_PER_TICK: usize = 8;
const LANGTON_RULE_COUNT: usize = 4;

#[derive(Clone, Copy)]
enum LangtonTurn {
    Left,
    Right,
}

#[derive(Clone, Copy)]
struct LangtonRule {
    name: &'static str,
    turns: [LangtonTurn; 4],
    len: u8,
}

const LANGTON_RULES: [LangtonRule; LANGTON_RULE_COUNT] = [
    LangtonRule {
        name: "RL",
        turns: [
            LangtonTurn::Right,
            LangtonTurn::Left,
            LangtonTurn::Left,
            LangtonTurn::Left,
        ],
        len: 2,
    },
    LangtonRule {
        name: "LR",
        turns: [
            LangtonTurn::Left,
            LangtonTurn::Right,
            LangtonTurn::Left,
            LangtonTurn::Left,
        ],
        len: 2,
    },
    LangtonRule {
        name: "RLLR",
        turns: [
            LangtonTurn::Right,
            LangtonTurn::Left,
            LangtonTurn::Left,
            LangtonTurn::Right,
        ],
        len: 4,
    },
    LangtonRule {
        name: "LRRR",
        turns: [
            LangtonTurn::Left,
            LangtonTurn::Right,
            LangtonTurn::Right,
            LangtonTurn::Right,
        ],
        len: 4,
    },
];

impl Meowbox {
    pub(super) async fn tick_langton(&mut self) {
        let State::Langton(stage, langton_state) = self.state else {
            return;
        };

        match stage {
            Stage::Setup => self.setup_langton(langton_state).await,
            Stage::Execution => {
                self.execute_langton(langton_state).await
            }
            Stage::Shutdown => self.shutdown_langton().await,
        }
    }

    async fn setup_langton(&mut self, langton_state: LangtonState) {
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
        MONO_DISPLAY_CH.send(MonoDisplayCommand::Init).await;
        MONO_DISPLAY_CH
            .send(MonoDisplayCommand::SetDisplayOn(true))
            .await;
        drain_langton_inputs();
        write_langton_label(langton_state).await;

        LARGE_DISPLAY_CH.send(LargeDisplayCommand::DisplayOn).await;
        BACKLIGHT_CH.send(BacklightCommand::SetHigh).await;
        let colors = langton_colors(langton_state.palette_index);
        LARGE_DISPLAY_CH
            .send(LargeDisplayCommand::Clear(colors.background))
            .await;
        draw_langton_cell(
            langton_state.x,
            langton_state.y,
            colors.ant,
        )
        .await;

        self.state = State::Langton(Stage::Execution, langton_state);
    }

    async fn execute_langton(
        &mut self,
        mut langton_state: LangtonState,
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

        let right_pressed =
            InputListener::take_input(Input::ButtonRight, true)
                .ok()
                .flatten()
                .is_some();
        let rule_delta = take_rule_delta();
        let palette_delta = take_palette_delta();

        if rule_delta != 0 {
            let rule_index = (langton_state.rule_index as i16
                + rule_delta)
                .rem_euclid(LANGTON_RULE_COUNT as i16)
                as usize;
            let palette_index = langton_state.palette_index;

            langton_state = LangtonState {
                rule_index,
                palette_index,
                ..LangtonState::default()
            };
            write_langton_label(langton_state).await;
            draw_langton_state(langton_state).await;

            for _ in 0..rule_delta.unsigned_abs() {
                BUZZER_2K3_CH.send(BuzzerCommand::Click).await;
            }
        }

        if palette_delta != 0 {
            langton_state.palette_index = langton_state
                .palette_index
                .saturating_add(palette_delta as i32);
            draw_langton_state(langton_state).await;

            for _ in 0..palette_delta.unsigned_abs() {
                BUZZER_2K3_CH.send(BuzzerCommand::Click).await;
            }
        }

        if right_pressed {
            langton_state = LangtonState {
                rule_index: langton_state.rule_index,
                palette_index: langton_state.palette_index,
                ..LangtonState::default()
            };
            draw_langton_state(langton_state).await;
            BUZZER_2K3_CH.send(BuzzerCommand::Click).await;
        } else {
            for _ in 0..LANGTON_STEPS_PER_TICK {
                step_langton(&mut langton_state).await;
            }
        }

        self.state = State::Langton(Stage::Execution, langton_state);
        Timer::after(Duration::from_millis(10)).await;
    }

    async fn shutdown_langton(&mut self) {
        LED_SHIFTER_CHANNEL.send(LedCommand::SetAllLow).await;
        BACKLIGHT_CH.send(BacklightCommand::SetLow).await;

        self.state = self.next_state.take().unwrap_or(State::Menu(
            Stage::Setup,
            MenuState::default(),
        ));
    }
}

async fn step_langton(langton_state: &mut LangtonState) {
    let current_state =
        cell_state(langton_state, langton_state.x, langton_state.y);
    let rule = current_rule(langton_state);
    let next_state = (current_state + 1) % rule.len;

    set_cell(
        langton_state,
        langton_state.x,
        langton_state.y,
        next_state,
    );

    langton_state.direction = match rule.turns[current_state as usize]
    {
        LangtonTurn::Left => turn_left(langton_state.direction),
        LangtonTurn::Right => turn_right(langton_state.direction),
    };

    let colors = langton_colors(langton_state.palette_index);
    draw_langton_cell(
        langton_state.x,
        langton_state.y,
        color_for_cell_state(colors, next_state),
    )
    .await;

    move_ant(langton_state);
    draw_langton_cell(langton_state.x, langton_state.y, colors.ant)
        .await;
}

async fn draw_langton_state(langton_state: LangtonState) {
    let colors = langton_colors(langton_state.palette_index);

    LARGE_DISPLAY_CH
        .send(LargeDisplayCommand::Clear(colors.background))
        .await;

    for y in 0..LANGTON_ROWS {
        for x in 0..LANGTON_COLUMNS {
            let state = cell_state(&langton_state, x, y);

            if state == 0 {
                continue;
            }

            draw_langton_cell(
                x,
                y,
                color_for_cell_state(colors, state),
            )
            .await;
        }
    }

    draw_langton_cell(langton_state.x, langton_state.y, colors.ant)
        .await;
}

fn cell_state(
    langton_state: &LangtonState,
    x: usize,
    y: usize,
) -> u8 {
    let index = y * LANGTON_COLUMNS + x;
    let word_index = index / u64::BITS as usize;
    let bit_index = index % u64::BITS as usize;
    let mask = 1_u64 << bit_index;
    let low = (langton_state.cells[word_index] & mask) != 0;
    let high = (langton_state.cells_high[word_index] & mask) != 0;

    low as u8 | ((high as u8) << 1)
}

fn set_cell(
    langton_state: &mut LangtonState,
    x: usize,
    y: usize,
    state: u8,
) {
    let index = y * LANGTON_COLUMNS + x;
    let word_index = index / u64::BITS as usize;
    let bit_index = index % u64::BITS as usize;
    let mask = 1_u64 << bit_index;

    if state & 1 != 0 {
        langton_state.cells[word_index] |= mask;
    } else {
        langton_state.cells[word_index] &= !mask;
    }

    if state & 2 != 0 {
        langton_state.cells_high[word_index] |= mask;
    } else {
        langton_state.cells_high[word_index] &= !mask;
    }
}

fn turn_right(direction: LangtonDirection) -> LangtonDirection {
    match direction {
        LangtonDirection::Up => LangtonDirection::Right,
        LangtonDirection::Right => LangtonDirection::Down,
        LangtonDirection::Down => LangtonDirection::Left,
        LangtonDirection::Left => LangtonDirection::Up,
    }
}

fn turn_left(direction: LangtonDirection) -> LangtonDirection {
    match direction {
        LangtonDirection::Up => LangtonDirection::Left,
        LangtonDirection::Left => LangtonDirection::Down,
        LangtonDirection::Down => LangtonDirection::Right,
        LangtonDirection::Right => LangtonDirection::Up,
    }
}

fn move_ant(langton_state: &mut LangtonState) {
    match langton_state.direction {
        LangtonDirection::Up => {
            langton_state.y =
                (langton_state.y + LANGTON_ROWS - 1) % LANGTON_ROWS;
        }
        LangtonDirection::Right => {
            langton_state.x = (langton_state.x + 1) % LANGTON_COLUMNS;
        }
        LangtonDirection::Down => {
            langton_state.y = (langton_state.y + 1) % LANGTON_ROWS;
        }
        LangtonDirection::Left => {
            langton_state.x = (langton_state.x + LANGTON_COLUMNS - 1)
                % LANGTON_COLUMNS;
        }
    }
}

async fn draw_langton_cell(x: usize, y: usize, color: u16) {
    LARGE_DISPLAY_CH
        .send(LargeDisplayCommand::FillRect {
            x: x as u16 * CELL_SIZE,
            y: y as u16 * CELL_SIZE,
            width: CELL_SIZE,
            height: CELL_SIZE,
            color,
        })
        .await;
}

async fn write_langton_label(langton_state: LangtonState) {
    let rule = current_rule(&langton_state);
    let mut label: String<MONO_DISPLAY_LINE_WIDTH> = String::new();
    write!(label, "L {}", rule.name).unwrap();

    MONO_DISPLAY_CH.send(MonoDisplayCommand::Clear).await;
    MONO_DISPLAY_CH
        .send(MonoDisplayCommand::WriteStr(label))
        .await;
}

fn take_rule_delta() -> i16 {
    let left_cw = take_input_count(Input::RotaryEncoderRotateLeft(
        Direction::Clockwise,
    ));
    let left_ccw = take_input_count(Input::RotaryEncoderRotateLeft(
        Direction::Anticlockwise,
    ));

    left_cw as i16 - left_ccw as i16
}

fn take_palette_delta() -> i16 {
    let right_cw = take_input_count(Input::RotaryEncoderRotateRight(
        Direction::Clockwise,
    ));
    let right_ccw = take_input_count(
        Input::RotaryEncoderRotateRight(Direction::Anticlockwise),
    );

    right_cw as i16 - right_ccw as i16
}

fn drain_langton_inputs() {
    let _ = take_input_count(Input::RotaryEncoderRotateLeft(
        Direction::Clockwise,
    ));
    let _ = take_input_count(Input::RotaryEncoderRotateLeft(
        Direction::Anticlockwise,
    ));
    let _ = take_input_count(Input::RotaryEncoderRotateRight(
        Direction::Clockwise,
    ));
    let _ = take_input_count(Input::RotaryEncoderRotateRight(
        Direction::Anticlockwise,
    ));
    let _ = take_input_count(Input::ButtonLeft);
    let _ = take_input_count(Input::ButtonRight);
    let _ = take_input_count(Input::DpadLeft);
    let _ = take_input_count(Input::DpadRight);
    let _ = take_input_count(Input::DpadTop);
    let _ = take_input_count(Input::DpadBottom);
}

fn take_input_count(input: Input) -> u16 {
    InputListener::take_input(input, true)
        .ok()
        .flatten()
        .unwrap_or_default()
}

fn current_rule(langton_state: &LangtonState) -> LangtonRule {
    LANGTON_RULES[langton_state.rule_index % LANGTON_RULE_COUNT]
}

#[derive(Clone, Copy)]
struct LangtonColors {
    background: u16,
    states: [u16; 4],
    ant: u16,
}

fn color_for_cell_state(colors: LangtonColors, state: u8) -> u16 {
    colors.states[state as usize]
}

fn langton_colors(index: i32) -> LangtonColors {
    if index == 0 {
        return LangtonColors {
            background: WHITE,
            states: [WHITE, BLACK, 0x001f, 0x07e0],
            ant: 0xf800,
        };
    }

    let seed = index as u32;
    let background = random_rgb565(seed ^ 0x4c41_4e47);
    let state_one = random_rgb565(seed ^ 0x544f_4e31);
    let state_two = random_rgb565(seed ^ 0x544f_4e32);
    let state_three = random_rgb565(seed ^ 0x544f_4e33);
    let ant = random_rgb565(seed ^ 0x414e_5421);

    LangtonColors {
        background,
        states: [background, state_one, state_two, state_three],
        ant,
    }
}

fn random_rgb565(seed: u32) -> u16 {
    let mut value = seed.wrapping_add(0x9e37_79b9);
    value = (value ^ (value >> 16)).wrapping_mul(0x85eb_ca6b);
    value = (value ^ (value >> 13)).wrapping_mul(0xc2b2_ae35);
    value ^= value >> 16;

    let red = ((value >> 27) & 0x1f) as u16;
    let green = ((value >> 18) & 0x3f) as u16;
    let blue = ((value >> 11) & 0x1f) as u16;

    (red << 11) | (green << 5) | blue
}
