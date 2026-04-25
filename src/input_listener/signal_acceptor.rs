use rotary_encoder_embedded::Direction;

use super::{Input, InputListener};

impl InputListener {
    pub(super) fn handle_no_wait_for_signal(&mut self, input: Input) {
        match input {
            Input::RotaryEncoderPressLeft => {
                self.rotary_encoder_press_left += 1
            }
            Input::RotaryEncoderRotateLeft(dir) => match dir {
                Direction::Clockwise => {
                    self.rotary_encoder_rotate_left_cw += 1;
                }
                Direction::Anticlockwise => {
                    self.rotary_encoder_rotate_left_ccw += 1
                }
                Direction::None => {
                    panic!("Direction should not be None.")
                }
            },

            Input::RotaryEncoderPressRight => {
                self.rotary_encoder_press_right += 1
            }
            Input::RotaryEncoderRotateRight(dir) => match dir {
                Direction::Clockwise => {
                    self.rotary_encoder_rotate_right_cw += 1
                }
                Direction::Anticlockwise => {
                    self.rotary_encoder_rotate_right_ccw += 1
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
        if !(input == waited_input) {
            self.handle_no_wait_for_signal(input);
            return;
        }
    }

    pub(super) fn handle_wait_for_any_signal(
        &mut self,
        input: Input,
    ) {
        todo!()
    }
}
