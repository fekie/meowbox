use embassy_executor::task;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal,
};

use crate::hardware::LedShifterType;

pub static BUZZER_SIGNAL: Signal<CriticalSectionRawMutex, ()> =
    Signal::new();

// These are numbered 0-15. If the value is over 8, the next shift
// register will be selected and the selection bit will be dropped  by
// 8
#[repr(u8)]
pub enum LED {
    Red = 2,
    Orange = 3,
    YellowCenter = 1,
    Green = 4,
    Blue = 5,
    White = 0,
    YellowLeft = 8,
    YellowRight = 9,
    AmberLeft = 7,
    AmberRight = 6,
    ButtonLeft = 15,
    ButtonRight = 14,
    DpadLeft = 13,
    DpadBottom = 10,
    DpadTop = 12,
    DpadRight = 11,
}

#[task]
pub async fn start_led_shifter_listener(
    mut led_shifter: LedShifterType,
) {
    let mut red_led = led_shifter.get_pin_mut(0, 2, true);

    let mut dpad_bot_led = led_shifter.get_pin_mut(1, 2, true);

    let mut right_big_button_led =
        led_shifter.get_pin_mut(1, 6, true);

    let mut left_big_button_led = led_shifter.get_pin_mut(1, 7, true);
}
