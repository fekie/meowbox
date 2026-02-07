#[allow(unused_imports)]
use defmt::{error, info, warn};
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;

use crate::{
    hardware::{BLUE_LED, GREEN_LED, RED_LED, WHITE_LED, YELLOW_LED},
    physics,
    states::{ErrorStateType, LightRingState, Meowbox, Stage, State},
    tasks::all_leds_off,
};

//static CELL: StaticCell<u32> = StaticCell::new();

impl Meowbox {
    pub(super) async fn tick_flow_field(&mut self) {
        if let State::FlowField(stage, _) = self.state {
            match stage {
                Stage::Setup => self.setup_flow_field().await,
                Stage::Execution => self.execute_flow_field().await,
                Stage::Shutdown => self.shutdown_flow_field().await,
            }
        }
    }

    async fn setup_flow_field(&mut self) {
        // init positions of particles
        self.resources.particles[1].set_pos(10.0, 10.0);
        self.resources.particles[2].set_pos(20.0, 20.0);
        self.resources.particles[3].set_pos(30.0, 30.0);
        self.resources.particles[4].set_pos(127.0, 63.0);

        self.state = State::LightRing(
            Stage::Execution,
            LightRingState::default(),
        );
    }

    async fn execute_flow_field(&mut self) {
        // if let State::LightRing(_, light_ring_state) = &mut
        // self.state {
        //     match light_ring_state {
        //         LightRingState::Red => {
        //             // next state = green
        //             *light_ring_state = LightRingState::Green;

        //             // turn off all lights
        //             all_leds_off().await;

        //             // turn on green light
        //             GREEN_LED
        //                 .lock()
        //                 .await
        //                 .as_mut()
        //                 .unwrap()
        //                 .set_high();
        //         }
        //         LightRingState::Green => {
        //             // next state = green
        //             *light_ring_state = LightRingState::Blue;

        //             // turn off all lights
        //             all_leds_off().await;

        //             // turn on green light
        //             BLUE_LED
        //                 .lock()
        //                 .await
        //                 .as_mut()
        //                 .unwrap()
        //                 .set_high();
        //         }
        //         LightRingState::Blue => {
        //             // next state = yellow
        //             *light_ring_state = LightRingState::Yellow;

        //             // turn off all lights
        //             all_leds_off().await;

        //             // turn on yellow light
        //             YELLOW_LED
        //                 .lock()
        //                 .await
        //                 .as_mut()
        //                 .unwrap()
        //                 .set_high();
        //         }
        //         LightRingState::Yellow => {
        //             // next state = white
        //             *light_ring_state = LightRingState::White;

        //             // turn off all lights
        //             all_leds_off().await;

        //             // turn on white light
        //             WHITE_LED
        //                 .lock()
        //                 .await
        //                 .as_mut()
        //                 .unwrap()
        //                 .set_high();
        //         }
        //         LightRingState::White => {
        //             // next state = red
        //             *light_ring_state = LightRingState::Red;

        //             // turn off all lights
        //             all_leds_off().await;

        //             // turn on red light
        //
        // RED_LED.lock().await.as_mut().unwrap().set_high();
        //         }
        //     }
        // }

        // Timer::after(Duration::from_millis(200)).await;
    }

    async fn shutdown_flow_field(&mut self) {
        // TODO: turn all lights off
        // all_leds_off().await;

        // self.state = match self.next_state.take() {
        //     Some(x) => x,
        //     None => State::ErrorState(
        //         ErrorStateType::NextStateNotSpecified,
        //     ),
        // }
    }
}
