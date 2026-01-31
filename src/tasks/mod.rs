use defmt::{error, info, warn};
use embassy_executor::task;
use embassy_time::{Duration, Timer};
use rotary_encoder_embedded::angular_velocity::AngularVelocityMode;
use rotary_encoder_embedded::quadrature::QuadratureTableMode;

use crate::hardware::{BLUE_LED, GREEN_LED, LED_ARRAY, LEDType, RED_LED, YELLOW_LED};

use super::hardware;

use embassy_sync::signal::Signal;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};

use heapless::{Vec, vec};

use embassy_time::Instant;
use esp_hal::gpio::Input;
use rotary_encoder_embedded::Direction;
use rotary_encoder_embedded::RotaryEncoder;
use rotary_encoder_embedded::standard::StandardMode;

pub static BUZZER_SIGNAL: Signal<CriticalSectionRawMutex, BuzzerSequence> = Signal::new();
pub static LED_ROTATION_SIGNAL: Signal<CriticalSectionRawMutex, LEDRotationParams> = Signal::new();

/// A value comes into the channel when
pub static RIGHT_CLOCKWISE_ROTATION: Channel<CriticalSectionRawMutex, (), 64> = Channel::new();
pub static RIGHT_COUNTER_CLOCKWISE_ROTATION: Channel<CriticalSectionRawMutex, (), 64> =
    Channel::new();

pub enum BuzzerSequence {
    SimpleTone200ms,
    Intermittent10ms200ms,
    Intermittent5ms2000ms,
}

#[task]
pub async fn left_button_event(
    button: &'static hardware::ButtonType,
    led: &'static hardware::ButtonLEDType,
    buzzer: &'static hardware::BuzzerType,
) {
    loop {
        button.lock().await.as_mut().unwrap().wait_for_low().await;
        led.lock().await.as_mut().unwrap().set_low();

        info!("left button triggered");

        // wait 200ms
        for _ in 0..20 {
            buzzer.lock().await.as_mut().unwrap().toggle();
            Timer::after(Duration::from_millis(10)).await;
        }

        //Timer::after(Duration::from_millis(200)).await;
        buzzer.lock().await.as_mut().unwrap().set_low();
        led.lock().await.as_mut().unwrap().set_high();
    }
}

#[task]
pub async fn right_button_event(
    button: &'static hardware::ButtonType,
    led: &'static hardware::ButtonLEDType,
    buzzer: &'static hardware::BuzzerType,
) {
    // TODO: basically make the buzzer beeping a separate task, that
    // waits for a message on a channel
    loop {
        button.lock().await.as_mut().unwrap().wait_for_low().await;
        led.lock().await.as_mut().unwrap().set_low();

        info!("right button triggered");

        // wait 200ms and alternate buzzer
        for _ in 0..200 {
            buzzer.lock().await.as_mut().unwrap().toggle();
            Timer::after(Duration::from_millis(1)).await;
        }

        //buzzer.lock().await.as_mut().unwrap().set_high();
        //Timer::after(Duration::from_millis(200)).await;
        buzzer.lock().await.as_mut().unwrap().set_low();
        led.lock().await.as_mut().unwrap().set_high();
    }
}

#[task]
pub async fn rotary_switch_left_event(
    rotary_switch: &'static hardware::RotarySwitchType,
    led: &'static hardware::ButtonLEDType,
    buzzer: &'static hardware::BuzzerType,
) {
    // TODO: basically make the buzzer beeping a separate task, that
    // waits for a message on a channel
    loop {
        rotary_switch
            .lock()
            .await
            .as_mut()
            .unwrap()
            .wait_for_falling_edge()
            .await;

        let params = LEDRotationParams::default();
        LED_ROTATION_SIGNAL.signal(params);

        led.lock().await.as_mut().unwrap().set_low();

        Timer::after(Duration::from_millis(200)).await;

        //buzzer.lock().await.as_mut().unwrap().set_high();
        //Timer::after(Duration::from_millis(200)).await;
        //buzzer.lock().await.as_mut().unwrap().set_low();
        led.lock().await.as_mut().unwrap().set_high();
    }
}

