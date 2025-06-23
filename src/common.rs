#![allow(dead_code)]

use std::time::Duration;

use bevy_math::*;
use bytemuck::{Pod, Zeroable};

// pub const PARTICLE_RADIUS: f32 = 2.0;
// pub const PARTICLES_X: u32 = 1000;
// pub const PARTICLES_Y: u32 = 1000;
pub const PARTICLE_RADIUS: f32 = 5.0;
pub const PARTICLES_X: u32 = 500;
pub const PARTICLES_Y: u32 = 500;

pub const WINDOW_SIZE_X: f32 = 1000.0;
pub const PARTICLE_DIAM: f32 = PARTICLE_RADIUS * 2.0;

pub const TOTAL_NUM_PARTICLES: usize = (PARTICLES_X * PARTICLES_Y) as usize;
pub const MAX_PARTICLES: u32 = PARTICLES_X * PARTICLES_Y; // * 2;
pub const GRID_CELL_SIZE_PARTICLE: u32 = 1; // size of the grid cell using the size of particles
pub const NUM_BINS_X: u32 = (WINDOW_SIZE_X / PARTICLE_DIAM) as u32 / GRID_CELL_SIZE_PARTICLE;
pub const NUM_BINS_WITH_PADDING: u32 = NUM_BINS_X + 2;
pub const TOTAL_NUM_BINS: usize = (NUM_BINS_X * NUM_BINS_X) as usize;
pub const TOTAL_NUM_BINS_WITH_PADDING: usize =
    (NUM_BINS_WITH_PADDING * NUM_BINS_WITH_PADDING) as usize;
pub const TOTAL_NUM_BIN_INDICES: usize = TOTAL_NUM_BINS_WITH_PADDING + 1; // not only do I need to have padding, I need 1 extra bin at the end to avoid going out of bounds trying to get the number of particles
pub const FPS: f64 = 60.0;
pub const DELTA: f64 = (1.0 / FPS) / SUBSTEPS as f64;
pub const DURATION_PER_FRAME: Duration = Duration::from_millis(((1.0 / FPS) * 1000.0) as u64);
pub const DELTA_SQUARED: f64 = DELTA * DELTA;
pub const SUBSTEPS: usize = 4;
pub const GRAVITY: f32 = -1000.0 / SUBSTEPS as f32;

pub const PARTICLES_PER_GROUP: u32 = 64; // needs to match the shader
pub const THREADS_PER_GROUP: u32 = 256; // needs to match the shader
pub const COMPUTE_GROUPS: u32 = (PARTICLES_X * PARTICLES_Y).div_ceil(THREADS_PER_GROUP);

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
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct Uniform {
    pub window_size_px: f32,
    pub particle_radius_px: f32,
    pub current_dispatch: u32,
    pub num_particles: u32,
    pub dispatch_metadata: [u32; 9 * 4], // this is actually [(u32, u32); 9], but then it has to be [(u32, u32, u32, u32); 9] due to alignment requirements, but Pod doesn't like tuples
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

/// assumes it is never at an edge
pub fn get_bin_index_above(bin: u32) -> u32 {
    bin + NUM_BINS_WITH_PADDING
}

/// assumes it is never at an edge
pub fn get_bin_index_below(bin: u32) -> u32 {
    bin - NUM_BINS_WITH_PADDING
}

pub fn get_bin_id_from_pos(pos: Vec2) -> usize {
    // this function needs to pretend the particles are one cell up and to the right
    // this will probably break if bins are not exactly the size of a cell as I am directly using the diameter of the particles and should use something else, idk what
    let offset_pos = pos + Vec2::splat(PARTICLE_DIAM);
    let grid_pos_x = (offset_pos.x / PARTICLE_DIAM) as u32 / GRID_CELL_SIZE_PARTICLE;
    let grid_pos_y = (offset_pos.y / PARTICLE_DIAM) as u32 / GRID_CELL_SIZE_PARTICLE;

    (grid_pos_x + (grid_pos_y * NUM_BINS_WITH_PADDING)) as usize
}

pub fn get_bin_index(row: u32, col: u32) -> u32 {
    col + (NUM_BINS_WITH_PADDING * row)
}

pub fn create_dispatch(row_offset: u32, col_offset: u32) -> Vec<u32> {
    let mut dispatch: Vec<u32> = Vec::new();
    let mut col = col_offset;
    let mut row = row_offset;
    loop {
        if col >= NUM_BINS_WITH_PADDING - 1 {
            row += 3;
            col = col_offset;
        }
        if row >= NUM_BINS_WITH_PADDING - 1 {
            break;
        }
        dispatch.push(get_bin_index(row, col));
        col += 3;
    }
    dispatch
}
