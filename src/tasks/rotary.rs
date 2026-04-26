use embassy_executor::task;
use embassy_time::{Duration, Timer};
use esp_hal::gpio;
use rotary_encoder_embedded::{
    Direction, quadrature::QuadratureTableMode,
};

use super::hardware;
use crate::{input_listener, input_listener::INPUT_CHANNEL};

const ROTARY_SW_DEBOUNCE_MS: u64 = 200;

#[task]
pub async fn rotary_switch_left_event(
    rotary_switch: &'static hardware::RotarySwitchType,
    //buzzer: &'static hardware::BuzzerType,
    //led: &'static hardware::ButtonLEDType,
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

        INPUT_CHANNEL
            .send(input_listener::Input::RotaryEncoderPressLeft)
            .await;

        Timer::after(Duration::from_millis(ROTARY_SW_DEBOUNCE_MS))
            .await;
    }
}

#[task]
pub async fn rotary_switch_right_event(
    rotary_switch: &'static hardware::RotarySwitchType,
) {
    loop {
        rotary_switch
            .lock()
            .await
            .as_mut()
            .unwrap()
            .wait_for_falling_edge()
            .await;

        INPUT_CHANNEL
            .send(input_listener::Input::RotaryEncoderPressRight)
            .await;

        Timer::after(Duration::from_millis(ROTARY_SW_DEBOUNCE_MS))
            .await;
    }
}

#[task]
pub async fn left_rotary_rotation_watcher(
    left_rotary_a: gpio::Input<'static>,
    left_rotary_b: gpio::Input<'static>,
) {
    // start an encoder that we set the values of manually

    let mut raw_encoder = QuadratureTableMode::new(3);
    let _dir = raw_encoder.update(false, false);

    loop {
        // whenever this happens, update the state of the encoder
        let dir = raw_encoder
            .update(left_rotary_b.is_low(), left_rotary_a.is_low());

        match dir {
            Direction::Clockwise => {
                INPUT_CHANNEL
                    .send(
                        input_listener::Input::RotaryEncoderRotateLeft(Direction::Clockwise),
                    )
                    .await;
                //light_ring.forward().await;
                //menu_scroll_down();
            }
            Direction::Anticlockwise => {
                INPUT_CHANNEL
                    .send(
                        input_listener::Input::RotaryEncoderRotateLeft(Direction::Anticlockwise),
                    )
                    .await;
                //light_ring.backward().await;
                //menu_scroll_up();
            }
            Direction::None => {}
        }

        Timer::after(Duration::from_micros(1000)).await; // 1 kHz
    }
}

#[task]
pub async fn right_rotary_rotation_watcher(
    right_rotary_a: gpio::Input<'static>,
    right_rotary_b: gpio::Input<'static>,
) {
    // start an encoder that we set the values of manually
    // this used to be 4, but it would sometimes miss count.
    // even still, any value of this usually overcounts
    let mut raw_encoder = QuadratureTableMode::new(3);
    let _dir = raw_encoder.update(false, false);

    loop {
        // whenever this happens, update the state of the encoder
        let dir = raw_encoder
            .update(right_rotary_b.is_low(), right_rotary_a.is_low());

        match dir {
            Direction::Clockwise => {
                INPUT_CHANNEL
                    .send(
                        input_listener::Input::RotaryEncoderRotateRight(Direction::Clockwise),
                    )
                    .await;
            }
            Direction::Anticlockwise => {
                INPUT_CHANNEL
                    .send(
                        input_listener::Input::RotaryEncoderRotateRight(Direction::Anticlockwise),
                    )
                    .await;
            }
            Direction::None => {}
        }

        Timer::after(Duration::from_micros(1000)).await; // 1 kHz
    }
}
