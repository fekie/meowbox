use core::fmt::Write;

use embassy_time::{Duration, Timer};
use heapless::String;
use rotary_encoder_embedded::Direction;

use super::{MenuState, Meowbox, Stage, State, SynthState};
use crate::{
    hardware::{
        large_display::{
            BACKLIGHT_CH, BacklightCommand, LARGE_DISPLAY_CH,
            LargeDisplayCommand,
        },
        led_shifter::{LED, LED_SHIFTER_CHANNEL, LedCommand},
        mono_display::{
            MONO_DISPLAY_CH, MONO_DISPLAY_LINE_WIDTH,
            MonoDisplayCommand,
        },
        speaker::{
            SPEAKER_CHANNEL, SpeakerCommand, Waveform,
            waveform_sample,
        },
    },
    input_listener::{Input, InputListener},
};

const BLACK: u16 = 0x0000;
const DIM_GRAY: u16 = 0x4208;
const WAVEFORM_COLORS: [u16; Waveform::COUNT] =
    [0x07e0, 0xf800, 0x001f, 0xffe0];
const FREQUENCY_MIN_HZ: u16 = 20;
const FREQUENCY_MAX_HZ: u16 = 5000;
const FREQUENCY_SCROLL_FACTOR: f32 = 1.029_302_2;
const TONE_CHUNK: Duration = Duration::from_millis(30);
const TICK_TIME: Duration = Duration::from_millis(10);
const INPUT_LED_TIME: Duration = Duration::from_millis(100);
const DISPLAY_WIDTH: u16 = 240;
const DISPLAY_HEIGHT: u16 = 320;
const WAVEFORM_CENTER_X: i16 = 120;
const WAVEFORM_AMPLITUDE: f32 = 108.0;
const WAVEFORM_DOT_SIZE: u16 = 4;
const WAVEFORM_Y_STEP: u16 = 2;
const DISPLAY_MIN_WINDOW_SECONDS: f32 = 0.04 / 3.0;
const DISPLAY_MIN_CYCLES: f32 = 2.0 / 3.0;

impl Meowbox {
    pub(super) async fn tick_synth(&mut self) {
        let State::Synth(stage, synth_state) = self.state
        else {
            return;
        };

        match stage {
            Stage::Setup => self.setup_synth(synth_state).await,
            Stage::Execution => self.execute_synth(synth_state).await,
            Stage::Shutdown => self.shutdown_synth().await,
        }
    }

    async fn setup_synth(&mut self, mut synth_state: SynthState) {
        synth_state.playing = false;

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
            .send(LedCommand::SetHigh(LED::DpadLeft))
            .await;
        LED_SHIFTER_CHANNEL
            .send(LedCommand::SetHigh(LED::DpadRight))
            .await;

        MONO_DISPLAY_CH
            .send(MonoDisplayCommand::SwitchToTerminal)
            .await;
        MONO_DISPLAY_CH.send(MonoDisplayCommand::Init).await;
        MONO_DISPLAY_CH
            .send(MonoDisplayCommand::SetDisplayOn(true))
            .await;

        drain_synth_inputs();
        write_synth_label(synth_state).await;

        LARGE_DISPLAY_CH
            .send(LargeDisplayCommand::StopAnimation)
            .await;
        LARGE_DISPLAY_CH.send(LargeDisplayCommand::DisplayOn).await;
        BACKLIGHT_CH.send(BacklightCommand::SetHigh).await;
        draw_waveform(synth_state).await;

        self.state = State::Synth(Stage::Execution, synth_state);
    }

