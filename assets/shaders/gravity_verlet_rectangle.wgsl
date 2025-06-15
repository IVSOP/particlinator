// this shader applies gravity, rectangle constraint, and verlet integration
// each thread processes a single particle

struct ParticlePhysics {
    pos: vec2f,
    old_pos: vec2f,
    accel: vec2f,
};

struct Uniform {
    window_size_px: f32,
    particle_radius_px: f32,
    current_dispatch: u32,
    num_particles: u32,
    dispatch_metadata: array<vec4<u32>, 9>, // [offset, len, useless, useless]. vec4 for alignment reasons
};

@group(0) @binding(0) var<storage, read_write> particles: array<ParticlePhysics>;

@group(1) @binding(0) var<uniform> sim_options: Uniform;


// FIXME: do not hardcode this
const SUBSTEPS: u32 = 4;
const GRAVITY: f32 = -1000.0 / f32(SUBSTEPS);
const WINDOW_SIZE_X: f32 = 1000.0;
const FPS: f32 = 60.0;
const DELTA: f32 = (1.0 / FPS) / f32(SUBSTEPS);
const DELTA_SQUARED: f32 = DELTA * DELTA;
const FRICTION: f32 = 0.9999; // 0.0 = max friction, 1.0 = no friction
const PARTICLE_RADIUS: f32 = 1.0;


fn gravity(particle: ptr<function, ParticlePhysics>) {
    (*particle).accel.y += GRAVITY / 12.5;
}

fn rectangle_constraint(particle: ptr<function, ParticlePhysics>) {
    (*particle).pos = clamp((*particle).pos, vec2f(PARTICLE_RADIUS, PARTICLE_RADIUS), vec2f(WINDOW_SIZE_X  - PARTICLE_RADIUS));
}

fn verlet(particle: ptr<function, ParticlePhysics>) {
    let pos = (*particle).pos;
    let old_pos = (*particle).old_pos;
    let accel = (*particle).accel;

    let vel = pos - old_pos;
    let damped_vel = vel * FRICTION;

    (*particle).old_pos = pos;
    (*particle).pos = pos + damped_vel + (accel * f32(DELTA_SQUARED));
    (*particle).accel = vec2f(0.0, 0.0);
}

@compute @workgroup_size(256, 1, 1)
fn step(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let id = invocation_id.x;

    if id < sim_options.num_particles {
        var particle: ParticlePhysics = particles[id];
        gravity(&particle);
        verlet(&particle);
        rectangle_constraint(&particle);

        particles[id] = particle;
    }
}
