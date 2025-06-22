#![allow(dead_code)]

use bevy_math::Vec2;

use crate::{common::PARTICLE_DIAM, spawner::*};

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
                end_frame: 2400 + initial_delay,
                spawn_every_n: 2,
                spawner_type: SpawnerType::Directional {
                    pos: Vec2::new(11.0, y as f32),
                    dir: Vec2::new(100000.0, 0.0),
                },
            });
        }
        
        // Right side spawners (x = 989.0, negative direction)
        for y in y_positions {
            spawners.push(Spawner {
                start_frame: 60 + initial_delay,
                end_frame: 2400 + initial_delay,
                spawn_every_n: 2,
                spawner_type: SpawnerType::Directional {
                    pos: Vec2::new(989.0, y as f32),
                    dir: Vec2::new(-100000.0, 0.0),
                },
            });
        }

        // top spawners, only show up when the others stop
        let x_positions = (100..=900).step_by(25);
        for x in x_positions {
            let pos = Vec2::new(x as f32, 989.0);
            let center = Vec2::splat(500.0);
            spawners.push(Spawner {
                start_frame: 300 + initial_delay,
                end_frame: 2400 + initial_delay,
                spawn_every_n: 2,
                spawner_type: SpawnerType::Directional {
                    pos,
                    dir: 100000.0 * (center - pos).normalize(),
                },
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
                end_frame: 2350 + initial_delay,
                spawn_every_n: 2,
                spawner_type: SpawnerType::Directional {
                    pos,
                    dir: 100000.0 * (target - pos).normalize(),
                },
            });
        }
        
        // Right side spawners (x = 989.0, negative direction)
        for y in y_positions {
            let pos = Vec2::new(989.0, y as f32);
            let target = Vec2::new(500.0, 0.0);
            spawners.push(Spawner {
                start_frame: 60 + initial_delay,
                end_frame: 2350 + initial_delay,
                spawn_every_n: 2,
                spawner_type: SpawnerType::Directional {
                    pos,
                    dir: 100000.0 * (target - pos).normalize(),
                },
            });
        }

        // top spawners, only show up when the others stop
        let x_positions = (100..=900).step_by(25);
        for x in x_positions {
            let pos = Vec2::new(x as f32, 990.0);
            let target = Vec2::new(500.0, 0.0);
            spawners.push(Spawner {
                start_frame: 300 + initial_delay,
                end_frame: 2350 + initial_delay,
                spawn_every_n: 2,
                spawner_type: SpawnerType::Directional {
                    pos,
                    dir: 100000.0 * (target - pos).normalize(),
                },
            })
        }

        spawners
    }

    pub fn t2() -> Vec<Spawner> {
        let mut spawners = Vec::new();
        let initial_delay = 60 * 2;

        let num_spawners_circle = 18;
        let delay_per_spawner: u64 = 20;
        let space_per_spawner = PARTICLE_DIAM;
        let num_spawners = 20;

        let center = Vec2::new(500.0, 650.0);
        let radius = 290.0;
        for i in 0..num_spawners_circle {
            let i = i as u64;
            spawners.push(Spawner {
                start_frame: 0 + initial_delay + (i * delay_per_spawner),
                end_frame: 3400,
                spawn_every_n: 2,
                spawner_type: SpawnerType::SpinAround {
                    center,
                    dir: Vec2::NEG_Y,
                    strength: 100000.0,
                    radius: radius + 26.0,
                }
            });
        }

        for i in 0..num_spawners {
            let i = i as u64;
            spawners.push(Spawner {
                start_frame: 0 + initial_delay + (num_spawners_circle * delay_per_spawner) + (i * delay_per_spawner),
                end_frame: 3400,
                spawn_every_n: 2,
                spawner_type: SpawnerType::Directional {
                    pos: Vec2::new(11.0 + (i as f32 * space_per_spawner), 989.0),
                    dir: Vec2::ZERO,
                }
            });
        }

        for i in 0..num_spawners {
            let i = i as u64;
            spawners.push(Spawner {
                start_frame: 0 + initial_delay + (num_spawners_circle * delay_per_spawner) + (i * delay_per_spawner),
                end_frame: 3400,
                spawn_every_n: 2,
                spawner_type: SpawnerType::Directional {
                    pos: Vec2::new(989.0 - (i as f32 * space_per_spawner), 989.0),
                    dir: Vec2::ZERO,
                }
            });
        }
        
        spawners
    }

    pub fn t3() -> Vec<Spawner> {
        let mut spawners = Vec::new();
        let initial_delay = 60 * 2;
        let delay = 20;

        let num_spawners_per_dir = 20;
        let spacing = PARTICLE_DIAM;
        let strength = 100000.0;
        let max_pos = 998.0;
        let min_pos = 2.0;

        // top right, aims left
        for i in 0..num_spawners_per_dir {
            let i = i as u64;
            spawners.push(
                Spawner {
                    start_frame: initial_delay + (delay * i),
                    end_frame: 2600 - (i * delay),
                    spawn_every_n: 2,
                    spawner_type: SpawnerType::Directional {
                        pos: Vec2::new(max_pos, max_pos - (i as f32 * spacing)),
                        dir: Vec2::NEG_X * strength,
                    }
                }
            );
        }

        // top left, aims down
        for i in 0..num_spawners_per_dir {
            let i = i as u64;
            spawners.push(
                Spawner {
                    start_frame: initial_delay + (delay * i),
                    end_frame: 2600 - (i * delay),
                    spawn_every_n: 2,
                    spawner_type: SpawnerType::Directional {
                        pos: Vec2::new(min_pos + (i as f32 * spacing), max_pos),
                        dir: Vec2::NEG_Y * strength,
                    }
                }
            );
        }

        // bottom left, aims up
        for i in 0..num_spawners_per_dir {
            let i = i as u64;
            spawners.push(
                Spawner {
                    start_frame: initial_delay + (delay * i),
                    end_frame: 2600 - (i * delay),
                    spawn_every_n: 2,
                    spawner_type: SpawnerType::Directional {
                        pos: Vec2::new(min_pos, min_pos + (i as f32 * spacing)),
                        dir: Vec2::X * strength,
                    }
                }
            );
        }

        // bottom right, aims up
        for i in 0..num_spawners_per_dir {
            let i = i as u64;
            spawners.push(
                Spawner {
                    start_frame: initial_delay + (delay * i),
                    end_frame: 2600 - (i * delay),
                    spawn_every_n: 2,
                    spawner_type: SpawnerType::Directional {
                        pos: Vec2::new(max_pos - (i as f32 * spacing), min_pos),
                        dir: Vec2::Y * strength,
                    }
                }
            );
        }

        spawners
    }

    pub fn t4() -> Vec<Spawner> {
        let mut spawners = Vec::new();
        let initial_delay = 60 * 2;
        let delay = 5;
        let strength = 100000.0;

        let y_positions = (5..=995).step_by(10);

        for (i, y) in y_positions.enumerate() {
            let y = y as f32;
            let i = i as u64;
            spawners.push(
                Spawner {
                    start_frame: initial_delay + (i * delay),
                    end_frame: 1200,
                    spawn_every_n: 2,
                    spawner_type: SpawnerType::Directional {
                        pos: Vec2::new(500.0 - 0.5, y),
                        dir: Vec2::NEG_X * strength,
                    }
                }
            );
            spawners.push(
                Spawner {
                    start_frame: initial_delay + (i * delay),
                    end_frame: 1200,
                    spawn_every_n: 2,
                    spawner_type: SpawnerType::Directional {
                        pos: Vec2::new(500.0 + 0.5, y),
                        dir: Vec2::X * strength,
                    }
                }
            );
        }
        

        spawners
    }

    pub fn small() -> Vec<Spawner> {
        let mut spawners = Vec::new();
        let y_positions = (779..=980).step_by(11);
        let initial_delay = 60 * 2;
        
        // Left side spawners (x = 11.0, positive direction)
        for y in y_positions.clone() {
            spawners.push(Spawner {
                start_frame: 0 + initial_delay,
                end_frame: 1800 + initial_delay,
                spawn_every_n: 5,
                spawner_type: SpawnerType::Directional {
                    pos: Vec2::new(20.0, y as f32),
                    dir: Vec2::new(100000.0, 0.0),
                },
            });
        }
        
        // Right side spawners (x = 989.0, negative direction)
        for y in y_positions {
            spawners.push(Spawner {
                start_frame: 60 + initial_delay,
                end_frame: 1800 + initial_delay,
                spawn_every_n: 5,
                spawner_type: SpawnerType::Directional {
                    pos: Vec2::new(989.0, y as f32),
                    dir: Vec2::new(-100000.0, 0.0),
                },
            });
        }
        
        spawners
    }
}

fn generate_circle_points(center: Vec2, radius: f32, num_points: usize) -> Vec<Vec2> {
    // Validate that num_points is a power of 2 and greater than 2
    if num_points <= 2 || !num_points.is_power_of_two() {
        return Vec::new();
    }

    let mut points = Vec::with_capacity(num_points);
    let angle_step = 2.0 * std::f32::consts::PI / num_points as f32;

    for i in 0..num_points {
        let angle = i as f32 * angle_step;
        let x = center.x + radius * angle.cos();
        let y = center.y + radius * angle.sin();
        points.push(Vec2::new(x, y));
    }

    points
}