#[task]
pub async fn rotary_switch_right_event(
    rotary_switch: &'static hardware::RotarySwitchType,
    led: &'static hardware::ButtonLEDType,
    buzzer: &'static hardware::BuzzerType,
) {
    // TODO: basically make the buzzer beeping a separate task, that
    // waits for a message on a channel
    loop {
        rotary_switch
            .lock()
            .await
            .as_mut()
            .unwrap()
            .wait_for_falling_edge()
            .await;
        led.lock().await.as_mut().unwrap().set_low();

        // play simple tone
        BUZZER_SIGNAL.signal(BuzzerSequence::SimpleTone200ms);

        Timer::after(Duration::from_millis(200)).await;

        //buzzer.lock().await.as_mut().unwrap().set_high();
        //Timer::after(Duration::from_millis(200)).await;
        //buzzer.lock().await.as_mut().unwrap().set_low();
        led.lock().await.as_mut().unwrap().set_high();
    }
}

#[task]
pub async fn play_sequence_listener(buzzer: &'static hardware::BuzzerType) {
    // TODO: basically make the buzzer beeping a separate task, that
    // waits for a message on a channel
    loop {
        let sequence = BUZZER_SIGNAL.wait().await;
        execute_sequence(buzzer, &sequence).await;
        // execute sequency here
    }
}

async fn execute_sequence(buzzer: &'static hardware::BuzzerType, sequence: &BuzzerSequence) {
    match sequence {
        BuzzerSequence::SimpleTone200ms => {
            buzzer.lock().await.as_mut().unwrap().set_high();
            Timer::after(Duration::from_millis(200)).await;

            buzzer.lock().await.as_mut().unwrap().set_low();
        }
        BuzzerSequence::Intermittent10ms200ms => {
            for _ in 0..20 {
                buzzer.lock().await.as_mut().unwrap().toggle();
                Timer::after(Duration::from_millis(10)).await;
            }

            buzzer.lock().await.as_mut().unwrap().set_low();
        }
        BuzzerSequence::Intermittent5ms2000ms => {
            for _ in 0..400 {
                buzzer.lock().await.as_mut().unwrap().toggle();
                Timer::after(Duration::from_millis(5)).await;
            }

            buzzer.lock().await.as_mut().unwrap().set_low();
        }
    }
}

/// Specifies the parameters for an led rotation. You can specify
/// a pattern of up to 20 LEDs. You can also specify the amount of cycles
/// to do.

pub struct LEDRotationParams {
    /// LED lightup pattern order.
    pub selection: Vec<LEDSelect, 20>,
    /// Amount of cycles of this to do.
    pub cycles: u64,
    /// Amount of time one LED stays on, in ms.
    pub interval: u64,
}

impl Default for LEDRotationParams {
    fn default() -> Self {
        let mut selection = Vec::new();
        selection.push(LEDSelect::RED).unwrap();
        selection.push(LEDSelect::GREEN).unwrap();
        selection.push(LEDSelect::BLUE).unwrap();
        selection.push(LEDSelect::YELLOW).unwrap();
        selection.push(LEDSelect::WHITE).unwrap();

        LEDRotationParams {
            selection,
            cycles: 5,
            interval: 80,
        }
    }
}

#[repr(usize)]
#[derive(Clone, Copy, Debug)]
pub enum LEDSelect {
    RED = 0,
    GREEN = 1,
    BLUE = 2,
    YELLOW = 3,
    WHITE = 4,
}

#[task]
pub async fn led_rotation() {
    loop {
        let params = LED_ROTATION_SIGNAL.wait().await;

        all_leds_off().await;

        // save the last index, so we can wait half the interval time
        // before turning it off
        let mut last_i: Option<usize> = None;

        for _ in 0..params.cycles {
            for led_select in &params.selection {
                let i: usize = *led_select as usize;
                LED_ARRAY[i].lock().await.as_mut().unwrap().set_high();

                let half_time = params.interval / 2;

                // wait half the interval to turn the previous one off
                Timer::after(Duration::from_millis(half_time)).await;

                if let Some(last) = last_i {
                    LED_ARRAY[last].lock().await.as_mut().unwrap().set_low();
                }

                last_i = Some(i);

                Timer::after(Duration::from_millis(half_time)).await;
            }
        }

        all_leds_off().await;
    }
}

