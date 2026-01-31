#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use defmt::error;
use defmt::info;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

use esp_hal::clock::CpuClock;
use esp_println as _;

use esp_hal::rng::Rng;

use meowbox::hardware::{self, LEFT_BUTTON_LED, RIGHT_BUTTON_LED};

use ssd1306::prelude::*;

use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::Point,
    prelude::*,
    text::{Baseline, Text},
};

use core::f32::consts::PI;
use noise_perlin::perlin_2d;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    defmt::info!("{:?}", info);
    loop {}
}

use meowbox::tasks::{
    led_rotation, left_button_event, left_rotary_rotation_watcher, play_sequence_listener,
    right_button_event, right_rotary_rotation_watcher, rotary_switch_left_event,
    rotary_switch_right_event,
};

use meowbox::physics::{self, SCREEN_WIDTH};

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let non_mutex_peripherals = hardware::init_peripherals(peripherals).await;
    let mut display = non_mutex_peripherals.display;

    let rng = Rng::new();

    info!("Embassy initialized!");

    let _ = spawner.spawn(right_button_event(
        &hardware::RIGHT_BUTTON,
        &hardware::RIGHT_BUTTON_LED,
        &hardware::BUZZER,
    ));
    let _ = spawner.spawn(left_button_event(
        &hardware::LEFT_BUTTON,
        &hardware::LEFT_BUTTON_LED,
        &hardware::BUZZER,
    ));

    let _ = spawner.spawn(rotary_switch_left_event(
        &hardware::ROTARY_SWITCH_LEFT,
        &hardware::LEFT_BUTTON_LED,
        &hardware::BUZZER,
    ));

    let _ = spawner.spawn(rotary_switch_right_event(
        &hardware::ROTARY_SWITCH_RIGHT,
        &hardware::RIGHT_BUTTON_LED,
        &hardware::BUZZER,
    ));

    let _ = spawner.spawn(play_sequence_listener(&hardware::BUZZER));

    let _ = spawner.spawn(led_rotation());

    let _ = spawner.spawn(right_rotary_rotation_watcher(
        non_mutex_peripherals.right_rotary_a,
        non_mutex_peripherals.right_rotary_b,
    ));

    let _ = spawner.spawn(left_rotary_rotation_watcher(
        non_mutex_peripherals.left_rotary_a,
        non_mutex_peripherals.left_rotary_b,
    ));

    // wait before and after initing display, or else it competes for power and stuff will fail
    Timer::after(Duration::from_millis(500)).await;
    //display.init().await.expect("failed to initialize display");
    loop {
        match display.init().await {
            Ok(_) => {
                info!("display initialized!");
                break;
            }
            Err(e) => {
                error!("display init failed");
                Timer::after(Duration::from_millis(200)).await;
            }
        }
    }

    //info!("display initialized!");
    Timer::after(Duration::from_millis(500)).await;

    // after this, turn on button leds
    LEFT_BUTTON_LED.lock().await.as_mut().unwrap().set_high();
    RIGHT_BUTTON_LED.lock().await.as_mut().unwrap().set_high();

    // let text_style = MonoTextStyleBuilder::new()
    //     .font(&FONT_6X10)
    //     .text_color(BinaryColor::On)
    //     .build();

    let mut particles: [physics::Particle; 5] = [
        physics::Particle::default(),
        physics::Particle::default(),
        physics::Particle::default(),
        physics::Particle::default(),
        physics::Particle::default(),
    ];

    particles[1].set_pos(10.0, 10.0);
    particles[2].set_pos(20.0, 20.0);
    particles[3].set_pos(30.0, 30.0);
    particles[4].set_pos(127.0, 63.0);

    //display.flush().await.unwrap();

    let mut angle: f32 = 0.0;

    // We have a 128x64 screen, so we
    // will do a 8x4 grid flow field, where
    // each one has an angle (each has the same magnitude).
    // This array will contain row 0 first, then row 1, etc
    //let mut flow_field: [f32; FLOW_FIELD_SIZE] = [0.0; FLOW_FIELD_SIZE];

    let mut flow_field = physics::FlowField::new();

    for (i, chunk) in flow_field.0.iter_mut().enumerate() {
        // a full rotation is 2pi, so we want to have each one generate
        // a bit more of a rotation than the last

        let y = i / SCREEN_WIDTH as usize;
        let x = i % SCREEN_WIDTH as usize;

        let perlin_angle = perlin_2d(x as f32 * 0.03, y as f32 * 0.03).clamp(-1.0, 1.0) * 2.0 * PI;

        *chunk = perlin_angle;
    }

    loop {
        // TODO: run the routine here, and after each one finishes it goes and checks
        // what the next routine is needed to run

        if let Err(e) = display.clear(BinaryColor::Off) {
            info!("error on clear");
        }

        for (i, particle) in particles.iter_mut().enumerate() {
            if let Err(e) = Pixel(
                Point::new(particle.x() as i32, particle.y() as i32),
                BinaryColor::On,
            )
            .draw(&mut display)
            {
                info!("error on draw");
            }

            particle.update_velocity(&flow_field);
            particle.update_position();
        }

        if let Err(e) = display.flush().await {
            info!("error on flush");
        }

        // make the angle be able to swing plus or minus pi/2
        angle += ((physics::random(&rng) - 0.5) * 2.0) * PI / 2.0;

        for chunk in &mut flow_field.0 {
            *chunk += angle;
        }

        Timer::after(Duration::from_millis(1)).await;
    }
}
