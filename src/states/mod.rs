use defmt::Format;
#[allow(unused_imports)]
use defmt::{error, info, warn};
use embassy_executor::task;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal,
};
use menu_state::menu::MenuResources;

use crate::{leds::LightRingState, physics::PhysicsResources};

pub mod automata;
pub mod cries;
pub mod error_state;
pub mod flow_field;
pub mod light_ring_loop;
pub mod light_show;
pub mod menu_state;
pub mod unimplemented;

/// Static cells are used in this program to hold values we want
/// to be on the stack, while also being able to juggle around a
/// mutable reference for it. This has to be done because a value
/// created inside a setup function will be dropped and not live long
/// enough. It is technically possible to return it and pass it up the
/// chain, but it complicates the interface and requires more copying.
pub struct Resources {
    pub physics_resources: PhysicsResources,
    pub menu_resoures: MenuResources,
    //pub particles: &'static mut [physics::Particle; 5],
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
            physics_resources: PhysicsResources::new(),
            menu_resoures: MenuResources::new(),
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
            State::LightRing(_, _) => {
                self.tick_light_ring_loop().await
            }
            State::FlowField(_, _) => self.tick_flow_field().await,
            State::ErrorState(_) => self.tick_error_state().await,
            State::Menu(_, _) => self.tick_menu_state().await,
            State::LightShow(_) => self.tick_light_show().await,
            State::Cries(_, _) => self.tick_cries().await,
            State::Automata(_, _) => self.tick_automata().await,
            State::Unimplemented(_) => {
                self.tick_unimplemented().await
            }
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
            State::Menu(_, current) => {
                self.state = State::Menu(Stage::Shutdown, current);
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
            State::LightShow(_) => {
                self.state = State::LightShow(Stage::Shutdown);
            }
            State::Cries(_, cry_index) => {
                self.state = State::Cries(Stage::Shutdown, cry_index);
            }
            State::Automata(_, automata_state) => {
                self.state =
                    State::Automata(Stage::Shutdown, automata_state);
            }
            State::Unimplemented(_) => {
                self.state = State::Unimplemented(Stage::Shutdown);
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

#[derive(Clone, Copy, Debug, Default)]
pub struct MenuState {
    /// The amount of scroll (starting from the top) that the screen
    /// needs to display.
    scroll: usize,
    /// The currently index of the currently covered item.
    index: usize,
}

/// The overarching state of the machine that specifies which routines
/// it is currently running. States can contain smaller substates.
#[derive(Clone, Copy, Debug)]
pub enum State {
    Menu(Stage, MenuState),
    LightRing(Stage, LightRingState),
    FlowField(Stage, FlowFieldState),
    LightShow(Stage),
    Cries(Stage, usize),
    Automata(Stage, AutomataState),
    Unimplemented(Stage),
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

/// Variations of the flow field state.
#[derive(Clone, Copy, Debug)]
pub enum FlowFieldState {
    Slow,
    Fast,
}

#[derive(Clone, Copy, Debug)]
pub struct AutomataState {
    pub rule: u8,
    pub palette_index: i32,
    pub camera_x: usize,
    pub camera_y: usize,
}

impl Default for AutomataState {
    fn default() -> Self {
        Self {
            rule: 1,
            palette_index: 0,
            camera_x: 60,
            camera_y: 0,
        }
    }
}

pub static STATE_CHANGE_REQUEST: Signal<
    CriticalSectionRawMutex,
    State,
> = Signal::new();

#[task]
pub async fn left_button_event() {
    loop {
        let _next_state = STATE_CHANGE_REQUEST.wait().await;
    }
}
