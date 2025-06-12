struct ParticlePhysics {
    pos: vec2f,
    old_pos: vec2f,
    accel: vec2f,
};

struct Uniform {
    window_size_px: f32,
    particle_radius_px: f32,
    num_particles: u32,
};


@group(0) @binding(0) var<storage, read_write> particles: array<ParticlePhysics>;

@group(1) @binding(0) var<uniform> sim_options: Uniform;

@compute @workgroup_size(64, 1, 1)
fn step(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    const DELTA = 1.0 / 60.0;
    const DELTA_SQUARED = DELTA * DELTA;
    const GRAVITY = -10.0;
    let radius = sim_options.particle_radius_px;
    let window = sim_options.window_size_px;
    // TODO: get this from the uniform
    let min_dist = radius * 2.0; // minimum distance between each other for there to be a collision
    let min_dist_squared = min_dist * min_dist;
    var particle = particles[invocation_id.x];

    // gravity
    particle.accel.y += GRAVITY;

    // Collide with borders. for now just clamps the position
    if (particle.pos.x - radius < 0.0) {
        particle.pos.x = radius;
    } else if (particle.pos.x + radius > window) {
        particle.pos.x = window - radius;
    }
    if (particle.pos.y - radius < 0.0) {
        particle.pos.y = radius;
    } else if (particle.pos.y + radius > window) {
        particle.pos.y = window - radius;
    }


    // collide each particle with every other particles
    let RESPONSE_COEF = 0.75;
    for (var i: u32 = 0; i < sim_options.num_particles; i++) {
        if (i == invocation_id.x) {
            // particle.vel.x += 1.0;
            continue;
        }

        var other_particle = particles[i];

        var collisionAxis_x = particle.pos.x - other_particle.pos.x;
        var collisionAxis_y = particle.pos.y - other_particle.pos.y;

        let dist_squared = (collisionAxis_x * collisionAxis_x) + (collisionAxis_y * collisionAxis_y);

        if (dist_squared < min_dist_squared) {

            let dist = sqrt(dist_squared);
            collisionAxis_x /= dist;
            collisionAxis_y /= dist;

            let delta = 0.5 * 0.5 * RESPONSE_COEF * (dist - min_dist);

            particle.pos.x -= collisionAxis_x * (0.5 * delta);
            particle.pos.y -= collisionAxis_y * (0.5 * delta);

            // other_particle.pos.x += collisionAxis_x * delta;
            // other_particle.pos.y += collisionAxis_y * delta;
            // particles[i] = other_particle;
        }
    }

    // update positions
    let vel = particle.pos - particle.old_pos;
    particle.old_pos = particle.pos;
    particle.pos += vel + (particle.accel * DELTA_SQUARED);
    particle.accel = vec2f(0.0, 0.0);

    // workgroupBarrier();


    particles[invocation_id.x] = particle;
}
