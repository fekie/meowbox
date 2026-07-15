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
const AUTOMATA_CELLS: usize = AUTOMATA_COLUMNS * AUTOMATA_ROWS;
const KERNEL_STEPS_PER_TICK: usize = 2;
const MAX_KERNEL_SIZE: usize = 5;
const CONVOLUTION_LIGHTNESS: u16 = 1;
const INPUT_LED_TIME: Duration = Duration::from_millis(100);
const KERNEL_HIGHLIGHT: u16 = 0xf800;

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
        mut automata_state: AutomataState,
    ) {
        automata_state.kernel_index = 0;
        automata_state.kernel_running = false;
        automata_state.kernel_pass = 0;

        LED_SHIFTER_CHANNEL.send(LedCommand::SetAllLow).await;
        LED_SHIFTER_CHANNEL
            .send(LedCommand::SetHigh(LED::ButtonLeft))
            .await;
        LED_SHIFTER_CHANNEL
            .send(LedCommand::SetHigh(LED::ButtonRight))
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
        drain_automata_inputs();
        write_rule_label(automata_state.rule).await;

        LARGE_DISPLAY_CH.send(LargeDisplayCommand::DisplayOn).await;
        BACKLIGHT_CH.send(BacklightCommand::SetHigh).await;
        draw_rule(automata_state).await;

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

        let right_button_pressed =
            InputListener::take_input(Input::ButtonRight, true)
                .ok()
                .flatten()
                .is_some();
        let _ = InputListener::take_input(
            Input::RotaryEncoderPressRight,
            true,
        );

        drain_dpad_inputs();

        let rule_delta = take_rule_delta();
        let palette_delta = take_palette_delta();

        if rule_delta != 0 {
            automata_state.rule =
                (automata_state.rule as i16 + rule_delta)
                    .rem_euclid(256) as u8;
            automata_state.kernel_index = 0;
            automata_state.kernel_running = false;
            automata_state.kernel_pass = 0;
        }

        if palette_delta != 0 {
            automata_state.palette_index = automata_state
                .palette_index
                .saturating_add(palette_delta as i32);
        }

        if rule_delta != 0 || palette_delta != 0 {
            write_rule_label(automata_state.rule).await;
            draw_rule(automata_state).await;
            draw_convolution_state(automata_state).await;
            flash_input_led(rule_delta, palette_delta).await;

            let click_count = rule_delta.unsigned_abs()
                + palette_delta.unsigned_abs();
            for _ in 0..click_count {
                BUZZER_2K3_CH.send(BuzzerCommand::Click).await;
            }
        }

        if right_button_pressed {
            if !automata_state.kernel_running {
                automata_state.kernel_running = true;
            }
            flash_right_button().await;
            BUZZER_2K3_CH.send(BuzzerCommand::Click).await;
        }

        if automata_state.kernel_running {
            tick_blur_kernel(&mut automata_state).await;
        }

        self.state =
            State::Automata(Stage::Execution, automata_state);

        Timer::after(Duration::from_millis(10)).await;
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

fn drain_automata_inputs() {
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
    let _ = take_input_count(Input::ButtonRight);
    let _ = take_input_count(Input::RotaryEncoderPressRight);
    drain_dpad_inputs();
}

fn drain_dpad_inputs() {
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

async fn flash_input_led(rule_delta: i16, palette_delta: i16) {
    if rule_delta != 0 {
        let led = if rule_delta > 0 {
            LED::AmberLeft
        } else {
            LED::ButtonLeft
        };

        LED_SHIFTER_CHANNEL
            .send(LedCommand::TemporaryToggle(led, INPUT_LED_TIME))
            .await;
    }

    if palette_delta != 0 {
        let led = if palette_delta > 0 {
            LED::AmberRight
        } else {
            LED::ButtonRight
        };

        LED_SHIFTER_CHANNEL
            .send(LedCommand::TemporaryToggle(led, INPUT_LED_TIME))
            .await;
    }
}

async fn flash_right_button() {
    LED_SHIFTER_CHANNEL
        .send(LedCommand::TemporaryToggle(
            LED::ButtonRight,
            INPUT_LED_TIME,
        ))
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

async fn draw_rule(automata_state: AutomataState) {
    let mut previous = [false; AUTOMATA_COLUMNS];
    let mut current = [false; AUTOMATA_COLUMNS];
    let (background, foreground) =
        color_scheme(automata_state.palette_index);

    previous[AUTOMATA_COLUMNS / 2] = true;

    LARGE_DISPLAY_CH
        .send(LargeDisplayCommand::Clear(background))
        .await;
    draw_binary_row(0, &previous, foreground).await;

    for row in 1..AUTOMATA_ROWS {
        for column in 0..AUTOMATA_COLUMNS {
            let left = column
                .checked_sub(1)
                .map(|index| previous[index])
                .unwrap_or(false);
            let center = previous[column];
            let right =
                previous.get(column + 1).copied().unwrap_or(false);

            current[column] =
                next_cell(automata_state.rule, left, center, right);
        }

        draw_binary_row(row, &current, foreground).await;
        previous = current;
        current = [false; AUTOMATA_COLUMNS];
    }
}

async fn draw_binary_row(
    row: usize,
    cells: &[bool; AUTOMATA_COLUMNS],
    color: u16,
) {
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
        draw_cell_run(start, row, run_len, color).await;
    }
}

async fn tick_blur_kernel(automata_state: &mut AutomataState) {
    for _ in 0..KERNEL_STEPS_PER_TICK {
        let old_kernel_size =
            kernel_size_for_pass(automata_state.kernel_pass);

        draw_kernel(
            automata_state.kernel_index,
            *automata_state,
            old_kernel_size,
            false,
        )
        .await;

        advance_kernel_position(automata_state);
        let new_kernel_size =
            kernel_size_for_pass(automata_state.kernel_pass);

        draw_kernel(
            automata_state.kernel_index,
            *automata_state,
            new_kernel_size,
            true,
        )
        .await;
    }
}

async fn draw_convolution_state(automata_state: AutomataState) {
    if !automata_state.kernel_running {
        return;
    }

    for completed_pass in 0..automata_state.kernel_pass {
        let completed_kernel_size =
            kernel_size_for_pass(completed_pass);
        draw_full_kernel_pass(automata_state, completed_kernel_size)
            .await;
    }

    let kernel_size =
        kernel_size_for_pass(automata_state.kernel_pass);

    for center_index in 0..automata_state.kernel_index {
        draw_kernel(center_index, automata_state, kernel_size, false)
            .await;
    }

    draw_kernel(
        automata_state.kernel_index,
        automata_state,
        kernel_size,
        true,
    )
    .await;
}

async fn draw_full_kernel_pass(
    automata_state: AutomataState,
    kernel_size: usize,
) {
    for center_index in 0..AUTOMATA_CELLS {
        draw_kernel(center_index, automata_state, kernel_size, false)
            .await;
    }
}

fn advance_kernel_position(automata_state: &mut AutomataState) {
    automata_state.kernel_index += 1;

    if automata_state.kernel_index >= AUTOMATA_CELLS {
        automata_state.kernel_index = 0;
        automata_state.kernel_pass =
            automata_state.kernel_pass.saturating_add(1);
    }
}

fn kernel_size_for_pass(pass: usize) -> usize {
    match pass {
        0 => 5,
        1 => 4,
        _ => 3,
    }
}

async fn draw_kernel(
    center_index: usize,
    automata_state: AutomataState,
    kernel_size: usize,
    highlighted: bool,
) {
    let center_column = center_index / AUTOMATA_ROWS;
    let center_row = center_index % AUTOMATA_ROWS;
    let first_delta = kernel_first_delta(kernel_size);

    for row_offset in 0..kernel_size {
        for column_offset in 0..kernel_size {
            let row_delta = first_delta + row_offset as i16;
            let column_delta = first_delta + column_offset as i16;
            let Some(column) = offset_index(
                center_column,
                column_delta,
                AUTOMATA_COLUMNS,
            ) else {
                continue;
            };
            let Some(row) =
                offset_index(center_row, row_delta, AUTOMATA_ROWS)
            else {
                continue;
            };

            draw_blurred_cell(
                column,
                row,
                automata_state,
                kernel_size,
                highlighted,
            )
            .await;
        }
    }
}

async fn draw_blurred_cell(
    column: usize,
    row: usize,
    automata_state: AutomataState,
    kernel_size: usize,
    highlighted: bool,
) {
    let (background, foreground) =
        color_scheme(automata_state.palette_index);
    let (intensity, denominator) = blurred_intensity(
        automata_state.rule,
        column,
        row,
        kernel_size,
    );
    let mut color = mix_rgb565(
        background,
        foreground,
        intensity,
        denominator.saturating_mul(CONVOLUTION_LIGHTNESS),
    );

    if highlighted {
        color = mix_rgb565(color, KERNEL_HIGHLIGHT, 7, 16);
    }

    draw_cell_run(column, row, 1, color).await;
}

async fn draw_cell_run(
    column: usize,
    row: usize,
    run_len: usize,
    color: u16,
) {
    LARGE_DISPLAY_CH
        .send(LargeDisplayCommand::FillRect {
            x: column as u16 * CELL_SIZE,
            y: row as u16 * CELL_SIZE,
            width: run_len as u16 * CELL_SIZE,
            height: CELL_SIZE,
            color,
        })
        .await;
}

fn blurred_intensity(
    rule: u8,
    column: usize,
    row: usize,
    kernel_size: usize,
) -> (u16, u16) {
    let rows = source_rows_for_blur(rule, row, kernel_size);
    let mut total = 0;
    let denominator = kernel_denominator(kernel_size);
    let center_row = kernel_size / 2;

    for row_index in 0..kernel_size {
        for column_index in 0..kernel_size {
            let column_delta =
                kernel_first_delta(kernel_size) + column_index as i16;

            let Some(source_column) =
                offset_index(column, column_delta, AUTOMATA_COLUMNS)
            else {
                continue;
            };

            if rows[row_index][source_column] {
                if row_index == center_row && source_column == column {
                    return (denominator, denominator);
                }

                total += kernel_weight(kernel_size, row_index)
                    * kernel_weight(kernel_size, column_index);
            }
        }
    }

    (total, denominator)
}

fn source_rows_for_blur(
    rule: u8,
    target_row: usize,
    kernel_size: usize,
) -> [[bool; AUTOMATA_COLUMNS]; MAX_KERNEL_SIZE] {
    let mut rows = [[false; AUTOMATA_COLUMNS]; MAX_KERNEL_SIZE];
    let mut previous = [false; AUTOMATA_COLUMNS];
    let mut current = [false; AUTOMATA_COLUMNS];

    previous[AUTOMATA_COLUMNS / 2] = true;
    store_blur_row(&mut rows, target_row, 0, &previous, kernel_size);

    let last_delta =
        kernel_first_delta(kernel_size) + kernel_size as i16 - 1;
    let last_row = if last_delta < 0 {
        target_row
    } else {
        target_row
            .saturating_add(last_delta as usize)
            .min(AUTOMATA_ROWS - 1)
    };

    for current_row in 1..=last_row {
        for current_column in 0..AUTOMATA_COLUMNS {
            let left = current_column
                .checked_sub(1)
                .map(|index| previous[index])
                .unwrap_or(false);
            let center = previous[current_column];
            let right = previous
                .get(current_column + 1)
                .copied()
                .unwrap_or(false);

            current[current_column] =
                next_cell(rule, left, center, right);
        }

        store_blur_row(
            &mut rows,
            target_row,
            current_row,
            &current,
            kernel_size,
        );
        previous = current;
        current = [false; AUTOMATA_COLUMNS];
    }

    rows
}

fn store_blur_row(
    rows: &mut [[bool; AUTOMATA_COLUMNS]; MAX_KERNEL_SIZE],
    target_row: usize,
    source_row: usize,
    cells: &[bool; AUTOMATA_COLUMNS],
    kernel_size: usize,
) {
    let row_delta = source_row as isize - target_row as isize;
    let first_delta = kernel_first_delta(kernel_size) as isize;
    let last_delta = first_delta + kernel_size as isize - 1;

    if !(first_delta..=last_delta).contains(&row_delta) {
        return;
    }

    rows[(row_delta - first_delta) as usize] = *cells;
}

fn kernel_first_delta(kernel_size: usize) -> i16 {
    -(kernel_size as i16 / 2)
}

fn kernel_weight(kernel_size: usize, index: usize) -> u16 {
    match kernel_size {
        5 => [1, 4, 6, 4, 1][index],
        4 => [1, 3, 3, 1][index],
        _ => [1, 6, 1][index],
    }
}

fn kernel_denominator(kernel_size: usize) -> u16 {
    match kernel_size {
        5 => 128,
        4 => 32,
        _ => 32,
    }
}

fn offset_index(
    index: usize,
    delta: i16,
    limit: usize,
) -> Option<usize> {
    if delta < 0 {
        index.checked_sub(delta.unsigned_abs() as usize)
    } else {
        let index = index.checked_add(delta as usize)?;
        (index < limit).then_some(index)
    }
}

fn color_scheme(index: i32) -> (u16, u16) {
    if index == 0 {
        return (WHITE, BLACK);
    }

    let seed = index as u32;
    let background = random_rgb565(seed ^ 0x4d45_4f57);
    let mut foreground = random_rgb565(seed ^ 0x4341_5453);

    if foreground == background {
        foreground ^= 0xffff;
    }

    (background, foreground)
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

fn mix_rgb565(
    start: u16,
    end: u16,
    numerator: u16,
    denominator: u16,
) -> u16 {
    let numerator = numerator.min(denominator) as u32;
    let denominator = denominator as u32;

    let start_red = ((start >> 11) & 0x1f) as u32;
    let start_green = ((start >> 5) & 0x3f) as u32;
    let start_blue = (start & 0x1f) as u32;

    let end_red = ((end >> 11) & 0x1f) as u32;
    let end_green = ((end >> 5) & 0x3f) as u32;
    let end_blue = (end & 0x1f) as u32;

    let red =
        weighted_channel(start_red, end_red, numerator, denominator);
    let green = weighted_channel(
        start_green,
        end_green,
        numerator,
        denominator,
    );
    let blue = weighted_channel(
        start_blue,
        end_blue,
        numerator,
        denominator,
    );

    (red << 11) | (green << 5) | blue
}

fn weighted_channel(
    start: u32,
    end: u32,
    numerator: u32,
    denominator: u32,
) -> u16 {
    let inverse = denominator - numerator;
    ((start * inverse + end * numerator) / denominator) as u16
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
