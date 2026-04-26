use embassy_executor::task;
use embassy_time::{Duration, Timer};
use esp_hal::gpio;

use crate::{input_listener, input_listener::INPUT_CHANNEL};

const BUTTON_DEBOUNCE_MS: u64 = 200;

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
