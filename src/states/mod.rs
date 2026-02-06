use defmt::Format;
#[allow(unused_imports)]
use defmt::{error, info, warn};
use embassy_executor::task;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel,
    signal::Signal,
};
use embassy_time::{Duration, Timer};

use super::hardware::{
    self, BLUE_LED, GREEN_LED, LED_ARRAY, RED_LED, YELLOW_LED,
};

pub mod error_state;
pub mod light_ring;

/// State must be contained inside a wrapper. This is because
/// including next_state within State would cause an infinite size
/// type. Because a Box requires a heap (which the program does not
/// have), a wrapper containing both must be used.
pub struct Meowbox {
    state: State,
    /// If next_state exists, the next tick will go into shutdown.
    /// Each tick, the shutdown function will be run until the
    /// shutdown function changes the state and clears
    /// next_state.
    next_state: Option<State>,
    /// This is set to true when a shutdown is needed. When this is
    /// set to true, the state will go into shutdown mode, but
    /// `next_state` will not be set to None until the state is
    /// actually transisitoned.
    needs_to_shutdown: bool,
}

impl Meowbox {
    pub async fn tick(&mut self) {
        self.check_for_shutdown_transition();

        match self.state {
            State::LightRing(stage, _) => {
                match stage {
                    Stage::Setup => self.setup_light_ring().await,
                    Stage::Execution => {
                        self.execute_light_ring().await
                    }
                    // shutdown should
                    Stage::Shutdown => {
                        self.shutdown_light_ring().await
                    }
                }
            }
            State::ErrorState(etype) => match etype {
                _ => {
                    RED_LED.lock().await.as_mut().unwrap().toggle();
                    error!("error state {:?}", etype);
                    Timer::after(Duration::from_millis(200)).await;
                }
            },

            // If we dont yet have the state implemented, go to the
            // error state.
            _ => {
                self.state = State::ErrorState(
                    ErrorStateType::StateNotImplemented,
                )
            }
        }
    }

    /// Goes into shutdown if `needs_to_shutdown` is `true`
    fn check_for_shutdown_transition(&mut self) {
        if !self.needs_to_shutdown {
            return;
        }

        // If there is a need to shutdown, then set to shutdown.
        // Annoying, a match tree has to be used here.
        match self.state {
            State::Menu(_) => {
                self.state = State::Menu(Stage::Shutdown);
            }
            State::LightRing(_, light_ring_state) => {
                self.state = State::LightRing(
                    Stage::Shutdown,
                    light_ring_state,
                );
            }
            State::FlowField(_, flow_field_state) => {
                self.state = State::FlowField(
                    Stage::Shutdown,
                    flow_field_state,
                )
            }
            // If we hit errorr state, dont change anything
            State::ErrorState(_) => {}
        }
    }
}

/// The overarching state of the machine that specifies which routines
/// it is currently running. States can contain smaller substates.
#[derive(Clone, Copy, Debug)]
pub enum State {
    Menu(Stage),
    LightRing(Stage, LightRingState),
    FlowField(Stage, FlowFieldState),
    ErrorState(ErrorStateType),
}

impl State {
    // Run the next tick based on the current state. Optionally
    // return the next state to switch to.
    //async fn tick(&mut self) -> Option<State> {}
}

/// Used to determine which stage of the command/state is currently
/// being run. Whenever the state is going to change, the current
/// state will be put into Shutdown, and then turn itself into the
/// Setup portion of the next state.
#[derive(Default, Clone, Copy, Debug)]
pub enum Stage {
    #[default]
    Setup,
    Execution,
    Shutdown,
}

#[derive(Default, Clone, Copy, Debug, Format)]
pub enum ErrorStateType {
    #[default]
    Unknown,
    StateNotImplemented,
    NextStateNotSpecified,
}

#[derive(Default, Clone, Copy, Debug)]
pub enum LightRingState {
    #[default]
    Red,
    Green,
    Blue,
    Yellow,
    White,
}

/// Variations of the flow field state.
#[derive(Clone, Copy, Debug)]
pub enum FlowFieldState {
    Slow,
    Fast,
}

pub static STATE_CHANGE_REQUEST: Signal<
    CriticalSectionRawMutex,
    State,
> = Signal::new();

#[task]
pub async fn left_button_event() {
    loop {
        let next_state = STATE_CHANGE_REQUEST.wait().await;
    }
}
