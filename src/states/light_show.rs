use embassy_time::{Duration, Timer};
use rotary_encoder_embedded::Direction;

use super::{
    LightShowDirection, LightShowMode, LightShowState, MenuState,
    Meowbox, Stage, State,
};
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

const RANDOM_LIGHT_SHOW_LEDS: [LED; 16] = [
    LED::Red,
    LED::Orange,
    LED::YellowCenter,
    LED::Green,
    LED::Blue,
    LED::White,
    LED::YellowLeft,
    LED::YellowRight,
    LED::AmberLeft,
    LED::AmberRight,
    LED::ButtonLeft,
    LED::ButtonRight,
    LED::DpadLeft,
    LED::DpadBottom,
    LED::DpadTop,
    LED::DpadRight,
];

const CLOCKWISE_RING_LEDS: [LED; 16] = [
    LED::Red,
    LED::Orange,
    LED::YellowCenter,
    LED::Green,
    LED::Blue,
    LED::White,
    LED::YellowRight,
    LED::AmberRight,
    LED::ButtonRight,
    LED::DpadRight,
    LED::DpadTop,
    LED::DpadBottom,
    LED::DpadLeft,
    LED::ButtonLeft,
    LED::AmberLeft,
    LED::YellowLeft,
];

const RANDOM_INTERVAL_STEP_MS: u64 = 25;
const RANDOM_INTERVAL_MIN_MS: u64 = 50;
const RANDOM_INTERVAL_MAX_MS: u64 = 2_000;
const RANDOM_LIGHT_COUNT_MIN: u8 = 1;
const RANDOM_LIGHT_COUNT_MAX: u8 = RANDOM_LIGHT_SHOW_LEDS.len() as u8;

const RING_STEP_CHANGE_MS: u64 = 1;
const RING_STEP_MIN_MS: u64 = 5;
const RING_STEP_MAX_MS: u64 = 250;

impl Meowbox {
    pub(super) async fn tick_light_show(&mut self) {
        let State::LightShow(stage, light_show_state) = self.state
        else {
            return;
        };

        match stage {
            Stage::Setup => {
                self.setup_light_show(light_show_state).await
            }
            Stage::Execution => {
                self.execute_light_show(light_show_state).await
            }
            Stage::Shutdown => self.shutdown_light_show().await,
        }
    }

    async fn setup_light_show(
        &mut self,
        light_show_state: LightShowState,
    ) {
        MONO_DISPLAY_CH.send(MonoDisplayCommand::Clear).await;
        MONO_DISPLAY_CH
            .send(MonoDisplayCommand::SetDisplayOn(false))
            .await;
        BACKLIGHT_CH.send(BacklightCommand::SetLow).await;
        LARGE_DISPLAY_CH.send(LargeDisplayCommand::DisplayOff).await;
        LED_SHIFTER_CHANNEL.send(LedCommand::SetAllLow).await;

        drain_light_show_inputs();

        self.state =
            State::LightShow(Stage::Execution, light_show_state);
    }

