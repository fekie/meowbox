use defmt::{error, info, warn};
use embassy_executor::task;
use embassy_time::{Duration, Timer};

use crate::hardware::{
    BLUE_LED, GREEN_LED, LED_ARRAY, LEDType, RED_LED, ROTARY_RIGHT_A, ROTARY_RIGHT_B,
};

use super::hardware;

use embassy_sync::signal::Signal;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};

use heapless::{Vec, vec};

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
        BUZZER_SIGNAL.signal(BuzzerSequence::Intermittent5ms2000ms);

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

#[task]
pub async fn right_rotary_rotation_watcher() {
    loop {
        // We check rotation by waiting for the A signal to go high.
        // If B is high when A is high, then we know B came first and we
        // went clockwise
        ROTARY_RIGHT_A
            .lock()
            .await
            .as_mut()
            .unwrap()
            .wait_for_rising_edge()
            .await;

        match ROTARY_RIGHT_B.lock().await.as_mut().unwrap().is_high() {
            // CW rotation
            true => {
                GREEN_LED.lock().await.as_mut().unwrap().toggle();
            }
            // CCW rotation
            false => {
                BLUE_LED.lock().await.as_mut().unwrap().toggle();
            }
        }

        loop {
            let a_is_low = ROTARY_RIGHT_A.lock().await.as_mut().unwrap().is_low();
            let b_is_low = ROTARY_RIGHT_B.lock().await.as_mut().unwrap().is_low();

            if (a_is_low && b_is_low) {
                break;
            }

            Timer::after(Duration::from_micros(100)).await;
        }

        // wait until both lines are low
    }
}

// #[task]
// async fn full_led_rotation(
//     red_led: &'static hardware::LEDType,
//     green_led: &'static hardware::LEDType,
//     blue_led: &'static hardware::LEDType,
//     yellow_led: &'static hardware::LEDType,
//     white_led: &'static hardware::LEDType,
//     cycles: u16,
// ) {
//     // make sure all of them are set low first
//     red_led.lock().await.as_mut().unwrap().set_low();
//     green_led.lock().await.as_mut().unwrap().set_low();
//     blue_led.lock().await.as_mut().unwrap().set_low();
//     yellow_led.lock().await.as_mut().unwrap().set_low();
//     white_led.lock().await.as_mut().unwrap().set_low();

//     for _ in 0..cycles {
//         red_led.lock().await.as_mut().unwrap().set_high();
//     }
// }

pub async fn all_leds_off() {
    // set all leds to off
    for led in LED_ARRAY {
        led.lock().await.as_mut().unwrap().set_low();
    }
}
