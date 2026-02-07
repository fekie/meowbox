use defmt::Format;
#[allow(unused_imports)]
use defmt::{error, info, warn};
use embassy_executor::task;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel,
    signal::Signal,
};
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;

use crate::physics;

pub mod error_state;
pub mod flow_field;
pub mod light_ring;

static FOO: StaticCell<u32> = StaticCell::new();

static _PARTICLES: StaticCell<[physics::Particle; 5]> =
    StaticCell::new();

/// Static cells are used in this program to hold values we want
/// to be on the stack, while also being able to juggle around a
/// mutable reference for it. This has to be done because a value
/// created inside a setup function will be dropped and not live long
/// enough. It is technically possible to return it and pass it up the
/// chain, but it complicates the interface and requires more copying.
pub struct Resources {
    pub particles: &'static mut [physics::Particle; 5],
}

/// State must be contained inside a wrapper. This is because
/// including next_state within State would cause an infinite size
/// type. Because a Box requires a heap (which the program does not
/// have), a wrapper containing both must be used.
pub struct Meowbox {
    pub state: State,
    /// If next_state exists, the next tick will go into shutdown.
    /// Each tick, the shutdown function will be run until the
    /// shutdown function changes the state and clears
    /// next_state.
    pub next_state: Option<State>,
    /// This is set to true when a shutdown is needed. When this is
    /// set to true, the state will go into shutdown mode, but
    /// `next_state` will not be set to None until the state is
    /// actually transisitoned.
    pub needs_to_shutdown: bool,
    pub resources: Resources,
}

impl Meowbox {
    pub fn new(starting_state: State) -> Self {
        let resources = Resources {
            particles: _PARTICLES.init([
                physics::Particle::default(),
                physics::Particle::default(),
                physics::Particle::default(),
                physics::Particle::default(),
                physics::Particle::default(),
            ]),
        };

        Meowbox {
            state: starting_state,
            next_state: None,
            needs_to_shutdown: false,
            resources,
        }
    }

    pub async fn tick(&mut self) {
        self.check_for_shutdown_transition();

        //*self.resources.foo += 1;
        //info!("{}", self.resources.foo);

        match self.state {
            State::LightRing(_, _) => self.tick_light_ring().await,
            State::FlowField(_, _) => self.tick_flow_field().await,
            State::ErrorState(_) => self.tick_error_state().await,
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

        self.needs_to_shutdown = false;

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
                );
            }
            State::Debug(_, _, _) => {
                self.state =
                    State::ErrorState(ErrorStateType::Unknown);
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
    /// Does both the light ring and the flow field. This is a good
    /// way to see if the device is still "running" properly
    Debug(Stage, LightRingState, FlowFieldState),
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
