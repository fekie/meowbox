#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use core::f32::consts::PI;

#[allow(unused_imports)]
use defmt::{error, info, warn};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_storage::{ReadStorage, Storage};
use esp_hal::{clock::CpuClock, dma::DmaDescriptor, rng::Rng};
use esp_println as _;
use esp_println::println;
use esp_storage::FlashStorage;
use meowbox::{
    hardware::{self, LEFT_BUTTON_LED, RED_LED, RIGHT_BUTTON_LED},
    states::{MenuState, Meowbox, Stage, State},
    tasks::{
        mono_display::{
            MONO_DISPLAY_CH, MonoDisplay, MonoDisplayCommand,
        },
        neopixel::NeoPixelHandle,
    },
};
use micromath::F32Ext;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    defmt::info!("{:?}", info);
    loop {}
}

use meowbox::tasks::{
    display_task, led_rotation, left_button_event,
    left_rotary_rotation_watcher, neopixel_command_listener,
    play_sequence_listener, right_button_event,
    right_rotary_rotation_watcher, rotary_switch_left_event,
    rotary_switch_right_event,
};
use static_cell::StaticCell;

// This creates a default app-descriptor required by the esp-idf
// bootloader. For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

static DESCRIPTORS: StaticCell<[DmaDescriptor; 8]> =
    StaticCell::new();
