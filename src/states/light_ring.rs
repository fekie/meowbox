use super::{Meowbox, State};
use crate::tasks::all_leds_off;

// Light Ring
impl Meowbox {
    pub(super) async fn setup_light_ring(&mut self) {}

    pub(super) async fn execute_light_ring(&mut self) {}

    /// This method is called if the state is in shutdown. Shutdown
    /// is only started when an item exists in next_state.
    pub(super) async fn shutdown_light_ring(&mut self) {
        // TODO: turn all lights off
        all_leds_off().await;

        self.state = self.next_state.take().unwrap();

        // change to next state
        //*self = self.next_state;
    }
}
