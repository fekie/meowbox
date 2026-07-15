use core::sync::atomic::{AtomicU16, Ordering::SeqCst};

use rotary_encoder_embedded::Direction;

use super::{Input, InputListener};

impl InputListener {
    pub(super) fn handle_no_wait_for_signal(input: Input) {
        match input {
            Input::RotaryEncoderPressLeft => {
                increment_counter(&super::ROTARY_ENCODER_PRESS_LEFT);
            }

            Input::RotaryEncoderRotateLeft(dir) => match dir {
                Direction::Clockwise => {
                    increment_counter(
                        &super::ROTARY_ENCODER_ROTATE_LEFT_CW,
                    );
                }
                Direction::Anticlockwise => {
                    increment_counter(
                        &super::ROTARY_ENCODER_ROTATE_LEFT_CCW,
                    );
                }
                Direction::None => {
                    panic!("Direction should not be None.")
                }
            },

            Input::RotaryEncoderPressRight => {
                increment_counter(&super::ROTARY_ENCODER_PRESS_RIGHT);
            }

            Input::RotaryEncoderRotateRight(dir) => match dir {
                Direction::Clockwise => {
                    increment_counter(
                        &super::ROTARY_ENCODER_ROTATE_RIGHT_CW,
                    );
                }
                Direction::Anticlockwise => {
                    increment_counter(
                        &super::ROTARY_ENCODER_ROTATE_RIGHT_CCW,
                    );
                }
                Direction::None => {
                    panic!("Direction should not be None.")
                }
            },

            Input::ButtonLeft => {
                increment_counter(&super::BUTTON_LEFT);
            }
            Input::ButtonRight => {
                increment_counter(&super::BUTTON_RIGHT);
            }
            Input::ButtonRightReleased => {
                increment_counter(&super::BUTTON_RIGHT_RELEASED);
            }

            Input::DpadBottom => {
                increment_counter(&super::DPAD_BOTTOM);
            }
            Input::DpadTop => {
                increment_counter(&super::DPAD_TOP);
            }
            Input::DpadLeft => {
                increment_counter(&super::DPAD_LEFT);
            }
            Input::DpadRight => {
                increment_counter(&super::DPAD_RIGHT);
            }
        }
    }

    pub(super) fn handle_wait_for_signal(
        &mut self,
        input: Input,
        waited_input: Input,
    ) {
        todo!()
    }

    pub(super) fn handle_wait_for_any_signal(
        &mut self,
        input: Input,
    ) {
        todo!()
    }
}

fn increment_counter(counter: &AtomicU16) {
    let _ = counter.fetch_update(SeqCst, SeqCst, |value| {
        Some(value.saturating_add(1))
    });
}
