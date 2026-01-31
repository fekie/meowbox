use defmt::{error, info, warn};
use embassy_executor::task;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::Input;
use heapless::Vec;
use rotary_encoder_embedded::{
    Direction, quadrature::QuadratureTableMode,
};

use super::{
    BUZZER_SIGNAL, BuzzerSequence, LED_ROTATION_SIGNAL,
    LEDRotationParams, hardware,
};
use crate::hardware::{
    BLUE_LED, GREEN_LED, LED_ARRAY, RED_LED, YELLOW_LED,
};

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
pub async fn left_rotary_rotation_watcher(
    left_rotary_a: Input<'static>,
    left_rotary_b: Input<'static>,
) {
    // start an encoder that we set the values of manually
    let mut raw_encoder = QuadratureTableMode::new(4);
    let _dir = raw_encoder.update(false, false);

    loop {
        // whenever this happens, update the state of the encoder
        let dir = raw_encoder
            .update(left_rotary_b.is_low(), left_rotary_a.is_low());

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
        let dir = raw_encoder
            .update(right_rotary_b.is_low(), right_rotary_a.is_low());

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
