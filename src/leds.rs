// NOTE: Honestly I will likely change from mutexes into a
// signal-command structure.

use crate::{
    hardware::{BLUE_LED, GREEN_LED, RED_LED, WHITE_LED, YELLOW_LED},
    tasks::all_leds_off,
};

#[derive(Default, Clone, Copy, Debug)]
pub enum LightRingState {
    #[default]
    Red,
    Green,
    Blue,
    Yellow,
    White,
}

pub struct LightRing {
    state: LightRingState,
}

impl LightRing {
    pub async fn new() -> Self {
        let mut light_ring = Self {
            state: LightRingState::default(),
        };

        light_ring.display().await;

        light_ring
    }

    pub async fn forward(&mut self) {
        self.state = match self.state {
            LightRingState::Red => LightRingState::Green,
            LightRingState::Green => LightRingState::Blue,
            LightRingState::Blue => LightRingState::Yellow,
            LightRingState::Yellow => LightRingState::White,
            LightRingState::White => LightRingState::Red,
        };

        self.display().await;
    }

    pub async fn backward(&mut self) {
        self.state = match self.state {
            LightRingState::Red => LightRingState::White,
            LightRingState::Green => LightRingState::Red,
            LightRingState::Blue => LightRingState::Green,
            LightRingState::Yellow => LightRingState::Blue,
            LightRingState::White => LightRingState::Yellow,
        };

        self.display().await;
    }

    pub async fn display(&mut self) {
        match self.state {
            LightRingState::Red => {
                // turn off all leds
                all_leds_off().await;

                RED_LED.lock().await.as_mut().unwrap().set_high();
            }
            LightRingState::Green => {
                // turn off all lights
                all_leds_off().await;

                GREEN_LED.lock().await.as_mut().unwrap().set_high();
            }
            LightRingState::Blue => {
                // turn off all lights
                all_leds_off().await;

                BLUE_LED.lock().await.as_mut().unwrap().set_high();
            }
            LightRingState::Yellow => {
                // turn off all lights
                all_leds_off().await;

                // turn on white light
                YELLOW_LED.lock().await.as_mut().unwrap().set_high();
            }
            LightRingState::White => {
                // turn off all lights
                all_leds_off().await;

                // turn on red light
                WHITE_LED.lock().await.as_mut().unwrap().set_high();
            }
        }
    }
}
