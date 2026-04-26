use defmt::println;
use embassy_executor::task;
use embassy_time::{Duration, Timer};
use esp_hal::gpio;

use crate::{input_listener, input_listener::INPUT_CHANNEL};

const BUTTON_DEBOUNCE_MS: u64 = 200;
const DPAD_DEBOUNCE_MS: u64 = 100;

#[task]
pub async fn button_left_listener(
    mut left_button: gpio::Input<'static>,
) {
    loop {
        left_button.wait_for_falling_edge().await;

        INPUT_CHANNEL.send(input_listener::Input::ButtonLeft).await;

        Timer::after(Duration::from_millis(BUTTON_DEBOUNCE_MS)).await;
    }
}

#[task]
pub async fn button_right_listener(
    mut right_button: gpio::Input<'static>,
) {
    loop {
        right_button.wait_for_falling_edge().await;

        INPUT_CHANNEL.send(input_listener::Input::ButtonRight).await;

        Timer::after(Duration::from_millis(BUTTON_DEBOUNCE_MS)).await;
    }
}

#[task]
pub async fn dpad_bottom_listener(
    mut dpad_bottom: gpio::Input<'static>,
) {
    loop {
        dpad_bottom.wait_for_falling_edge().await;

        INPUT_CHANNEL.send(input_listener::Input::DpadBottom).await;

        Timer::after(Duration::from_millis(DPAD_DEBOUNCE_MS)).await;
    }
}

#[task]
pub async fn dpad_top_listener(mut dpad_top: gpio::Input<'static>) {
    loop {
        dpad_top.wait_for_falling_edge().await;

        INPUT_CHANNEL.send(input_listener::Input::DpadTop).await;

        Timer::after(Duration::from_millis(DPAD_DEBOUNCE_MS)).await;
    }
}

#[task]
pub async fn dpad_left_listener(mut dpad_left: gpio::Input<'static>) {
    loop {
        dpad_left.wait_for_falling_edge().await;

        INPUT_CHANNEL.send(input_listener::Input::DpadLeft).await;

        Timer::after(Duration::from_millis(DPAD_DEBOUNCE_MS)).await;
    }
}

#[task]
pub async fn dpad_right_listener(
    mut dpad_right: gpio::Input<'static>,
) {
    loop {
        dpad_right.wait_for_falling_edge().await;

        INPUT_CHANNEL.send(input_listener::Input::DpadRight).await;

        Timer::after(Duration::from_millis(DPAD_DEBOUNCE_MS)).await;
    }
}
