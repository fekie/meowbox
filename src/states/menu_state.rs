use defmt::info;
use embassy_time::{Duration, Timer};

use super::{Meowbox, State, light_ring::LightRingState};
use crate::{
    states::{ErrorStateType, MenuState, Stage},
    tasks::all_leds_off,
};

// Light Ring
impl Meowbox {
    pub(super) async fn tick_menu_state(&mut self) {
        if let State::Menu(stage, _) = self.state {
            match stage {
                Stage::Setup => self.setup_menu_state().await,
                Stage::Execution => self.execute_menu_state().await,
                Stage::Shutdown => self.shutdown_menu_state().await,
            }
        }
    }

    async fn setup_menu_state(&mut self) {
        // turn all leds off and go to next state
        all_leds_off().await;
        self.state =
            State::Menu(Stage::Execution, MenuState::default());
    }

    async fn execute_menu_state(&mut self) {
        if let State::Menu(_, menu_state) = &mut self.state {
            // Display stuff here
            info!("display menu")
        }

        Timer::after(Duration::from_millis(500)).await;
    }

    /// This method is called if the state is in shutdown. Shutdown
    /// is only started when an item exists in next_state.
    async fn shutdown_menu_state(&mut self) {
        // TODO: turn all lights off
        //all_leds_off().await;

        self.state = match self.next_state.take() {
            Some(x) => x,
            None => State::ErrorState(
                ErrorStateType::NextStateNotSpecified,
            ),
        }
    }
}
