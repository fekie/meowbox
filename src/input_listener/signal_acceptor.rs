use super::{Input, InputListener};

impl InputListener {
    pub(super) fn handle_no_wait_for_signal(&mut self, input: Input) {
        todo!()
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
    }
}
