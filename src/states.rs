/// The overarching state of the machine that specifies which routines
/// it is currently running. States can contain smaller substates.
pub enum State {
    Menu(Stage),
    FlowField(Stage, FlowFieldState),
}

/// Used to determine which stage of the command/state is currently
/// being run. Whenever the state is going to change, the current
/// state will be put into Shutdown, and then turn itself into the
/// Setup portion of the next state.
pub enum Stage {
    Setup,
    Execution,
    Shutdown,
}

/// Variations of the flow field state.
pub enum FlowFieldState {
    Slow,
    Fast,
}
