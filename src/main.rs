use std::{sync::Arc, thread::sleep, time::{Duration, Instant}};

use image::*;
use log::*;
use winit::{
    application::ApplicationHandler, dpi::{PhysicalSize, Size}, event::{ElementState, WindowEvent}, event_loop::{ActiveEventLoop, ControlFlow, EventLoop}, keyboard::{KeyCode, PhysicalKey}, platform::wayland::WindowAttributesExtWayland, window::*
};
use rfd::FileDialog;
use bevy_math::*;

mod common;
use common::*;

mod renderer;
use renderer::*;

mod egui;

mod spawner;
use spawner::*;

struct App {
    renderer: Option<Renderer>,
    frame_count: u64,
    lock_fps: bool,

    // index at which each bin starts
    bin_indices: Vec<u32>,
    // linearized bins containing the particles
    bin_particles: Vec<u32>,

    simulation_state: SimulationState,

    spawners: Vec<Spawner>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            renderer: None,
            frame_count: 0,
            lock_fps: true,
            bin_indices: vec![0; TOTAL_NUM_BIN_INDICES],
            bin_particles: vec![0; TOTAL_NUM_PARTICLES], // WARN: this might be too small, if needed use MAX_PARTICLES
            simulation_state: SimulationState::default(),
            spawners: vec![],
        }
    }
}

#[derive(Default, Clone)]
pub enum SimulationState {
    #[default]
    Paused,
    Running
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create window object
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_inner_size(Size::Physical(PhysicalSize::<u32>::new(WINDOW_SIZE_X as u32, WINDOW_SIZE_X as u32)))
                        .with_resizable(false)
                        .with_name("particlinator", "particlinator")
                        .with_title("particlinator"),
                )
                .unwrap(),
        );

        // let particles = _create_phys();
        let particles = create_empty_phys();
        let instances = create_instances();

        let state = pollster::block_on(
            Renderer::new(
                window.clone(),
                &instances,
                &particles,
                0
            )
        );
        self.renderer = Some(state);

        self.spawners = {
            let mut spawners = Vec::new();
            let y_positions = (10..=500).step_by(2);
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
                    dir: 10000.0 * (target - pos).normalize(),
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
                    dir: 10000.0 * (target - pos).normalize(),
                    spawner_type: SpawnerType::Directional,
                });
            }

            // top spawners, only show up when the others stop
            let x_positions = (10..=990).step_by(5);
            for x in x_positions {
                let pos = Vec2::new(x as f32, 500.0);
                let target = Vec2::new(500.0, 0.0);
                spawners.push(Spawner {
                    start_frame: 300 + initial_delay,
                    end_frame: 2000 + initial_delay,
                    spawn_every_n: 2,
                    pos,
                    dir: 10000.0 * (target - pos).normalize(),
                    spawner_type: SpawnerType::Directional,
                })
            }

            
            spawners
        };

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let renderer = self.renderer.as_mut().unwrap();

        renderer.egui_renderer.handle_input(&renderer.window, &event);

        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            // FIXME: I think the sleep and placement of logic is wrong
            WindowEvent::RedrawRequested => {
                let frame_start = Instant::now();

                if matches!(self.simulation_state, SimulationState::Running) {
                    let mut new_particles: Vec<ParticlePhysics> = vec![];
                    for spawner in self.spawners.iter_mut() {
                        if self.frame_count > spawner.start_frame && self.frame_count < spawner.end_frame {
                            if let Some(particle) = spawner.spawn(self.frame_count) {
                                new_particles.push(particle);
                            }
                        }
                    }

                    if new_particles.len() > 0 {
                        renderer.add_particles(&new_particles);
                    }

                    binning_gpu_step(renderer, &mut self.bin_indices, &mut self.bin_particles);
                    // check(&self.bin_indices, &self.bin_particles);

                    self.frame_count += 1;
                }

                let input = renderer.render(self.simulation_state.clone(), self.lock_fps);
                // Emits a new redraw requested event.
                renderer.get_window().request_redraw();

                match input {
                    None => (),
                    Some(InputEvent::Reset) => {
                        self.reset_simulation();
                    },
                    Some(InputEvent::SetColors) => {
                        let file_opt = FileDialog::new()
                            // wtf????? these don't work and result in only allowing png
                            // .add_filter("png", &["png", "PNG"])
                            // .add_filter("jpg", &["jpg", "JPG", "jpeg", "JPEG"])
                            // .set_directory("/")
                            .pick_file();

                        if let Some(file) = file_opt {
                            let image: Rgba32FImage = ImageReader::open(&file)
                                .expect(&format!("Could not open {:?}", &file))
                                .decode().expect(&format!("Could not decode {:?}", &file))
                                .resize_exact(NUM_BINS_X, NUM_BINS_X, imageops::FilterType::Triangle)
                                .flipv()
                                .into_rgba32f();

                            self.set_image(&image);
                        }
                    },
                    Some(InputEvent::PauseOrUnpause) => {
                        self.simulation_state = match self.simulation_state {
                            SimulationState::Paused => SimulationState::Running,
                            SimulationState::Running => SimulationState::Paused,
                        }
                    },
                    Some(InputEvent::LockOrUnlock) => {
                        self.lock_fps = !self.lock_fps;
                    }
                }

                if self.lock_fps {
                    let frame_end = Instant::now();
                    let frame_duration = frame_end.duration_since(frame_start);
                    if frame_duration < DURATION_PER_FRAME {
                        std::thread::sleep(DURATION_PER_FRAME - frame_duration);
                    }
                }

                let last = Instant::now();
                // TODO: only print once per second or something?
                print!("Frame lasted for {:?}\r", last.duration_since(frame_start));
            }
            WindowEvent::Resized(size) => {
                // Reconfigures the size of the surface. We do not re-render
                // here as this event is always followed up by redraw request.
                renderer.resize(size);
            }
            #[allow(unused_variables)]
            WindowEvent::KeyboardInput { device_id, event, is_synthetic } => {
                if matches!(event.physical_key, PhysicalKey::Code(KeyCode::Space)) {
                    if matches!(event.state, ElementState::Pressed) {
                        if !event.repeat {
                            let particles = self.renderer.as_ref().unwrap().read_particles();
                            println!("There are {} particles", particles.len());
                            self.renderer.as_mut().unwrap().render_menu = !self.renderer.as_mut().unwrap().render_menu;
                            self.simulation_state = SimulationState::Running;
                            self.lock_fps = true;
                            self.reset_simulation();
                        }
                    }
                }
            }
            _ => (),
        }
    }
}

