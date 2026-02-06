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
    /// Run the next tick based on the current state.
    pub async fn tick(&mut self) {
        match self {
            Self::LightRing(_, _) => {
                self.control_light_ring();
            }
            Self::ErrorState(etype) => match etype {
                _ => {
                    RED_LED.lock().await.as_mut().unwrap().toggle();
                    error!("error state");
                    Timer::after(Duration::from_millis(200)).await;
                }
            },

            // If we dont yet have the state implemented, go to the
            // error state.
            _ => {
                *self = Self::ErrorState(
                    ErrorStateType::StateNotImplemented,
                )
            }
        }
    }

    pub fn setup() {}
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

#[derive(Default, Clone, Copy, Debug)]
pub enum ErrorStateType {
    #[default]
    Unknown,
    StateNotImplemented,
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
