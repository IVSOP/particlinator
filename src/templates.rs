#![allow(dead_code)]

use bevy_math::Vec2;

use crate::spawner::*;

pub struct Templates;

impl Templates {
    pub fn t0() -> Vec<Spawner> {
        let mut spawners = Vec::new();
        let y_positions = (779..=989).step_by(10);
        let initial_delay = 60 * 2;
        
        // Left side spawners (x = 11.0, positive direction)
        for y in y_positions.clone() {
            spawners.push(Spawner {
                start_frame: 0 + initial_delay,
                end_frame: 2000 + initial_delay,
                spawn_every_n: 2,
                pos: Vec2::new(11.0, y as f32),
                dir: Vec2::new(100000.0, 0.0),
                spawner_type: SpawnerType::Directional,
            });
        }
        
        // Right side spawners (x = 989.0, negative direction)
        for y in y_positions {
            spawners.push(Spawner {
                start_frame: 60 + initial_delay,
                end_frame: 2000 + initial_delay,
                spawn_every_n: 2,
                pos: Vec2::new(989.0, y as f32),
                dir: Vec2::new(-100000.0, 0.0),
                spawner_type: SpawnerType::Directional,
            });
        }

        // top spawners, only show up when the others stop
        let x_positions = (100..=900).step_by(25);
        for x in x_positions {
            let pos = Vec2::new(x as f32, 989.0);
            let center = Vec2::splat(500.0);
            spawners.push(Spawner {
                start_frame: 300 + initial_delay,
                end_frame: 2000 + initial_delay,
                spawn_every_n: 2,
                pos,
                dir: 100000.0 * (center - pos).normalize(),
                spawner_type: SpawnerType::Directional,
            })
        }

        
        spawners
    }

    pub fn t1() -> Vec<Spawner> {
        let mut spawners = Vec::new();
        let y_positions = (779..=989).step_by(10);
        let initial_delay = 60 * 2;
        
        // Left side spawners (x = 11.0, positive direction)
        for y in y_positions.clone() {
            let pos = Vec2::new(11.0, y as f32);
            let target = Vec2::new(500.0, 0.0);
            spawners.push(Spawner {
                start_frame: 0 + initial_delay,
                end_frame: 2000 + initial_delay,
                spawn_every_n: 2,
                pos,
                dir: 100000.0 * (target - pos).normalize(),
                spawner_type: SpawnerType::Directional,
            });
        }
        
        // Right side spawners (x = 989.0, negative direction)
        for y in y_positions {
            let pos = Vec2::new(989.0, y as f32);
            let target = Vec2::new(500.0, 0.0);
            spawners.push(Spawner {
                start_frame: 60 + initial_delay,
                end_frame: 2000 + initial_delay,
                spawn_every_n: 2,
                pos,
                dir: 100000.0 * (target - pos).normalize(),
                spawner_type: SpawnerType::Directional,
            });
        }

        // top spawners, only show up when the others stop
        let x_positions = (100..=900).step_by(25);
        for x in x_positions {
            let pos = Vec2::new(x as f32, 990.0);
            let target = Vec2::new(500.0, 0.0);
            spawners.push(Spawner {
                start_frame: 300 + initial_delay,
                end_frame: 2000 + initial_delay,
                spawn_every_n: 2,
                pos,
                dir: 100000.0 * (target - pos).normalize(),
                spawner_type: SpawnerType::Directional,
            })
        }

        spawners
    }
}
