use core::f32::consts::PI;

#[allow(unused_imports)]
use defmt::{error, info, warn};
use embassy_time::{Duration, Timer};
use noise_perlin::perlin_2d;
use static_cell::StaticCell;

use crate::{
    hardware::{BLUE_LED, GREEN_LED, RED_LED, WHITE_LED, YELLOW_LED},
    physics::{self, SCREEN_WIDTH},
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
        let physics_resources = &mut self.resources.physics_resources;

        // init positions of particles
        physics_resources.particles[1].set_pos(10.0, 10.0);
        physics_resources.particles[2].set_pos(20.0, 20.0);
        physics_resources.particles[3].set_pos(30.0, 30.0);
        physics_resources.particles[4].set_pos(127.0, 63.0);

        for (i, chunk) in
            physics_resources.flow_field.0.iter_mut().enumerate()
        {
            // a full rotation is 2pi, so we want to have each one
            // generate a bit more of a rotation than the last

            let y = i / SCREEN_WIDTH as usize;
            let x = i % SCREEN_WIDTH as usize;

            let perlin_angle =
                perlin_2d(x as f32 * 0.03, y as f32 * 0.03)
                    .clamp(-1.0, 1.0)
                    * 2.0
                    * PI;

            *chunk = perlin_angle;
        }

        self.state = State::LightRing(
            Stage::Execution,
            LightRingState::default(),
        );
    }

    async fn execute_flow_field(&mut self) {
        // if let Err(e) = display.clear(BinaryColor::Off) {
        //     info!("error on clear");
        // }

        // for (i, particle) in particles.iter_mut().enumerate() {
        //     if let Err(e) = Pixel(
        //         Point::new(particle.x() as i32, particle.y() as
        // i32),         BinaryColor::On,
        //     )
        //     .draw(&mut display)
        //     {
        //         info!("error on draw");
        //     }

        //     particle.update_velocity(&flow_field);
        //     particle.update_position();
        // }

        // if let Err(e) = display.flush().await {
        //     info!("error on flush");
        // }

        // // make the angle be able to swing plus or minus pi/2
        // angle += ((physics::random(&rng) - 0.5) * 2.0) * PI / 2.0;

        // for chunk in &mut flow_field.0 {
        //     *chunk += angle;
        // }

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