    async fn execute_light_show(
        &mut self,
        mut light_show_state: LightShowState,
    ) {
        if left_button_requested_main_menu() {
            self.next_state =
                Some(State::Menu(Stage::Setup, MenuState::default()));
            self.needs_to_shutdown = true;
            return;
        }

        handle_light_show_inputs(&mut light_show_state).await;

        match light_show_state.mode {
            LightShowMode::RandomBlink => {
                run_random_blink(&mut light_show_state).await;
            }
            LightShowMode::RingTrail => {
                run_ring_trail(&mut light_show_state).await;
            }
        }

        self.state =
            State::LightShow(Stage::Execution, light_show_state);
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

async fn handle_light_show_inputs(
    light_show_state: &mut LightShowState,
) {
    drain_button_inputs();

    let previous_mode = take_total(Input::RotaryEncoderPressRight)
        + take_total(Input::DpadLeft)
        + take_total(Input::DpadTop);
    let next_mode = take_total(Input::RotaryEncoderPressLeft)
        + take_total(Input::DpadRight)
        + take_total(Input::DpadBottom);

    if previous_mode != 0 || next_mode != 0 {
        scroll_mode(light_show_state, next_mode, previous_mode);
        LED_SHIFTER_CHANNEL.send(LedCommand::SetAllLow).await;
    }

    match light_show_state.mode {
        LightShowMode::RandomBlink => {
            let speed_up = take_total(
                Input::RotaryEncoderRotateRight(Direction::Clockwise),
            );
            let slow_down = take_total(
                Input::RotaryEncoderRotateRight(Direction::Anticlockwise),
            );
            light_show_state.random_interval_ms = adjust_u64(
                light_show_state.random_interval_ms,
                slow_down,
                speed_up,
                RANDOM_INTERVAL_STEP_MS,
                RANDOM_INTERVAL_MIN_MS,
                RANDOM_INTERVAL_MAX_MS,
            );

            let more_lights = take_total(
                Input::RotaryEncoderRotateLeft(Direction::Clockwise),
            );
            let fewer_lights = take_total(
                Input::RotaryEncoderRotateLeft(Direction::Anticlockwise),
            );
            light_show_state.random_light_count = adjust_u8(
                light_show_state.random_light_count,
                more_lights,
                fewer_lights,
                RANDOM_LIGHT_COUNT_MIN,
                RANDOM_LIGHT_COUNT_MAX,
            );
        }
        LightShowMode::RingTrail => {
            let clockwise = take_total(
                Input::RotaryEncoderRotateLeft(Direction::Clockwise),
            );
            let counterclockwise = take_total(
                Input::RotaryEncoderRotateLeft(Direction::Anticlockwise),
            );

            if clockwise > counterclockwise {
                light_show_state.ring_direction =
                    LightShowDirection::Clockwise;
            } else if counterclockwise > clockwise {
                light_show_state.ring_direction =
                    LightShowDirection::Counterclockwise;
            }

            let speed_up = take_total(
                Input::RotaryEncoderRotateRight(Direction::Clockwise),
            );
            let slow_down = take_total(
                Input::RotaryEncoderRotateRight(Direction::Anticlockwise),
            );
            light_show_state.ring_step_ms = adjust_u64(
                light_show_state.ring_step_ms,
                slow_down,
                speed_up,
                RING_STEP_CHANGE_MS,
                RING_STEP_MIN_MS,
                RING_STEP_MAX_MS,
            );
        }
    }
}

async fn run_random_blink(light_show_state: &mut LightShowState) {
    let mut leds = RANDOM_LIGHT_SHOW_LEDS;
    shuffle_leds(&mut leds, &mut light_show_state.random_seed);

    LED_SHIFTER_CHANNEL.send(LedCommand::SetAllLow).await;

    for led in leds
        .iter()
        .take(light_show_state.random_light_count as usize)
    {
        LED_SHIFTER_CHANNEL.send(LedCommand::SetHigh(*led)).await;
    }

    Timer::after(Duration::from_millis(
        light_show_state.random_interval_ms,
    ))
    .await;
}

async fn run_ring_trail(light_show_state: &mut LightShowState) {
    let ring_duration_ms =
        light_show_state.ring_step_ms + trail_ms(light_show_state);
    let led = CLOCKWISE_RING_LEDS
        [light_show_state.ring_index % CLOCKWISE_RING_LEDS.len()];

    LED_SHIFTER_CHANNEL
        .send(LedCommand::TemporarySetHigh(
            led,
            Duration::from_millis(ring_duration_ms),
        ))
        .await;

    advance_ring(light_show_state);

    Timer::after(Duration::from_millis(
        light_show_state.ring_step_ms,
    ))
    .await;
}

fn drain_light_show_inputs() {
    let _ = take_total(Input::ButtonLeft);
    drain_button_inputs();

    let _ = take_total(Input::RotaryEncoderPressLeft);
    let _ = take_total(Input::RotaryEncoderPressRight);
    let _ = take_total(Input::RotaryEncoderRotateLeft(
        Direction::Clockwise,
    ));
    let _ = take_total(Input::RotaryEncoderRotateLeft(
        Direction::Anticlockwise,
    ));
    let _ = take_total(Input::RotaryEncoderRotateRight(
        Direction::Clockwise,
    ));
    let _ = take_total(Input::RotaryEncoderRotateRight(
        Direction::Anticlockwise,
    ));
    let _ = take_total(Input::DpadLeft);
    let _ = take_total(Input::DpadRight);
    let _ = take_total(Input::DpadTop);
    let _ = take_total(Input::DpadBottom);
}

fn drain_button_inputs() {
    let _ = take_total(Input::ButtonRight);
    let _ = take_total(Input::ButtonRightReleased);
}

fn left_button_requested_main_menu() -> bool {
    take_total(Input::ButtonLeft) != 0
}

fn take_total(input: Input) -> u16 {
    InputListener::take_input(input, true)
        .ok()
        .flatten()
        .unwrap_or(0)
}

fn scroll_mode(
    light_show_state: &mut LightShowState,
    next: u16,
    previous: u16,
) {
    if (next + previous) % 2 == 0 {
        return;
    }

    light_show_state.mode = match light_show_state.mode {
        LightShowMode::RandomBlink => LightShowMode::RingTrail,
        LightShowMode::RingTrail => LightShowMode::RandomBlink,
    };
}

fn adjust_u64(
    current: u64,
    increase: u16,
    decrease: u16,
    step: u64,
    min: u64,
    max: u64,
) -> u64 {
    let increase_amount = u64::from(increase) * step;
    let decrease_amount = u64::from(decrease) * step;

    current
        .saturating_add(increase_amount)
        .saturating_sub(decrease_amount)
        .clamp(min, max)
}

fn adjust_u8(
    current: u8,
    increase: u16,
    decrease: u16,
    min: u8,
    max: u8,
) -> u8 {
    let increased = current.saturating_add(increase as u8);
    increased.saturating_sub(decrease as u8).clamp(min, max)
}

fn shuffle_leds(leds: &mut [LED; 16], seed: &mut u16) {
    let mut index = 0;

    while index < leds.len() {
        *seed = lfsr_next(*seed);
        let remaining = leds.len() - index;
        let swap_index = index + (*seed as usize % remaining);
        leds.swap(index, swap_index);
        index += 1;
    }
}

fn lfsr_next(state: u16) -> u16 {
    let bit =
        (state ^ (state >> 2) ^ (state >> 3) ^ (state >> 5)) & 1;

    (state >> 1) | (bit << 15)
}

fn trail_ms(light_show_state: &LightShowState) -> u64 {
    (light_show_state.ring_step_ms / 2).max(1)
}

fn advance_ring(light_show_state: &mut LightShowState) {
    match light_show_state.ring_direction {
        LightShowDirection::Clockwise => {
            light_show_state.ring_index =
                (light_show_state.ring_index + 1)
                    % CLOCKWISE_RING_LEDS.len();
        }
        LightShowDirection::Counterclockwise => {
            light_show_state.ring_index =
                (light_show_state.ring_index + CLOCKWISE_RING_LEDS.len()
                    - 1)
                    % CLOCKWISE_RING_LEDS.len();
        }
    }
}
