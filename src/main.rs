use std::{sync::Arc, time::{Duration, Instant}};

use log::*;
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalSize, Size},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::*,
};
use bevy_math::*;

mod common;
use common::*;

mod renderer;
use renderer::*;

struct App {
    state: Option<State>,
    last_frame_time: Instant,
    frame_count: u32,
    elapsed_time: f64,
}

impl Default for App {
    fn default() -> Self {
        Self {
            state: None,
            last_frame_time: Instant::now(),
            frame_count: 0,
            elapsed_time: 0.0,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create window object
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_inner_size(Size::Physical(PhysicalSize::<u32>::new(WINDOW_SIZE as u32, WINDOW_SIZE as u32)))
                        .with_resizable(false),
                )
                .unwrap(),
        );

        let state = pollster::block_on(State::new(window.clone()));
        self.state = Some(state);

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let state = self.state.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            // FIXME: I think the sleep and placement of logic is wrong
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let delta_time = now.duration_since(self.last_frame_time).as_secs_f64();
                self.last_frame_time = now;
                
                // Update frame count and elapsed time for FPS calculation
                self.frame_count += 1;
                self.elapsed_time += delta_time;

                // Print FPS every second
                if self.elapsed_time >= 1.0 {
                    let fps = self.frame_count as f64 / self.elapsed_time;
                    println!("FPS: {:.2}", fps);
                    self.frame_count = 0;
                    self.elapsed_time = 0.0;
                }

                let mut particles = state.read_particles();
                apply_gravity(&mut particles);
                basic_solver(&mut particles);
                update_position(&mut particles);
                rectangle_constraint(&mut particles);
                state.write_particles(&particles);

                state.render();
                // Emits a new redraw requested event.
                state.get_window().request_redraw();

                // Sleep to maintain target FPS
                let frame_time = now.elapsed();
                if frame_time < Duration::from_secs_f64(DELTA) {
                    std::thread::sleep(Duration::from_secs_f64(DELTA) - frame_time);
                }

            }
            WindowEvent::Resized(size) => {
                // Reconfigures the size of the surface. We do not re-render
                // here as this event is always followed up by redraw request.
                state.resize(size);
            }
            _ => (),
        }
    }
}

fn apply_gravity(
    particles: &mut [ParticlePhysics]
) {
    for particle in particles.iter_mut() {
        particle.accel.y -= 10.0;
    }
}

fn update_position(
    particles: &mut [ParticlePhysics]
) {
    for particle in particles.iter_mut() {

        let vel = particle.pos - particle.old_pos;

        particle.old_pos = particle.pos;

        let accel = particle.accel;
        particle.pos += vel + (accel * DELTA_SQUARED as f32);

        particle.accel = Vec2::ZERO;
    }
}

fn rectangle_constraint(
    particles: &mut [ParticlePhysics]
) {
    for particle in particles.iter_mut() {
        let pos = particle.pos;
        if pos.x - PARTICLE_RADIUS < 0.0 {
            particle.pos.x = PARTICLE_RADIUS;
        } else if pos.x + PARTICLE_RADIUS > WINDOW_SIZE {
            particle.pos.x = WINDOW_SIZE - PARTICLE_RADIUS;
        }
        if pos.y - PARTICLE_RADIUS < 0.0 {
            particle.pos.y = PARTICLE_RADIUS;
        } else if pos.y + PARTICLE_RADIUS > WINDOW_SIZE{
            particle.pos.y = WINDOW_SIZE - PARTICLE_RADIUS;
        }
	}
}

fn basic_solver(
    particles: &mut [ParticlePhysics]
) {
    const RESPONSE_COEF: f32 = 0.75;
    const MIN_DIST: f32 = PARTICLE_RADIUS * 2.0;
    const MIN_DIST_SQUARED: f32 = MIN_DIST * MIN_DIST;

    for i in 0..particles.len() {
        let mut particle = particles[i].clone();

        for j in 0..particles.len() {
            if j == i {
                continue;
            }

            let mut other_particle = particles[j].clone();

            let mut collision_axis_x = particle.pos.x - other_particle.pos.x;
            let mut collision_axis_y = particle.pos.y - other_particle.pos.y;

            let dist_squared = (collision_axis_x * collision_axis_x) + (collision_axis_y * collision_axis_y);

            if dist_squared < MIN_DIST_SQUARED {

                let dist = dist_squared.sqrt();
                collision_axis_x /= dist;
                collision_axis_y /= dist;

                let delta = 0.5 * 0.5 * RESPONSE_COEF * (dist - MIN_DIST);

                particle.pos.x -= collision_axis_x * (0.5 * delta);
                particle.pos.y -= collision_axis_y * (0.5 * delta);

                other_particle.pos.x += collision_axis_x * (0.5 * delta);
                other_particle.pos.y += collision_axis_y * (0.5 * delta);

                particles[i] = particle;
                particles[j] = other_particle;
            }

        }
    }
}

fn main() {
    env_logger::Builder::new()
        .filter_level(LevelFilter::Info)
        .format_timestamp_secs()
        .format_module_path(true)
        .format_level(true)
        .init();

    let event_loop = EventLoop::new().unwrap();

    // When the current loop iteration finishes, immediately begin a new
    // iteration regardless of whether or not new events are available to
    // process. Preferred for applications that want to render as fast as
    // possible, like games.
    event_loop.set_control_flow(ControlFlow::Poll);

    // When the current loop iteration finishes, suspend the thread until
    // another event arrives. Helps keeping CPU utilization low if nothing
    // is happening, which is preferred if the application might be idling in
    // the background.
    // event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
