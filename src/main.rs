use std::{sync::Arc, time::{Duration, Instant}};

use image::*;
use log::*;
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalSize, Size},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::*,
};
use rfd::FileDialog;
use bevy_math::*;

mod common;
use common::*;

mod renderer;
use renderer::*;

mod egui;
use egui::*;

struct App {
    state: Option<State>,
    last_frame_time: Instant,
    frame_count: u32,
    elapsed_time: f64,

    // index at which each bin starts
    bin_indices: Vec<u32>,
    // linearized bins containing the particles
    bin_particles: Vec<u32>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            state: None,
            last_frame_time: Instant::now(),
            frame_count: 0,
            elapsed_time: 0.0,
            bin_indices: vec![0; TOTAL_NUM_BINS],
            bin_particles: vec![0; TOTAL_NUM_PARTICLES],
        }
    }
}

pub enum SimulationEvents {
    Reset,
    ChangeColors,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create window object
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_inner_size(Size::Physical(PhysicalSize::<u32>::new(WINDOW_SIZE_X as u32, WINDOW_SIZE_X as u32)))
                        .with_resizable(false),
                )
                .unwrap(),
        );

        let particles = create_phys();

        let instances = create_instances();

        let state = pollster::block_on(
            State::new(
                window.clone(),
                &instances,
                &particles
            )
        );
        self.state = Some(state);

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let state = self.state.as_mut().unwrap();

        // first let egui renderer see the event

        state.egui_renderer.handle_input(&state.window, &event);

        // then the normal renderer

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

                binning_step(state, &mut self.bin_indices, &mut self.bin_particles);

                let input = state.render();
                // Emits a new redraw requested event.
                state.get_window().request_redraw();

                // Sleep to maintain target FPS
                let frame_time = now.elapsed();
                if frame_time < Duration::from_secs_f64(DELTA) {
                    std::thread::sleep(Duration::from_secs_f64(DELTA) - frame_time);
                }

                match input {
                    None => (),
                    Some(InputEvent::Reset) => {
                        self.reset_simulation();
                    },
                    Some(InputEvent::SetColors) => {
                        let file_opt = FileDialog::new()
                            .add_filter("png", &["png", "PNG"])
                            .add_filter("jpg", &["jpg", "JPG", "jpeg", "JPEG"])
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
                    }
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

impl App {
    pub fn reset_simulation(&mut self) {
        let particles = create_phys();

        self.state.as_mut().unwrap().write_particles(&particles);
    }

    pub fn set_image(&mut self, image: &Rgba32FImage) {
        let state = self.state.as_mut().unwrap();
        let mut particles = state.read_particles();
        let mut instances: Vec<ParticleInstance> = Vec::with_capacity(particles.len());
        for particle in particles.iter_mut() {
            let grid_pos = particle.pos / WINDOW_SIZE_X;
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
}

pub fn create_phys() -> Vec<ParticlePhysics> {
        let mut vec = Vec::new();
        for x in 0..PARTICLES_X {
            for y in 0..PARTICLES_Y {
                let pos = Vec2::new(x as f32 * (WINDOW_SIZE_X / PARTICLES_X as f32), y as f32 * (WINDOW_SIZE_X / PARTICLES_Y as f32));
                let particle = ParticlePhysics {
                    pos,
                    old_pos: pos,
                    accel: Vec2::new(0.0, 0.0),
                };
                vec.push(particle);
            }
        }
        vec
}

pub fn create_instances() -> Vec<ParticleInstance> {
        let mut vec = Vec::new();
        for x in 0..PARTICLES_X {
            for y in 0..PARTICLES_Y {
                let instance = ParticleInstance {
                    color: Vec4::new(x as f32 / PARTICLES_X as f32, y as f32 / PARTICLES_Y as f32, (x + y) as f32 / 100.0, 1.0),
                };
                vec.push(instance);
            }
        }
        vec
}

pub fn basic_step(state: &mut State) {
    let mut particles = state.read_particles();
    apply_gravity(&mut particles);
    basic_solver(&mut particles);
    update_position(&mut particles);
    rectangle_constraint(&mut particles);
    state.write_particles(&particles);
}

pub fn basic_gpu_step(state: &mut State) {
    state.basic_gpu_solver();
}

pub fn binning_step(state: &mut State, bin_indices: &mut Vec<u32>, bin_particles: &mut Vec<u32>) {
    let mut particles = state.read_particles();
    create_bin(bin_indices, bin_particles, &particles);
    for _ in 0..SUBSTEPS {
        apply_gravity(&mut particles);
        update_position(&mut particles);
        create_bin(bin_indices, bin_particles, &particles);
        bin_solver(bin_indices, bin_particles, &mut particles);
        rectangle_constraint(&mut particles);
    }
    state.write_particles(&particles);
}

pub fn count_particles_per_bin(particles_per_bin: &mut [u32], particles: &[ParticlePhysics]) {
    for particle in particles.iter() {
        let linearized_cell_index = get_bin_id_from_pos(particle.pos);
        particles_per_bin[linearized_cell_index] += 1;
    }

    // for i in 0..particles_per_bin.len() {
    //     println!("{}: {}", i, particles_per_bin[i]);
    // }
}

/// assumes it is never at an edge
pub fn get_bin_index_above(bin: u32) -> u32 {
    bin.wrapping_add(NUM_BINS_X)
}

/// assumes it is never at an edge
pub fn get_bin_index_below(bin: u32) -> u32 {
    bin.wrapping_sub(NUM_BINS_X)
}

pub fn get_bin_id_from_pos(pos: Vec2) -> usize {
    let grid_pos_x = (pos.x / PARTICLE_DIAM) as u32 / GRID_CELL_SIZE_PARTICLE;
    let grid_pos_y = (pos.y / PARTICLE_DIAM) as u32 / GRID_CELL_SIZE_PARTICLE;

    // FIXME: prtty sure this clamp can also be removed once I fix the bins
    let index = (grid_pos_x + (grid_pos_y * NUM_BINS_X)).clamp(0, TOTAL_NUM_BINS as u32 - 1);

    index as usize
}

pub fn init_bins(
    // bin_indices[i] = where in bin_particles does this bin start
    bin_indices: &mut [u32],
    // bin_particles[i] = the index of some particle
    bin_particles: &mut[u32],
    // particles_per_bin[i] = number of particles in bin #i
    particles_per_bin: &[u32],
    particles: &[ParticlePhysics]
) {
    // using particles_per_bin, fill in bin_indices to indicate where each bin starts and ends
    bin_indices[0] = 0;
    for i in 1..TOTAL_NUM_BINS {
        bin_indices[i] = bin_indices[i - 1] + particles_per_bin[i - 1];
    }

    // to keep track of how many particles I have placed, I'll just increment bin_indices[i] when a particle is placed in index i

    // go over all particles and actually place them in the corresponding bins
    for particle_id in 0..particles.len() {
        let pos = particles[particle_id].pos;
        let linearized_cell_index = get_bin_id_from_pos(pos);
        let bin_location = bin_indices[linearized_cell_index] as usize;

        bin_indices[linearized_cell_index] += 1;
        bin_particles[bin_location] = particle_id as u32;
    }
}

pub fn create_bin(bin_indices: &mut Vec<u32>, bin_particles: &mut Vec<u32>, particles: &[ParticlePhysics]) {
    // FIXME: also store this array somewhere else
    let mut particles_per_bin: Vec<u32> = vec![0; TOTAL_NUM_BINS];

    count_particles_per_bin(&mut particles_per_bin, particles);
    init_bins(bin_indices, bin_particles, &particles_per_bin, particles);
}

pub fn binning_gpu_step(state: &mut State) {
    // TODO: reading and writing, while using compute shaders, makes no result
    // the compute shader probably writes but then those changes don't get caught
    // if the CPU never writes, can this be an issue? I thought everything here was synchronous
}

fn apply_gravity(
    particles: &mut [ParticlePhysics]
) {
    for particle in particles.iter_mut() {
        particle.accel.y += GRAVITY;
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
        } else if pos.x + PARTICLE_RADIUS > WINDOW_SIZE_X {
            particle.pos.x = WINDOW_SIZE_X - PARTICLE_RADIUS;
        }
        if pos.y - PARTICLE_RADIUS < 0.0 {
            particle.pos.y = PARTICLE_RADIUS;
        } else if pos.y + PARTICLE_RADIUS > WINDOW_SIZE_X{
            particle.pos.y = WINDOW_SIZE_X - PARTICLE_RADIUS;
        }
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

    for bin_x in 0..NUM_BINS_X {
        for bin_y in 0..NUM_BINS_X {
            let bin_number = bin_x + (NUM_BINS_X * bin_y);

            // the edges are usually unstable, but getting the surrounding cells is an exception
            // this shouldn't be needed but without this the entire simulation just collapses for some reason
            // FIXME: increase the bins so that I surround the simulation with empty bins. need to offset all the particles' positions when binning so that [0,0] represents an impossible position out of the screen, so I can just reuse the collide_bins code
            // For now I just make collide_bins check if the position is possible

            // collide with all the surrounding bins
            let bin_number_above = get_bin_index_above(bin_number);
            if bin_number_above < TOTAL_NUM_BINS as u32 {
                collide_bins(bin_number, bin_number_above.wrapping_sub(1), bin_indices, bin_particles, particles);
                collide_bins(bin_number, bin_number_above, bin_indices, bin_particles, particles);
                collide_bins(bin_number, bin_number_above.wrapping_add(1), bin_indices, bin_particles, particles);
            }
            
            collide_bins(bin_number, bin_number.wrapping_sub(1), bin_indices, bin_particles, particles);
            collide_same_bins(bin_number, bin_indices, bin_particles, particles);
            collide_bins(bin_number, bin_number.wrapping_add(1), bin_indices, bin_particles, particles);

            let bin_number_below = get_bin_index_below(bin_number);
            if bin_number_below < TOTAL_NUM_BINS as u32 {
                collide_bins(bin_number, bin_number_below.wrapping_sub(1), bin_indices, bin_particles, particles);
                collide_bins(bin_number, bin_number_below, bin_indices, bin_particles, particles);
                collide_bins(bin_number, bin_number_below.wrapping_add(1), bin_indices, bin_particles, particles);
            }
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
    // FIXME: remove this when edge bins are fixed. when the end bin is the last (top right corner), there is no bin + 1
    // when it is 0 and you try to get the row below, the overflow will also send it over the length of the array
    let len = bin_indices.len();
    let bin_end;
    if bin + 1 >= len as u32 {
        bin_end = len as u32;
    } else {
        bin_end = bin_indices[(bin + 1) as usize];
    }
    // let bin_end = bin_indices[(bin + 1) as usize];


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
    let len = bin_indices.len() as u32;

    // FIXME: remove this when edge bins are fixed
    if bin_b >= len as u32 {
        return;
    }

    
    let bin_start_a = bin_indices[bin_a as usize];
    let bin_end_a; // = bin_indices[(bin_a + 1) as usize];
    // FIXME: remove this when edge bins are fixed
    if bin_a + 1 == len {
        bin_end_a = len;
    } else {
        bin_end_a = bin_indices[(bin_a + 1) as usize];
    }

    let bin_start_b = bin_indices[bin_b as usize];
    // TODO: this is fucked up. if bin_b is the top right (i.e., the last bin), I can't calculate it's size like this
    // When this happens, instead of distance from bin_indices[N] to bin_indices[N + 1] I have to use bin_indices[N] to bin_indices.len()
    let bin_end_b;
    if bin_b + 1 == len {
        bin_end_b = len;
    } else {
        bin_end_b = bin_indices[(bin_b + 1) as usize];
    }

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
    const AVOID_NAN: f32 = 0.0001;

    let mut collision_axis_x = particle_a.pos.x - particle_b.pos.x;
    let mut collision_axis_y = particle_a.pos.y - particle_b.pos.y;

    let dist_squared = (collision_axis_x * collision_axis_x) + (collision_axis_y * collision_axis_y);

    if dist_squared < MIN_DIST_SQUARED && dist_squared > AVOID_NAN {

        // if dist_squared == 0.0 {
        //     panic!();
        // }

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