    async fn execute_synth(&mut self, mut synth_state: SynthState) {
        if take_input_count(Input::ButtonLeft) != 0 {
            self.next_state =
                Some(State::Menu(Stage::Setup, MenuState::default()));
            self.needs_to_shutdown = true;
            return;
        }

        if take_input_count(Input::ButtonRight) != 0 {
            synth_state.playing = true;
            LED_SHIFTER_CHANNEL
                .send(LedCommand::SetHigh(LED::ButtonRight))
                .await;
        }

        if take_input_count(Input::ButtonRightReleased) != 0 {
            synth_state.playing = false;
            SPEAKER_CHANNEL.send(SpeakerCommand::Silence).await;
        }

        let frequency_delta = take_frequency_delta();
        let waveform_delta = take_waveform_delta();

        if frequency_delta != 0 {
            synth_state.frequency_hz = adjust_frequency(
                synth_state.frequency_hz,
                frequency_delta,
            );
            write_synth_label(synth_state).await;
            draw_waveform(synth_state).await;
            flash_frequency_led(frequency_delta).await;
        }

        if waveform_delta != 0 {
            synth_state.waveform =
                shift_waveform(synth_state.waveform, waveform_delta);
            write_synth_label(synth_state).await;
            draw_waveform(synth_state).await;
            flash_waveform_led(waveform_delta).await;
        }

        if synth_state.playing {
            let _ = SPEAKER_CHANNEL.try_send(
                SpeakerCommand::PlayWaveform {
                    waveform: synth_state.waveform,
                    frequency_hz: synth_state.frequency_hz,
                    duration: TONE_CHUNK,
                },
            );
        }

        self.state = State::Synth(Stage::Execution, synth_state);
        Timer::after(TICK_TIME).await;
    }

    async fn shutdown_synth(&mut self) {
        let _ = SPEAKER_CHANNEL.try_send(SpeakerCommand::Silence);
        LARGE_DISPLAY_CH
            .send(LargeDisplayCommand::StopAnimation)
            .await;
        LED_SHIFTER_CHANNEL.send(LedCommand::SetAllLow).await;
        BACKLIGHT_CH.send(BacklightCommand::SetLow).await;

        self.state = self.next_state.take().unwrap_or(State::Menu(
            Stage::Setup,
            MenuState::default(),
        ));
    }
}

fn take_frequency_delta() -> i16 {
    let cw = take_input_count(Input::RotaryEncoderRotateLeft(
        Direction::Clockwise,
    ));
    let ccw = take_input_count(Input::RotaryEncoderRotateLeft(
        Direction::Anticlockwise,
    ));

    cw as i16 - ccw as i16
}

fn take_waveform_delta() -> i16 {
    let right = take_input_count(Input::DpadRight);
    let left = take_input_count(Input::DpadLeft);

    right as i16 - left as i16
}

fn adjust_frequency(frequency_hz: u16, delta: i16) -> u16 {
    let mut next = frequency_hz as f32;

    for _ in 0..delta.unsigned_abs() {
        if delta > 0 {
            next *= FREQUENCY_SCROLL_FACTOR;
        } else {
            next /= FREQUENCY_SCROLL_FACTOR;
        }
    }

    ((next + 0.5) as i32)
        .clamp(FREQUENCY_MIN_HZ as i32, FREQUENCY_MAX_HZ as i32)
        as u16
}

fn shift_waveform(waveform: Waveform, delta: i16) -> Waveform {
    let next = (waveform.index() as i16 + delta)
        .rem_euclid(Waveform::COUNT as i16) as usize;

    Waveform::from_index(next)
}

