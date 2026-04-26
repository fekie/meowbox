use defmt::println;
use embassy_executor::task;
use embassy_time::{Duration, Timer};
use esp_hal::gpio;

use crate::{input_listener, input_listener::INPUT_CHANNEL};

pub const BUTTON_DEBOUNCE: Duration = Duration::from_millis(200);
pub const DPAD_DEBOUNCE: Duration = Duration::from_millis(150);

#[task]
pub async fn button_left_listener(
    mut left_button: gpio::Input<'static>,
) {
    loop {
        left_button.wait_for_falling_edge().await;

        INPUT_CHANNEL.send(input_listener::Input::ButtonLeft).await;

        Timer::after(BUTTON_DEBOUNCE).await;
    }
}

#[task]
pub async fn button_right_listener(
    mut right_button: gpio::Input<'static>,
) {
    loop {
        right_button.wait_for_falling_edge().await;

        INPUT_CHANNEL.send(input_listener::Input::ButtonRight).await;

        Timer::after(BUTTON_DEBOUNCE).await;
    }
}

#[task]
pub async fn dpad_bottom_listener(
    mut dpad_bottom: gpio::Input<'static>,
) {
    loop {
        dpad_bottom.wait_for_falling_edge().await;

        INPUT_CHANNEL.send(input_listener::Input::DpadBottom).await;

        Timer::after(DPAD_DEBOUNCE).await;
    }
}

#[task]
pub async fn dpad_top_listener(mut dpad_top: gpio::Input<'static>) {
    loop {
        dpad_top.wait_for_falling_edge().await;

        INPUT_CHANNEL.send(input_listener::Input::DpadTop).await;

        Timer::after(DPAD_DEBOUNCE).await;
    }
}

#[task]
pub async fn dpad_left_listener(mut dpad_left: gpio::Input<'static>) {
    loop {
        dpad_left.wait_for_falling_edge().await;

        INPUT_CHANNEL.send(input_listener::Input::DpadLeft).await;

        Timer::after(DPAD_DEBOUNCE).await;
    }
}

#[task]
pub async fn dpad_right_listener(
    mut dpad_right: gpio::Input<'static>,
) {
    loop {
        dpad_right.wait_for_falling_edge().await;

        INPUT_CHANNEL.send(input_listener::Input::DpadRight).await;

        Timer::after(DPAD_DEBOUNCE).await;
    }
}
