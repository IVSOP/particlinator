use std::f32::consts::PI;

use bevy_math::Vec2;

use crate::common::*;

// FIXME: start passing in args like radius inside the enum iself

pub struct Spawner {
    // data to be used outside the spawner, by the app itself
    /// simulation frame where spawning starts
    pub start_frame: u64,
    /// end frame might be interpreted differently by the spawner functions and might be absolute or relative (don't ask, need to make this into an enum in the future)
    pub end_frame: u64,
    pub spawn_every_n: u64,
    
    // config data
    // pub pos: Vec2,
    // pub dir: Vec2,
    pub spawner_type: SpawnerType,
}

pub enum SpawnerType {
    Spin {
        pos: Vec2,
        strength: f32
    },
    SpinAround {
        center: Vec2,
        dir: Vec2,
        strength: f32,
        radius: f32,
    },
    Directional {
        pos: Vec2,
        // strength is already in dir
        dir: Vec2,
    },
}

impl Spawner {
    pub fn spawn(&self, frame: u64) -> Option<ParticlePhysics> {
        if frame % self.spawn_every_n == 0 {
            match self.spawner_type {
                SpawnerType::Spin{ pos, strength } => {
                    self.spin_in_place(frame, pos, strength)
                },
                SpawnerType::SpinAround{ center, dir, strength, radius } => {
                    self.spin_around(frame, center, dir, strength, radius)
                },
                SpawnerType::Directional{ pos, dir } => {
                    self.directional_spawn(pos, dir)
                }
            }.into()
        } else {
            None
        }
    }

    pub fn spin_in_place(&self, frame: u64, pos: Vec2, strength: f32) -> ParticlePhysics {
        let relative_frame = frame - self.start_frame;
        let rad = (relative_frame as f32 / 180.0) * PI;

        ParticlePhysics {
            pos,
            old_pos: pos,
            accel: Vec2::new(
                rad.cos() * strength,
                rad.sin() * strength,
            )
        }
    }

    pub fn spin_around(&self, frame: u64, center: Vec2, dir: Vec2, strength: f32, radius: f32) -> ParticlePhysics {
        let relative_frame = frame - self.start_frame;
        let rad = (relative_frame as f32 / 180.0) * PI;

        let spawn_pos = Vec2::new(
            center.x + radius * rad.cos(),
            center.y + radius * rad.sin(),
        );
        
        ParticlePhysics {
            pos: spawn_pos,
            old_pos: spawn_pos,
            accel: dir * strength
        }
    }

    pub fn directional_spawn(&self, pos: Vec2, dir: Vec2) -> ParticlePhysics {
        ParticlePhysics {
            pos,
            old_pos: pos,
            accel: dir,
        }
    }
}