impl App {
    pub fn reset_simulation(&mut self) {
        self.frame_count = 0;
        self.renderer.as_mut().unwrap().num_particles = 0;
        // let particles = create_empty_phys();
        // self.renderer.as_mut().unwrap().write_particles(&particles);
    }

    pub fn set_image(&mut self, image: &Rgba32FImage) {
        let state = self.renderer.as_mut().unwrap();
        let mut particles = state.read_particles();
        let mut instances: Vec<ParticleInstance> = Vec::with_capacity(particles.len());
        for particle in particles.iter_mut() {
            let grid_pos = (particle.pos / WINDOW_SIZE_X).clamp(Vec2::ZERO, Vec2::ONE);
            let image_color = image::imageops::sample_nearest(
                image,
                grid_pos.x as f32,
                grid_pos.y as f32
            ).expect(&format!("Failed to sample image in coordinates {:?}", grid_pos));

            instances.push(
                ParticleInstance {
                    color: Vec4::new(image_color[0], image_color[1], image_color[2], image_color[3]),
                }
            );
        }
        state.write_instances(&instances);
    }

    // TODO: put solver functions here, or at least the main functions
}

pub fn _create_phys() -> Vec<ParticlePhysics> {
    let mut vec = Vec::with_capacity(MAX_PARTICLES as usize);
    for row in 0..PARTICLES_Y {
        for col in 0..PARTICLES_X {
            let pos = Vec2::new(
                col as f32 * (WINDOW_SIZE_X / PARTICLES_X as f32) - PARTICLE_RADIUS,
                row as f32 * (WINDOW_SIZE_X / PARTICLES_Y as f32) - PARTICLE_RADIUS
            ) + Vec2::splat(PARTICLE_DIAM);
            let particle = ParticlePhysics {
                pos,
                old_pos: pos,
                accel: Vec2::ZERO,
            };
            vec.push(particle);
        }
    }
    vec.resize(MAX_PARTICLES as usize, ParticlePhysics { pos: Vec2::ZERO, old_pos: Vec2::ZERO, accel: Vec2::ZERO });
    vec
}

pub fn create_empty_phys() -> Vec<ParticlePhysics> {
    vec![
        ParticlePhysics {
            pos: Vec2::ZERO,
            old_pos: Vec2::ZERO,
            accel: Vec2::ZERO,
        };
        MAX_PARTICLES as usize
    ]
}

