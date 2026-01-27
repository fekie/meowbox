#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use defmt::dbg;
use defmt::debug;
use defmt::info;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

use esp_hal::clock::CpuClock;
use esp_hal::gpio::Input;
use esp_hal::gpio::InputConfig;
use esp_hal::timer::timg::TimerGroup;
use esp_println as _;

use esp_hal::i2c::master::Config as I2cConfig; // for convenience, importing as alias
use esp_hal::i2c::master::I2c;
use esp_hal::rng::Rng;
use esp_hal::time::Rate;

use micromath::F32Ext;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;

// OLED
use ssd1306::{I2CDisplayInterface, Ssd1306Async, prelude::*};

// Embedded Graphics
use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::Point,
    prelude::*,
    text::{Baseline, Text},
};

use core::f32::consts::PI;
use noise_perlin::perlin_2d;

use embassy_sync::mutex::Mutex;

const SCREEN_WIDTH: u32 = 128;
const SCREEN_HEIGHT: u32 = 64;
const FLOW_FIELD_SIZE: usize = 512; // total amount of chunks, 32 x 16
const FLOW_FORCE_MAGNITUDE_MULTIPLIER: f32 = 3.5;
const FLOW_CHUNK_SIZE: u32 = 4; // pixel size of chunks

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    defmt::info!("{:?}", info);
    loop {}
}

/// Represents a one pixel particle. Each setter will adjust the
/// position so the particle wraps to the other side of the screen.
#[derive(Debug, Default)]
struct Particle {
    x: f32,
    y: f32,
    velocity_x: f32,
    velocity_y: f32,
}

impl Particle {
    pub fn x(&self) -> f32 {
        self.x
    }

    pub fn y(&self) -> f32 {
        self.y
    }

    pub fn set_pos(&mut self, x: f32, y: f32) {
        let x_adj = x % SCREEN_WIDTH as f32;
        self.x = match x_adj.is_sign_positive() {
            true => x_adj,
            false => -x_adj,
        };

        let y_adj = y % SCREEN_HEIGHT as f32;
        self.y = match y_adj.is_sign_positive() {
            true => y_adj,
            false => -y_adj,
        };

        //self.x = x % SCREEN_WIDTH as f32;
        //self.y = y % SCREEN_HEIGHT as f32;
    }

    /// Updates its velocity according to what part
    /// of the flow field it lands on
    fn update_velocity(&mut self, flow_field: &FlowField) {
        let flow_field_x = (self.x / FLOW_CHUNK_SIZE as f32) as usize;
        let flow_field_y = (self.y / FLOW_CHUNK_SIZE as f32) as usize;
        let flow_field_index =
            (flow_field_x * (SCREEN_HEIGHT / (FLOW_CHUNK_SIZE)) as usize) + flow_field_y;
        let new_velocity_angle = flow_field.0[flow_field_index];

        self.velocity_x = new_velocity_angle.cos() * FLOW_FORCE_MAGNITUDE_MULTIPLIER;
        self.velocity_y = new_velocity_angle.sin() * FLOW_FORCE_MAGNITUDE_MULTIPLIER;
    }

    /// Updates position according to velocity
    fn update_position(&mut self) {
        self.set_pos(self.x + self.velocity_x, self.y + self.velocity_y);
    }
}

#[derive(Default)]
struct World {
    mode: Mode,
}

impl World {
    fn new() -> Self {
        World::default()
    }

    fn stop(&mut self) {
        self.mode = Mode::Stopped;
    }
}

#[derive(Default)]
enum Mode {
    #[default]
    Stopped,
    Nematode,
}

/// We have a 128x64 screen, so we
/// will do a 8x4 grid flow field, where
/// each one has an angle (each has the same magnitude).
/// This array will contain row 0 first, then row 1, etc
struct FlowField([f32; FLOW_FIELD_SIZE]);

impl FlowField {
    fn new() -> Self {
        Self([0.0; 512])
    }
}

/// Generates a value between 0.0 and 1.0
fn random(rng: &Rng) -> f32 {
    (rng.random() as u8) as f32 / 255.0
}

fn random_angle(rng: &Rng) -> f32 {
    random(rng) * 2.0 * PI
}

use esp_hal::gpio::Level;
use esp_hal::gpio::Output;
use esp_hal::gpio::OutputConfig;

use esp_hal::peripherals::Peripherals;
use esp_println::print;

use esp_rtos::embassy;

use embassy_executor::task;

use esp_hal::gpio::Pull;

//use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
//use embassy_sync::mutex::Mutex;

type ButtonType = Mutex<CriticalSectionRawMutex, Option<Input<'static>>>;
static RIGHT_BUTTON: ButtonType = Mutex::new(None);
static LEFT_BUTTON: ButtonType = Mutex::new(None);

