use std::f32::consts::PI;

use bevy_math::Vec2;

use crate::common::*;

// FIXME: start passing in args like radius inside the enum iself

pub struct Spawner {
    // data to be used outside the spawner, by the app itself
    pub start_frame: u64,
    pub end_frame: u64,
    pub spawn_every_n: u64,
    
    // config data
    pub pos: Vec2,
    pub dir: Vec2,
    pub spawner_type: SpawnerType,
}

pub enum SpawnerType {
    Circle,
    Circumference,
    Directional,
}

impl Spawner {
    pub fn spawn(&self, frame: u64) -> Option<ParticlePhysics> {
        if frame % self.spawn_every_n == 0 {
            match self.spawner_type {
                SpawnerType::Circle => {
                    self.circle_spawn(frame)
                },
                SpawnerType::Circumference => {
                    self.circumference_spawn(frame)
                },
                SpawnerType::Directional => {
                    self.directional_spawn(frame)
                }
            }.into()
        } else {
            None
        }
    }

    pub fn circle_spawn(&self, frame: u64) -> ParticlePhysics {
        let rad = (frame as f32 / 180.0) * PI;

        ParticlePhysics {
            pos: self.pos,
            old_pos: self.pos,
            accel: Vec2::new(
                rad.cos() * 10000.0,
                rad.sin() * 10000.0,
            )
        }
    }

    pub fn circumference_spawn(&self, frame: u64) -> ParticlePhysics {
        const RADIUS: f32 = 480.0;

        let rad = (frame as f32 / 180.0) * PI;

        let mut spawn_pos = Vec2::new(
            rad.cos(),
            rad.sin(),
        );
        spawn_pos *= RADIUS;
        spawn_pos += self.pos;

        let to_center = Vec2::splat(WINDOW_SIZE_X / 2.0);

        ParticlePhysics {
            pos: spawn_pos,
            old_pos: spawn_pos,
            accel: to_center * 15.0
        }
    }

    pub fn directional_spawn(&self, _frame: u64) -> ParticlePhysics {
        ParticlePhysics {
            pos: self.pos,
            old_pos: self.pos,
            accel: self.dir,
        }
    }
}