pub fn create_instances() -> Vec<ParticleInstance> {
    let mut vec = Vec::with_capacity(MAX_PARTICLES as usize);
    for row in 0..PARTICLES_Y {
        for col in 0..PARTICLES_X {
            let instance = ParticleInstance {
                color: Vec4::new(col as f32 / PARTICLES_X as f32, row as f32 / PARTICLES_Y as f32, (row + col) as f32 / 100.0, 1.0),
                // color: Vec4::splat(1.0),
            };
            vec.push(instance);
        }
    }
    vec.resize(MAX_PARTICLES as usize, ParticleInstance { color: Vec4::new(0.0, 1.0, 0.0, 1.0) });
    vec
}

pub fn basic_step(state: &mut Renderer) {
    let mut particles = state.read_particles();
    apply_gravity(&mut particles);
    basic_solver(&mut particles);
    update_position(&mut particles);
    rectangle_constraint(&mut particles);
    state.write_particles(&particles);
}

pub fn basic_gpu_step(state: &mut Renderer) {
    state.basic_gpu_solver();
}

pub fn binning_step(state: &mut Renderer, bin_indices: &mut Vec<u32>, bin_particles: &mut Vec<u32>) {
    let mut particles = state.read_particles();
    // why does this work fine without having a bin on the first frame? I guess all memory is 0 so it only collides cell [0]
    // create_bin(bin_indices, bin_particles, &particles);
    for _ in 0..SUBSTEPS {
        apply_gravity(&mut particles);
        update_position(&mut particles);
        rectangle_constraint(&mut particles);
        // when binning, no particle can be out of bounds
        create_bin(bin_indices, bin_particles, &particles);
        bin_solver(bin_indices, bin_particles, &mut particles);
    }
    state.write_particles(&particles);
}

pub fn count_particles_per_bin(particles_per_bin: &mut [u32], particles: &[ParticlePhysics]) {
    for particle in particles.iter() {
        let linearized_cell_index = get_bin_id_from_pos(particle.pos);
        particles_per_bin[linearized_cell_index] += 1;
    }
}

pub fn init_bins(
    // bin_indices[i] = where in bin_particles does this bin start
    bin_indices: &mut Vec<u32>,
    // bin_particles[i] = the index of some particle
    bin_particles: &mut[u32],
    // particles_per_bin[i] = number of particles in bin #i
    particles_per_bin: &[u32],
    particles: &[ParticlePhysics]
) {
    // using particles_per_bin, fill in bin_indices to indicate where each bin starts and ends
    bin_indices[0] = 0;
    for i in 1..TOTAL_NUM_BIN_INDICES {
        bin_indices[i] = bin_indices[i - 1] + particles_per_bin[i - 1];
    }

    // to keep track of how many particles I have placed in each bin, I clone the bin_indices and increment its indices, since when iterating particles I lose track of bins
    let mut bin_indices_clone = bin_indices.clone();

    // go over all particles and actually place them in the corresponding bins
    for (particle_id, particle) in particles.iter().enumerate() {
        let pos = particle.pos;
        let linearized_cell_index = get_bin_id_from_pos(pos);
        let bin_location = bin_indices_clone[linearized_cell_index] as usize;

        // the amount of particles per bin is initialized above, not here, but I have a clone for this
        bin_indices_clone[linearized_cell_index] += 1;
        bin_particles[bin_location] = particle_id as u32;
    }
}

pub fn create_bin(bin_indices: &mut Vec<u32>, bin_particles: &mut Vec<u32>, particles: &[ParticlePhysics]) {
    // FIXME: also store this array somewhere else? idk, maybe it'll come from gpu
    let mut particles_per_bin: Vec<u32> = vec![0; TOTAL_NUM_BINS_WITH_PADDING];

    count_particles_per_bin(&mut particles_per_bin, particles);
    init_bins(bin_indices, bin_particles, &particles_per_bin, particles);
}

pub fn binning_gpu_step(renderer: &mut Renderer, bin_indices: &mut Vec<u32>, bin_particles: &mut Vec<u32>) {
    // let mut particles = renderer.read_particles();
    // create_bin(bin_indices, bin_particles, &particles);

    for _ in 0..SUBSTEPS {
        renderer.gpu_update();
        let mut particles = renderer.read_particles();

        // apply_gravity(&mut particles);
        // update_position(&mut particles);
        // rectangle_constraint(&mut particles);
        
        create_bin(bin_indices, bin_particles, &particles);
        renderer.gpu_bin_solver(bin_indices, bin_particles);

        // _test_dispatches(&mut particles, bin_indices, bin_particles, compute_groups);
        // renderer.write_particles(&particles);
    }
}