type ButtonLEDType = Mutex<CriticalSectionRawMutex, Option<Output<'static>>>;
static RIGHT_BUTTON_LED: ButtonLEDType = Mutex::new(None);
static LEFT_BUTTON_LED: ButtonLEDType = Mutex::new(None);

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let mut rng = Rng::new();

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    info!("Embassy initialized!");

    let right_button = Input::new(
        peripherals.GPIO11,
        InputConfig::default().with_pull(Pull::Up),
    );

    let left_button = Input::new(
        peripherals.GPIO5,
        InputConfig::default().with_pull(Pull::Up),
    );

    let mut right_button_light =
        Output::new(peripherals.GPIO12, Level::High, OutputConfig::default());
    let mut left_button_light =
        Output::new(peripherals.GPIO6, Level::High, OutputConfig::default());

    // inner scope is so that once the mutex is written to, the MutexGuard is dropped, thus the
    // Mutex is released
    {
        *(RIGHT_BUTTON.lock().await) = Some(right_button);
        *(LEFT_BUTTON.lock().await) = Some(left_button);
        *(RIGHT_BUTTON_LED.lock().await) = Some(right_button_light);
        *(LEFT_BUTTON_LED.lock().await) = Some(left_button_light);
    }

    let _ = spawner.spawn(right_button_event(&RIGHT_BUTTON, &RIGHT_BUTTON_LED));
    let _ = spawner.spawn(left_button_event(&LEFT_BUTTON, &LEFT_BUTTON_LED));

    let i2c_bus = I2c::new(
        peripherals.I2C0,
        // I2cConfig is alias of esp_hal::i2c::master::I2c::Config
        I2cConfig::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_scl(peripherals.GPIO9)
    .with_sda(peripherals.GPIO10)
    .into_async();

    let interface = I2CDisplayInterface::new(i2c_bus);
    // initialize the display
    let mut display = Ssd1306Async::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().await.expect("failed to initialize display");

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    let mut particles: [Particle; 5] = [
        Particle::default(),
        Particle::default(),
        Particle::default(),
        Particle::default(),
        Particle::default(),
    ];

    particles[1].set_pos(10.0, 10.0);
    particles[2].set_pos(20.0, 20.0);
    particles[3].set_pos(30.0, 30.0);
    particles[4].set_pos(127.0, 63.0);

    display.flush().await.unwrap();

    let mut angle: f32 = 0.0;

    // We have a 128x64 screen, so we
    // will do a 8x4 grid flow field, where
    // each one has an angle (each has the same magnitude).
    // This array will contain row 0 first, then row 1, etc
    //let mut flow_field: [f32; FLOW_FIELD_SIZE] = [0.0; FLOW_FIELD_SIZE];

    let mut flow_field = FlowField::new();

    for (i, chunk) in flow_field.0.iter_mut().enumerate() {
        // a full rotation is 2pi, so we want to have each one generate
        // a bit more of a rotation than the last

        let y = i / SCREEN_WIDTH as usize;
        let x = i % SCREEN_WIDTH as usize;

        let perlin_angle = perlin_2d(x as f32 * 0.03, y as f32 * 0.03).clamp(-1.0, 1.0) * 2.0 * PI;

        *chunk = perlin_angle;
    }

    loop {
        display.clear(BinaryColor::Off).unwrap();

        for (i, particle) in particles.iter_mut().enumerate() {
            Pixel(
                Point::new(particle.x() as i32, particle.y() as i32),
                BinaryColor::On,
            )
            .draw(&mut display)
            .unwrap();

            particle.update_velocity(&flow_field);
            particle.update_position();
        }

        display.flush().await.unwrap();

        // make the angle be able to swing plus or minus pi/2
        angle += ((random(&rng) - 0.5) * 2.0) * PI / 2.0;

        for chunk in &mut flow_field.0 {
            *chunk += angle;
        }

        Timer::after(Duration::from_millis(0)).await;
    }
}

#[task]
async fn left_button_event(
    button: &'static Mutex<CriticalSectionRawMutex, Option<Input<'static>>>,
    led: &'static Mutex<CriticalSectionRawMutex, Option<Output<'static>>>,
) {
    loop {
        button.lock().await.as_mut().unwrap().wait_for_low().await;
        led.lock().await.as_mut().unwrap().set_low();
        Timer::after(Duration::from_millis(200)).await;
        led.lock().await.as_mut().unwrap().set_high();
    }
}

#[task]
async fn right_button_event(
    button: &'static Mutex<CriticalSectionRawMutex, Option<Input<'static>>>,
    led: &'static Mutex<CriticalSectionRawMutex, Option<Output<'static>>>,
) {
    loop {
        button.lock().await.as_mut().unwrap().wait_for_low().await;
        led.lock().await.as_mut().unwrap().set_low();
        Timer::after(Duration::from_millis(200)).await;
        led.lock().await.as_mut().unwrap().set_high();
    }
}
