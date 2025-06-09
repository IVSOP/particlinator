use std::f32::consts::PI;

use bevy_math::Vec2;

use crate::common::*;

// FIXME: start passing in args like radius inside the enum iself

pub struct Spawner {
    // data to be used outside the spawner, by the app itself
    pub start_frame: u64,
    pub end_frame: u64,
    
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
        }
    }

    pub fn circle_spawn(&self, frame: u64) -> Option<ParticlePhysics> {
        if frame % 100 == 0 {
            let rad = (frame as f32 / 180.0) * PI;

            Some(ParticlePhysics {
                pos: self.pos,
                old_pos: self.pos,
                accel: Vec2::new(
                    rad.cos() * 10000.0,
                    rad.sin() * 10000.0,
                )
            })
        } else {
            None
        }
    }

    pub fn circumference_spawn(&self, frame: u64) -> Option<ParticlePhysics> {
        if frame % 100 == 0 {

            const RADIUS: f32 = 480.0;

            let rad = (frame as f32 / 180.0) * PI;

            let mut spawn_pos = Vec2::new(
                rad.cos(),
                rad.sin(),
            );
            spawn_pos *= RADIUS;
            spawn_pos += self.pos;


            let to_center = Vec2::splat(WINDOW_SIZE_X / 2.0);

            Some(ParticlePhysics {
                pos: spawn_pos,
                old_pos: spawn_pos,
                accel: to_center * 15.0
            })
        } else {
            None
        }
    }

    pub fn directional_spawn(&self, frame: u64) -> Option<ParticlePhysics> {
        if frame % 60 == 0 {
            Some(ParticlePhysics {
                pos: self.pos,
                old_pos: self.pos,
                accel: self.dir,
            })
        } else {
            None
        }
    }
}
