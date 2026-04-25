//! Listens for all inputs. Designed for use in the state machine as
//! inputs through here can either be polled or waited on. The input
//! listener is also able to be killed by a signal (so that it is
//! possible to get a state machine to stop, even if it is waiting on
//! a signal.)

use defmt::dbg;
use embassy_executor::task;
use embassy_futures::select::Either;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel,
    mutex::Mutex,
};
use rotary_encoder_embedded::Direction;
use static_cell::StaticCell;

mod signal_acceptor;

const BUFFERED_INPUTS_SIZE: usize = 32;

pub static INPUT_CHANNEL: Channel<
    CriticalSectionRawMutex,
    Input,
    BUFFERED_INPUTS_SIZE,
> = Channel::new();

// okay so. i will store how many times something occurs. When an
// input is "taken", a parameter will be passed in

#[derive(Clone, Copy)]
pub struct AllInputs;

#[derive(Clone)]
pub struct InputListener {
    /// This is marked as Some with the specified input if there is
    /// an external source waiting on a signal. It basically says
    /// to start "forwarding" a signal to a waiter, instead of
    /// incrementing the counter for the keypress.
    pub external_wait_for_signal: Option<Either<Input, AllInputs>>,

    rotary_encoder_press_left: u8,
    rotary_encoder_rotate_left_cw: u8,
    rotary_encoder_rotate_left_ccw: u8,

    rotary_encoder_press_right: u8,
    rotary_encoder_rotate_right_cw: u8,
    rotary_encoder_rotate_right_ccw: u8,
}

static INPUT_LISTENER: Mutex<CriticalSectionRawMutex, InputListener> =
    Mutex::new(InputListener {
        external_wait_for_signal: None,
        rotary_encoder_press_left: 0,
        rotary_encoder_rotate_left_cw: 0,
        rotary_encoder_rotate_left_ccw: 0,
        rotary_encoder_press_right: 0,
        rotary_encoder_rotate_right_cw: 0,
        rotary_encoder_rotate_right_ccw: 0,
    });

// pub static INPUT_LISTENER: StaticCell<InputListener> =
//     StaticCell::new();

/// Initializes listener and starts listening for inputs.
#[task]
pub async fn start_input_listener_listener() {
    loop {
        let input = INPUT_CHANNEL.receive().await;

        let external_wait_for_signal = INPUT_LISTENER
            .lock()
            .await
            .external_wait_for_signal
            .clone();

        match external_wait_for_signal {
            // check to make sure that the input doesnt need to be
            // forwarded
            Some(input_kind) => match input_kind {
                Either::First(waited_input) => INPUT_LISTENER
                    .lock()
                    .await
                    .handle_wait_for_signal(input, waited_input),
                Either::Second(_any_input) => INPUT_LISTENER
                    .lock()
                    .await
                    .handle_wait_for_any_signal(input),
            },
            None => INPUT_LISTENER
                .lock()
                .await
                .handle_no_wait_for_signal(input),
        }
    }
}

// basically i will need a way to "drain" unused inputs. I think I
// will have to do this on a time basis. Or I could continue to store
// that it at least happened once.

#[derive(Debug, Clone, Copy)]
pub enum Input {
    RotaryEncoderPressLeft,
    RotaryEncoderRotateLeft(Direction),
    RotaryEncoderPressRight,
    RotaryEncoderRotateRight(Direction),
}

/// If this is found in a Result, the program should exit and change
/// state
pub struct KillSignal;

impl InputListener {
    // pub fn new() -> Self {
    //     Self::default()
    // }

    /// Wait for any input, including inputs that have already
    /// happened. Each call to this function will "take" one instance
    /// of the keypress.
    pub fn wait_for_any() -> Result<Input, KillSignal> {
        todo!()
    }

    /// Wait for a specific kind of input, including inputs that have
    /// already happened. Each call to this function will "take" one
    /// instance of the keypress.
    pub fn wait_for(input_kind: Input) -> Result<(), KillSignal> {
        todo!()
    }

    /// "Takes" one of the inputs that already exists. It is an option
    /// to take all of the inputs (an example is someone pressing
    /// a button 8 times, but we only care about the rest being
    /// buffered). Returns Some(total_taken) if an input was found.
    ///
    /// Returns Ok(None) if none of that input was found
    pub fn take_input(
        take_total: bool,
    ) -> Result<Option<u8>, KillSignal> {
        todo!()
    }

    /// Returns true if there are inputs that are able to be taken.
    pub fn inputs_available() -> Result<bool, KillSignal> {
        todo!()
    }

    /// Returns Ok(()) if there is no kill signal
    pub fn check_kill_signal() -> Result<(), KillSignal> {
        todo!()
    }

    /// Clears all signals and flags.
    pub fn clear() {
        todo!()
    }
}