fn _test_dispatches(particles: &mut [ParticlePhysics], bin_indices: &[u32], bin_particles: &[u32], compute_groups: &[u32; 9]) {
    warn!("TESTING DISPATCHES");
    let dispatches: [Vec<u32>; 9] = [
        create_dispatch(1, 1),
        create_dispatch(1, 2),
        create_dispatch(1, 3),
        create_dispatch(2, 1),
        create_dispatch(2, 2),
        create_dispatch(2, 3),
        create_dispatch(3, 1),
        create_dispatch(3, 2),
        create_dispatch(3, 3),
    ];

    let dispatch_metadata: [u32; 9 * 4] = create_dispatch_metadata(&dispatches)
        .iter()
        .flat_map(|&(a, b)| [a, b, 0, 0])
        .collect::<Vec<u32>>()
        .try_into()
        .expect("Array length mismatch");
    let flat_dispatches = dispatches.concat();

    let mut idk = 0;
    let check_bin = 309;

    for dispatch_id in 0..=8 {

        let num_workgroups = compute_groups[dispatch_id];
        let threads_per_group = THREADS_PER_GROUP;

        for group in 0..num_workgroups {
            for thread in 0..threads_per_group {
                let id = (group * threads_per_group) + thread;
                
                let dispatch_start = dispatch_metadata[dispatch_id * 4];
                let dispatch_len = dispatch_metadata[(dispatch_id * 4) + 1];
                if id < dispatch_len {
                    let bin = flat_dispatches[(dispatch_start + id) as usize];
                    if bin == check_bin {
                        idk += 1;
                    }
                    let start = bin_indices[bin as usize];
                    let end = bin_indices[(bin + 1) as usize];
                    for i in start..end {
                        let particle_id = bin_particles[i as usize];
                        particles[particle_id as usize].pos.y -= 1.0;
                    }
                }
            }
        }
    }

    warn!("Num of times {} is processed: {}", check_bin, idk);
}

fn apply_gravity(
    particles: &mut [ParticlePhysics]
) {
    for particle in particles.iter_mut() {
        particle.accel.y += GRAVITY / 12.5;
    }
}

fn update_position(particles: &mut [ParticlePhysics]) {
    const FRICTION: f32 = 0.9999; // 0.0 = max friction, 1.0 = no friction

    for particle in particles.iter_mut() {
        let vel = particle.pos - particle.old_pos;

        let damped_vel = vel * FRICTION;

        particle.old_pos = particle.pos;

        let accel = particle.accel;
        particle.pos += damped_vel + (accel * DELTA_SQUARED as f32);

        particle.accel = Vec2::ZERO;
    }
}

fn rectangle_constraint(
    particles: &mut [ParticlePhysics]
) {
    for particle in particles.iter_mut() {
        particle.pos = particle.pos.clamp(Vec2::splat(PARTICLE_RADIUS), Vec2::splat(WINDOW_SIZE_X  - PARTICLE_RADIUS));
	}
}

// FIXME: use collide() here
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

fn bin_solver(
    // bin_indices[i] = where in bin_particles does this bin start
    bin_indices: &mut [u32],
    // bin_particles[i] = the index of some particle
    bin_particles: &mut[u32],
    particles: &mut [ParticlePhysics]
) {

    // loop while ignoring the padding bins
    // starts at 1 due to padding on the left, top and bottom
    // the last valid column and row is NUM_BINS_WITH_PADDING - 2
    for bin_row in 1..=(NUM_BINS_WITH_PADDING - 2) {
        for bin_col in 1..=(NUM_BINS_WITH_PADDING - 2) {
            let bin_number = get_bin_index(bin_row, bin_col);

            // if bin_col == 2 {

                
                // collide with all the surrounding bins
                let bin_number_above = get_bin_index_above(bin_number);
                collide_bins(bin_number, bin_number_above - 1, bin_indices, bin_particles, particles);
                collide_bins(bin_number, bin_number_above, bin_indices, bin_particles, particles);
                collide_bins(bin_number, bin_number_above + 1, bin_indices, bin_particles, particles);
                
                collide_bins(bin_number, bin_number - 1, bin_indices, bin_particles, particles);
                collide_same_bins(bin_number, bin_indices, bin_particles, particles);
                collide_bins(bin_number, bin_number + 1, bin_indices, bin_particles, particles);
                
                let bin_number_below = get_bin_index_below(bin_number);
                collide_bins(bin_number, bin_number_below - 1, bin_indices, bin_particles, particles);
                collide_bins(bin_number, bin_number_below, bin_indices, bin_particles, particles);
                collide_bins(bin_number, bin_number_below + 1, bin_indices, bin_particles, particles);
            // }
        }
    }
}

