use core::sync::atomic::Ordering::SeqCst;

use rotary_encoder_embedded::Direction;

use super::{Input, InputListener};

impl InputListener {
    pub(super) fn handle_no_wait_for_signal(input: Input) {
        match input {
            Input::RotaryEncoderPressLeft => {
                super::ROTARY_ENCODER_PRESS_LEFT.fetch_add(1, SeqCst);
            }

            Input::RotaryEncoderRotateLeft(dir) => match dir {
                Direction::Clockwise => {
                    super::ROTARY_ENCODER_ROTATE_LEFT_CW
                        .fetch_add(1, SeqCst);
                }
                Direction::Anticlockwise => {
                    super::ROTARY_ENCODER_ROTATE_LEFT_CCW
                        .fetch_add(1, SeqCst);
                }
                Direction::None => {
                    panic!("Direction should not be None.")
                }
            },

            Input::RotaryEncoderPressRight => {
                super::ROTARY_ENCODER_PRESS_RIGHT
                    .fetch_add(1, SeqCst);
            }

            Input::RotaryEncoderRotateRight(dir) => match dir {
                Direction::Clockwise => {
                    super::ROTARY_ENCODER_ROTATE_RIGHT_CW
                        .fetch_add(1, SeqCst);
                }
                Direction::Anticlockwise => {
                    super::ROTARY_ENCODER_ROTATE_RIGHT_CCW
                        .fetch_add(1, SeqCst);
                }
                Direction::None => {
                    panic!("Direction should not be None.")
                }
            },
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
