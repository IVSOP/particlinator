use bevy_math::*;
use bytemuck::{Pod, Zeroable};

pub const WINDOW_SIZE: f32 = 1000.0;
pub const PARTICLE_RADIUS: f32 = 5.0;
pub const PARTICLES_X: u32 = 100;
pub const PARTICLES_Y: u32 = 100;
pub const MAX_PARTICLES: u32 = PARTICLES_X * PARTICLES_Y * 4;
pub const FPS: f64 = 60.0;
pub const DELTA: f64 = 1.0 / FPS;
pub const DELTA_SQUARED: f64 = 1.0 / FPS;

/// The CPU-side structure that describes a single vertex of the triangle.
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec2,
    pub uv: Vec2,
}

/// The CPU-side structure that describes an instance
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct ParticleInstance {
    pub color: Vec4,
}

/// Struct sent to the SSBO to be shared with the physics compute shader
#[derive(Clone, Copy, Pod, Zeroable, Debug, Default)]
#[repr(C)]
pub struct ParticlePhysics {
    pub pos: Vec2,
    pub old_pos: Vec2,
    pub accel: Vec2,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Uniform {
    pub window_size_px: f32,
    pub particle_radius_px: f32,
}

pub static VERTICES: [Vertex; 4] = [
    Vertex {
        position: Vec2::new(-0.5, -0.5),
        uv: Vec2::new(0.0, 0.0),
    },
    Vertex {
        position: Vec2::new(0.5, -0.5),
        uv: Vec2::new(1.0, 0.0),
    },
    Vertex {
        position: Vec2::new(0.5, 0.5),
        uv: Vec2::new(1.0, 1.0),
    },
    Vertex {
        position: Vec2::new(-0.5, 0.5),
        uv: Vec2::new(0.0, 1.0),
    },
];

pub static INDICES: [u32; 6] = [0, 1, 2, 2, 3, 0];