pub fn collide_same_bins(
    bin: u32,
    bin_indices: &[u32],
    bin_particles: &[u32],
    particles: &mut [ParticlePhysics],
) {
    let bin_start = bin_indices[bin as usize];
    let bin_end = bin_indices[(bin + 1) as usize];

    for p_a in bin_start..bin_end {
        let particle_a_index = bin_particles[p_a as usize].clone() as usize;
        let mut particle_a = particles[particle_a_index].clone();

        for p_b in bin_start..bin_end {

            if p_a == p_b {
                continue;
            }

            let particle_b_index = bin_particles[p_b as usize].clone() as usize;
            let mut particle_b = particles[particle_b_index].clone();

            collide(&mut particle_a, &mut particle_b);

            particles[particle_b_index] = particle_b;
        }

        particles[particle_a_index] = particle_a;
    }
}

pub fn collide_bins(
    bin_a: u32,
    bin_b: u32,
    bin_indices: &[u32],
    bin_particles: &[u32],
    particles: &mut [ParticlePhysics],
) {
    let bin_start_a = bin_indices[bin_a as usize];
    let bin_end_a = bin_indices[(bin_a + 1) as usize];

    let bin_start_b = bin_indices[bin_b as usize];
    let bin_end_b = bin_indices[(bin_b + 1) as usize];

    for p_a in bin_start_a..bin_end_a {
        let particle_a_index = bin_particles[p_a as usize].clone() as usize;
        let mut particle_a = particles[particle_a_index].clone();

        for p_b in bin_start_b..bin_end_b {
            let particle_b_index = bin_particles[p_b as usize].clone() as usize;

            let mut particle_b = particles[particle_b_index].clone();

            collide(&mut particle_a, &mut particle_b);
            particles[particle_b_index] = particle_b;
        }

        particles[particle_a_index] = particle_a;
    }
}

pub fn collide(particle_a: &mut ParticlePhysics, particle_b: &mut ParticlePhysics) {
    const RESPONSE_COEF: f32 = 0.75;
    const MIN_DIST: f32 = PARTICLE_RADIUS * 2.0;
    const MIN_DIST_SQUARED: f32 = MIN_DIST * MIN_DIST;
    // FIXME: make this into a flag, it's not really needed
    const AVOID_NAN: f32 = 0.0001;

    let mut collision_axis_x = particle_a.pos.x - particle_b.pos.x;
    let mut collision_axis_y = particle_a.pos.y - particle_b.pos.y;

    let dist_squared = (collision_axis_x * collision_axis_x) + (collision_axis_y * collision_axis_y);

    if dist_squared < MIN_DIST_SQUARED && dist_squared > AVOID_NAN {

        let dist = dist_squared.sqrt();
        collision_axis_x /= dist;
        collision_axis_y /= dist;

        let delta = 0.5 * 0.5 * RESPONSE_COEF * (dist - MIN_DIST);

        particle_a.pos.x -= collision_axis_x * (0.5 * delta);
        particle_a.pos.y -= collision_axis_y * (0.5 * delta);

        particle_b.pos.x += collision_axis_x * (0.5 * delta);
        particle_b.pos.y += collision_axis_y * (0.5 * delta);

        // let delta = RESPONSE_COEF * 0.5 * (PARTICLE_RADIUS - dist);
        // collision_axis_x = (collision_axis_x / dist) * delta;
        // collision_axis_y = (collision_axis_y / dist) * delta;


        // particle_a.pos.x -= collision_axis_x;
        // particle_a.pos.y -= collision_axis_y;

        // particle_b.pos.x += collision_axis_x;
        // particle_b.pos.y += collision_axis_y;
    }
}

/// check if any particles have the exact same position in the same bin
pub fn check(bin_indices: &[u32], bin_particles: &[u32]) {
    for bin in 0..NUM_BINS_X {
        let bin_start = bin_indices[bin as usize];
        let bin_end;
        if bin == NUM_BINS_X - 1 {
            bin_end = bin_indices.len() as u32;
        } else {
            bin_end = bin_indices[(bin + 1) as usize];
        }

        println!("Checking bin {bin}");
        for index in bin_start..bin_end {
            let particle_id = bin_particles[index as usize];
            for other_index in index+1..bin_end {
                let other_particle_id = bin_particles[other_index as usize];
                if particle_id == other_particle_id {
                    println!("Found the same particle {particle_id} on position {particle_id} of the bin")
                }
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
