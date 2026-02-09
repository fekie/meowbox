use core::f32::consts::PI;

use esp_hal::rng::Rng;
use micromath::F32Ext;
use static_cell::StaticCell;

pub const SCREEN_WIDTH: u32 = 128;
pub const SCREEN_HEIGHT: u32 = 64;
const FLOW_FIELD_SIZE: usize = 512; // total amount of chunks, 32 x 16
const FLOW_FORCE_MAGNITUDE_MULTIPLIER: f32 = 3.5;
const FLOW_CHUNK_SIZE: u32 = 4; // pixel size of chunks

static _PARTICLES: StaticCell<[Particle; 5]> = StaticCell::new();

pub struct PhysicsResources {
    pub particles: &'static mut [Particle; 5],
}

impl PhysicsResources {
    pub fn new() -> Self {
        Self {
            particles: _PARTICLES.init([
                Particle::default(),
                Particle::default(),
                Particle::default(),
                Particle::default(),
                Particle::default(),
            ]),
        }
    }
}

/// Represents a one pixel particle. Each setter will adjust the
/// position so the particle wraps to the other side of the screen.
#[derive(Debug, Default)]
pub struct Particle {
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
    pub fn update_velocity(&mut self, flow_field: &FlowField) {
        let flow_field_x = (self.x / FLOW_CHUNK_SIZE as f32) as usize;
        let flow_field_y = (self.y / FLOW_CHUNK_SIZE as f32) as usize;
        let flow_field_index = (flow_field_x
            * (SCREEN_HEIGHT / (FLOW_CHUNK_SIZE)) as usize)
            + flow_field_y;
        let new_velocity_angle = flow_field.0[flow_field_index];

        self.velocity_x = new_velocity_angle.cos()
            * FLOW_FORCE_MAGNITUDE_MULTIPLIER;
        self.velocity_y = new_velocity_angle.sin()
            * FLOW_FORCE_MAGNITUDE_MULTIPLIER;
    }

    /// Updates position according to velocity
    pub fn update_position(&mut self) {
        self.set_pos(
            self.x + self.velocity_x,
            self.y + self.velocity_y,
        );
    }
}

#[derive(Default)]
pub struct World {
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
pub struct FlowField(pub [f32; FLOW_FIELD_SIZE]);

impl FlowField {
    pub fn new() -> Self {
        Self([0.0; 512])
    }
}

/// Generates a value between 0.0 and 1.0
pub fn random(rng: &Rng) -> f32 {
    (rng.random() as u8) as f32 / 255.0
}

pub fn random_angle(rng: &Rng) -> f32 {
    random(rng) * 2.0 * PI
}

pub fn generate_particles() {}