fn drain_synth_inputs() {
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
    let _ = take_input_count(Input::ButtonRightReleased);
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

async fn write_synth_label(synth_state: SynthState) {
    let mut frequency: String<MONO_DISPLAY_LINE_WIDTH> =
        String::new();
    let mut waveform: String<MONO_DISPLAY_LINE_WIDTH> = String::new();

    write!(frequency, "{} Hz", synth_state.frequency_hz).unwrap();
    write!(waveform, "\n{}", synth_state.waveform.name())
        .unwrap();

    MONO_DISPLAY_CH.send(MonoDisplayCommand::Clear).await;
    MONO_DISPLAY_CH
        .send(MonoDisplayCommand::WriteStr(frequency))
        .await;
    MONO_DISPLAY_CH
        .send(MonoDisplayCommand::WriteStr(waveform))
        .await;
}

async fn draw_waveform(synth_state: SynthState) {
    let waveform = synth_state.waveform;
    let color = WAVEFORM_COLORS[waveform.index()];
    let window_seconds = display_window_seconds(synth_state.frequency_hz);
    let mut previous: Option<(u16, u16)> = None;

    LARGE_DISPLAY_CH
        .send(LargeDisplayCommand::Clear(BLACK))
        .await;
    LARGE_DISPLAY_CH
        .send(LargeDisplayCommand::FillRect {
            x: (WAVEFORM_CENTER_X - 1) as u16,
            y: 0,
            width: 2,
            height: DISPLAY_HEIGHT,
            color: DIM_GRAY,
        })
        .await;

    for y in (0..DISPLAY_HEIGHT).step_by(WAVEFORM_Y_STEP as usize) {
        let time =
            y as f32 / (DISPLAY_HEIGHT - 1) as f32 * window_seconds;
        let phase = time * synth_state.frequency_hz as f32;
        let sample = waveform_sample(waveform, phase);
        let x = WAVEFORM_CENTER_X
            + (sample * WAVEFORM_AMPLITUDE) as i16
            - WAVEFORM_DOT_SIZE as i16 / 2;
        let x = x
            .clamp(0, DISPLAY_WIDTH as i16 - WAVEFORM_DOT_SIZE as i16)
            as u16;

        if let Some((previous_x, previous_y)) = previous {
            draw_connected_segment(
                previous_x, previous_y, x, y, color,
            )
            .await;
        } else {
            draw_waveform_dot(x, y, color).await;
        }

        previous = Some((x, y));
    }
}

fn display_window_seconds(frequency_hz: u16) -> f32 {
    let two_cycle_window = DISPLAY_MIN_CYCLES / frequency_hz as f32;

    DISPLAY_MIN_WINDOW_SECONDS.max(two_cycle_window)
}

async fn draw_connected_segment(
    start_x: u16,
    start_y: u16,
    end_x: u16,
    end_y: u16,
    color: u16,
) {
    let x = start_x.min(end_x);
    let y = start_y.min(end_y);
    let width =
        start_x.abs_diff(end_x).saturating_add(WAVEFORM_DOT_SIZE);
    let height =
        start_y.abs_diff(end_y).saturating_add(WAVEFORM_DOT_SIZE);

    LARGE_DISPLAY_CH
        .send(LargeDisplayCommand::FillRect {
            x,
            y,
            width: width.min(DISPLAY_WIDTH - x),
            height: height.min(DISPLAY_HEIGHT - y),
            color,
        })
        .await;
}

async fn draw_waveform_dot(x: u16, y: u16, color: u16) {
    LARGE_DISPLAY_CH
        .send(LargeDisplayCommand::FillRect {
            x,
            y,
            width: WAVEFORM_DOT_SIZE,
            height: WAVEFORM_DOT_SIZE,
            color,
        })
        .await;
}

async fn flash_frequency_led(delta: i16) {
    let led = if delta > 0 {
        LED::AmberLeft
    } else {
        LED::ButtonLeft
    };

    LED_SHIFTER_CHANNEL
        .send(LedCommand::TemporarySetHigh(led, INPUT_LED_TIME))
        .await;
}

async fn flash_waveform_led(delta: i16) {
    LED_SHIFTER_CHANNEL
        .send(LedCommand::SetHigh(if delta > 0 {
            LED::DpadRight
        } else {
            LED::DpadLeft
        }))
        .await;
    LED_SHIFTER_CHANNEL
        .send(LedCommand::SetHigh(LED::DpadLeft))
        .await;
    LED_SHIFTER_CHANNEL
        .send(LedCommand::SetHigh(LED::DpadRight))
        .await;
}