static BUFFER: StaticCell<[u8; 2048]> = StaticCell::new();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let config =
        esp_hal::Config::default().with_cpu_clock(CpuClock::_80MHz);
    let peripherals = esp_hal::init(config);

    let non_mutex_peripherals =
        hardware::init_peripherals(peripherals).await;

    let flash_addr = 0x9000;

    let mut flash = FlashStorage::new(non_mutex_peripherals.flash);

    println!("Flash size = {}", flash.capacity());

    let foo = [4, 5, 6, 12, 13];

    flash.write(flash_addr, &foo).unwrap();

    let mut meow = [0, 0, 0, 0, 0];

    flash.read(flash_addr, &mut meow).unwrap();

    let foo2 = [7, 8, 9];

    flash.write(flash_addr, &foo2).unwrap();
    flash.read(flash_addr, &mut meow).unwrap();

    println!(
        "{} {} {} {} {}",
        meow[0], meow[1], meow[2], meow[3], meow[4]
    );

    // static mut DESCRIPTORS: [DmaDescriptor; 8] =
    //     [DmaDescriptor::EMPTY; 8];
    // static mut BUFFER: [u8; 2048] = [0; 2048];

    let descriptors = DESCRIPTORS.init([DmaDescriptor::EMPTY; 8]);
    let buffer = BUFFER.init([0u8; 2048]);

    let mut tx =
        non_mutex_peripherals.i2s_speaker.i2s_tx.build(descriptors);

    let mut phase = 0.0f32;
    let sample_rate = 44_100.0;
    let freq = 440.0; // A4 tone

    // let arena = &mut Arena::new();

    // // Add some new nodes to the arena
    // let a = arena.new_node(1);
    // let b = arena.new_node(2);

    // // Append b to a
    // a.append(b, arena);
    // assert_eq!(b.ancestors(arena).into_iter().count(), 2);

    // Enable the watchdog timer and feed it for the first time
    //non_mutex_peripherals.timg1.wdt.enable();
    //non_mutex_peripherals.timg1.wdt.feed();

    //let mut display = non_mutex_peripherals.display;

    //let rng = Rng::new();

    let mono_display =
        MonoDisplay::Graphics(non_mutex_peripherals.display);

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
    ));

    let _ = spawner.spawn(rotary_switch_right_event(
        &hardware::ROTARY_SWITCH_RIGHT,
        &hardware::RIGHT_BUTTON_LED,
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

    let _ = spawner.spawn(neopixel_command_listener(
        non_mutex_peripherals.neopixel,
    ));

    // TODO: spawn this task
    let _ = spawner.spawn(display_task(mono_display));

    let neopixel_handle = NeoPixelHandle::new();
    neopixel_handle.activate_with_hb(235, 30).await;

    // wait before and after initing display, or else it competes for
    // power and stuff will fail
    Timer::after(Duration::from_millis(500)).await;

    MONO_DISPLAY_CH.send(MonoDisplayCommand::Init).await;

    //display.init().await.expect("failed to initialize display");
    // loop {
    //     match display.init().await {
    //         Ok(_) => {
    //             info!("display initialized!");
    //             break;
    //         }
    //         Err(e) => {
    //             error!("display init failed");
    //             Timer::after(Duration::from_millis(200)).await;
    //         }
    //     }
    // }

    //info!("display initialized!");
    Timer::after(Duration::from_millis(500)).await;

    // after this, turn on button leds
    LEFT_BUTTON_LED.lock().await.as_mut().unwrap().set_high();
    RIGHT_BUTTON_LED.lock().await.as_mut().unwrap().set_high();

    // let text_style = MonoTextStyleBuilder::new()
    //     .font(&FONT_6X10)
    //     .text_color(BinaryColor::On)
    //     .build();

    // let mut particles: [physics::Particle; 5] = [
    //     physics::Particle::default(),
    //     physics::Particle::default(),
    //     physics::Particle::default(),
    //     physics::Particle::default(),
    //     physics::Particle::default(),
    // ];

    // particles[1].set_pos(10.0, 10.0);
    // particles[2].set_pos(20.0, 20.0);
    // particles[3].set_pos(30.0, 30.0);
    // particles[4].set_pos(127.0, 63.0);

    // display.flush().await.unwrap();

    // let mut angle: f32 = 0.0;

    // // We have a 128x64 screen, so we
    // // will do a 8x4 grid flow field, where
    // // each one has an angle (each has the same magnitude).
    // // This array will contain row 0 first, then row 1, etc
    // let mut flow_field: [f32; physics::FLOW_FIELD_SIZE] =
    //     [0.0; physics::FLOW_FIELD_SIZE];

    // let mut flow_field = physics::FlowField::new();

    // for (i, chunk) in flow_field.0.iter_mut().enumerate() {
    //     // a full rotation is 2pi, so we want to have each one
    //     // generate a bit more of a rotation than the last

    //     let y = i / SCREEN_WIDTH as usize;
    //     let x = i % SCREEN_WIDTH as usize;

    //     let perlin_angle =
    //         perlin_2d(x as f32 * 0.03, y as f32 * 0.03)
    //             .clamp(-1.0, 1.0)
    //             * 2.0
    //             * PI;

    //     *chunk = perlin_angle;
    // }

    //let state = State::LightRing(Stage::Setup,
    // LightRingState::White);

    let state = State::Menu(Stage::Setup, MenuState::default());

    // let state = State::FlowField(
    //     Stage::Setup,
    //     meowbox::states::FlowFieldState::Fast,
    // );

    //let menu_tree = menutree::MenuTree::new();

    let mut meowbox = Meowbox::new(state);
    // turn red led back on to compensate for menu turning it off

    loop {
        for chunk in buffer.chunks_exact_mut(4) {
            let sample = (phase.sin() * 8000.0) as i16;

            // stereo
            chunk[0] = sample as u8;
            chunk[1] = (sample >> 8) as u8;
            chunk[2] = sample as u8;
            chunk[3] = (sample >> 8) as u8;

            phase += 2.0 * PI * freq / sample_rate;
            if phase > 2.0 * PI {
                phase -= 2.0 * PI;
            }
        }

        // send to I2S
        let _ = tx.write_dma(buffer).unwrap();
    }

    loop {
        meowbox.tick().await;

        // TODO: run the routine here, and after each one finishes it
        // goes and checks what the next routine is needed to
        // run

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

        //non_mutex_peripherals.simple_speaker.toggle();

        Timer::after(Duration::from_millis(1)).await;
    }
}
