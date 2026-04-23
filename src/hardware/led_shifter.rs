use adv_shift_registers::wrappers::ShifterPin;
use either::Either::Left;
use embassy_executor::task;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel,
    signal::Signal,
};
use embassy_time::Duration;
use embedded_hal::digital::OutputPin;
use esp_println::{dbg, println};

use crate::hardware::LedShifterType;

pub enum RotationalDirection {
    Clockwise,
    Counterclockwise,
}

pub enum LedCommand {
    SetAllLow,
    SetAllHigh,
    Toggle(LED),
    SetHigh(LED),
    SetLow(LED),
    /// First parameter is the amount of time between each half light
    /// move. The second is the amount of total cycles to do. Third is
    /// direction.
    CycleALl {
        half_step_time: Duration,
        cycle_amount: u8,
        direction: RotationalDirection,
    },
}

pub static LED_SHIFTER_CHANNEL: Channel<
    CriticalSectionRawMutex,
    LedCommand,
    8,
> = Channel::new();

/// Represents one of the available LEDs on the board.
#[repr(usize)]
pub enum LED {
    // These are numbered 0-15. If the value is over 8, the next
    // shift register will be selected and the selection bit
    // will be dropped  by 8
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

// A wrapper over a pin so that it is possible to save state.
struct PinWrapper {
    shifter_pin: ShifterPin,
    value: bool,
}

impl PinWrapper {
    fn set_high(&mut self) {
        self.value = true;
        let _ = self.shifter_pin.set_high();
    }

    fn set_low(&mut self) {
        self.value = false;
        let _ = self.shifter_pin.set_low();
    }
}

#[task]
pub async fn led_shifter_listener(mut led_shifter: LedShifterType) {
    // array where the index of the LED (defined by the LED enum)
    let mut led_array: [PinWrapper; 16] = [
        PinWrapper {
            shifter_pin: led_shifter.get_pin_mut(0, 0, true),
            value: false,
        },
        PinWrapper {
            shifter_pin: led_shifter.get_pin_mut(0, 1, true),
            value: false,
        },
        PinWrapper {
            shifter_pin: led_shifter.get_pin_mut(0, 2, true),
            value: false,
        },
        PinWrapper {
            shifter_pin: led_shifter.get_pin_mut(0, 3, true),
            value: false,
        },
        PinWrapper {
            shifter_pin: led_shifter.get_pin_mut(0, 4, true),
            value: false,
        },
        PinWrapper {
            shifter_pin: led_shifter.get_pin_mut(0, 5, true),
            value: false,
        },
        PinWrapper {
            shifter_pin: led_shifter.get_pin_mut(0, 6, true),
            value: false,
        },
        PinWrapper {
            shifter_pin: led_shifter.get_pin_mut(0, 7, true),
            value: false,
        },
        PinWrapper {
            shifter_pin: led_shifter.get_pin_mut(1, 0, true),
            value: false,
        },
        PinWrapper {
            shifter_pin: led_shifter.get_pin_mut(1, 1, true),
            value: false,
        },
        PinWrapper {
            shifter_pin: led_shifter.get_pin_mut(1, 2, true),
            value: false,
        },
        PinWrapper {
            shifter_pin: led_shifter.get_pin_mut(1, 3, true),
            value: false,
        },
        PinWrapper {
            shifter_pin: led_shifter.get_pin_mut(1, 4, true),
            value: false,
        },
        PinWrapper {
            shifter_pin: led_shifter.get_pin_mut(1, 5, true),
            value: false,
        },
        PinWrapper {
            shifter_pin: led_shifter.get_pin_mut(1, 6, true),
            value: false,
        },
        PinWrapper {
            shifter_pin: led_shifter.get_pin_mut(1, 7, true),
            value: false,
        },
    ];

    // initialize all to low
    execute_command(&mut led_array, LedCommand::SetAllLow);

    loop {
        let cmd = LED_SHIFTER_CHANNEL.receive().await;

        //println!("but it gets the cmd");

        execute_command(&mut led_array, cmd);
    }

    // for led in LED::iter() {
    //     let i = led as usize;

    //     led_array[i] = So
    // }

    // for

    // let i = LED::Red as usize;
    // led_array[i] =
    //     Some(led_shifter.get_pin_mut(0, 2, true));

    // let mut red_led: adv_shift_registers::wrappers::ShifterPin =
    //     led_shifter.get_pin_mut(0, 2, true);

    // let mut dpad_bot_led = led_shifter.get_pin_mut(1, 2, true);

    // let mut right_big_button_led =
    //     led_shifter.get_pin_mut(1, 6, true);

    // let mut left_big_button_led = led_shifter.get_pin_mut(1, 7,
    // true);
}

fn execute_command(
    led_array: &mut [PinWrapper; 16],
    command: LedCommand,
) {
    match command {
        LedCommand::SetAllLow => {
            for i in 0..led_array.len() {
                let pin = &mut led_array[i];
                pin.set_low();
            }
        }
        LedCommand::SetAllHigh => {
            for i in 0..led_array.len() {
                let pin = &mut led_array[i];
                pin.set_high();
            }
        }
        LedCommand::Toggle(led) => {
            let pin = &mut led_array[led as usize];
            let current_value = pin.value;

            match current_value {
                true => {
                    pin.set_low();
                }
                false => {
                    pin.set_high();
                }
            };
        }
        LedCommand::SetHigh(led) => {
            let pin = &mut led_array[led as usize];
            pin.set_high();
        }
        LedCommand::SetLow(led) => {
            let pin = &mut led_array[led as usize];
            pin.set_low();
        }
        LedCommand::CycleALl {
            half_step_time: _,
            cycle_amount: _,
            direction: _,
        } => {
            todo!()
        }
    }
}