/// Encodes for the current state of the A and B lines. This makes
/// it easier to "traverse" the line signal, as each option only has
/// one valid "path" it can take.
#[derive(Copy, Clone, Debug)]
enum GrayState {
    AB00, // A=0, B=0
    AB01, // A=0, B=1
    AB11, // A=1, B=1
    AB10, // A=1, B=0
}

// impl GrayState {
//     /// Creates a GrayState by polling the right rotary encoder's pins
//     async fn new_right() -> Self {
//         let a = ROTARY_RIGHT_A.lock().await.as_ref().unwrap().is_high();
//         let b = ROTARY_RIGHT_B.lock().await.as_ref().unwrap().is_high();

//         match (a, b) {
//             (false, false) => GrayState::AB00,
//             (false, true) => GrayState::AB01,
//             (true, true) => GrayState::AB11,
//             (true, false) => GrayState::AB10,
//         }
//     }
// }

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum MicroRotation {
    Clockwise,
    CounterClockwise,
    None, // bounce / illegal / no movement
}

fn decode_transition(from: GrayState, to: GrayState) -> MicroRotation {
    match (from, to) {
        // clockwise (B leads A)
        (GrayState::AB00, GrayState::AB01) => MicroRotation::Clockwise,
        (GrayState::AB01, GrayState::AB11) => MicroRotation::Clockwise,
        (GrayState::AB11, GrayState::AB10) => MicroRotation::Clockwise,
        (GrayState::AB10, GrayState::AB00) => MicroRotation::Clockwise,

        // counter-clockwise
        (GrayState::AB00, GrayState::AB10) => MicroRotation::CounterClockwise,
        (GrayState::AB10, GrayState::AB11) => MicroRotation::CounterClockwise,
        (GrayState::AB11, GrayState::AB01) => MicroRotation::CounterClockwise,
        (GrayState::AB01, GrayState::AB00) => MicroRotation::CounterClockwise,

        // bounce, illegal, or no change
        _ => MicroRotation::None,
    }
}

#[task]
pub async fn left_rotary_rotation_watcher(
    left_rotary_a: Input<'static>,
    left_rotary_b: Input<'static>,
) {
    // start an encoder that we set the values of manually
    let mut raw_encoder = QuadratureTableMode::new(4);
    let _dir = raw_encoder.update(false, false);

    loop {
        // whenever this happens, update the state of the encoder
        let dir = raw_encoder.update(left_rotary_b.is_low(), left_rotary_a.is_low());

        match dir {
            Direction::Clockwise => {
                YELLOW_LED.lock().await.as_mut().unwrap().set_high();
                RED_LED.lock().await.as_mut().unwrap().set_low();
            }
            Direction::Anticlockwise => {
                RED_LED.lock().await.as_mut().unwrap().set_high();
                YELLOW_LED.lock().await.as_mut().unwrap().set_low();
            }
            Direction::None => {}
        }

        Timer::after(Duration::from_micros(1000)).await; // 1 kHz
    }
}

#[task]
pub async fn right_rotary_rotation_watcher(
    right_rotary_a: Input<'static>,
    right_rotary_b: Input<'static>,
) {
    // start an encoder that we set the values of manually
    let mut raw_encoder = QuadratureTableMode::new(4);
    let _dir = raw_encoder.update(false, false);

    loop {
        // whenever this happens, update the state of the encoder
        let dir = raw_encoder.update(right_rotary_b.is_low(), right_rotary_a.is_low());

        match dir {
            Direction::Clockwise => {
                BLUE_LED.lock().await.as_mut().unwrap().set_high();
                GREEN_LED.lock().await.as_mut().unwrap().set_low();
                //info!("clockwise");
                //Timer::after(Duration::from_millis(200)).await;

                // Increment some value
            }
            Direction::Anticlockwise => {
                GREEN_LED.lock().await.as_mut().unwrap().set_high();
                BLUE_LED.lock().await.as_mut().unwrap().set_low();
                //Timer::after(Duration::from_millis(200)).await;

                // Decrement some value
            }
            Direction::None => {
                //info!("nothing");
                // Do nothing
            }
        }

        Timer::after(Duration::from_micros(1000)).await; // 1 kHz
    }
}

pub async fn all_leds_off() {
    // set all leds to off
    for led in LED_ARRAY {
        led.lock().await.as_mut().unwrap().set_low();
    }
}
