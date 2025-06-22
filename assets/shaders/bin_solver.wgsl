// this shader does the collision calculations
// it uses 9 dispatches as well as bins, for performance and determinism

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

@group(2) @binding(0) var<storage, read> dispatches: array<u32>;

@group(3) @binding(0) var<storage, read> bin_indices: array<u32>;
@group(3) @binding(1) var<storage, read> bin_particles: array<u32>;

// FIXME: do not hardcode this
const NUM_BINS_WITH_PADDING: u32 = 252;
const PARTICLE_RADIUS: f32 = 2.0;
const PARTICLE_DIAM: f32 = PARTICLE_RADIUS * PARTICLE_RADIUS;
const GRID_CELL_SIZE_PARTICLE: u32 = 1;

fn get_bin_index_above(bin: u32) -> u32 {
    return bin + NUM_BINS_WITH_PADDING;
}

fn get_bin_index_below(bin: u32) -> u32 {
    return bin - NUM_BINS_WITH_PADDING;
}

fn get_bin_id_from_pos(pos: vec2f) -> u32 {
    // this function needs to pretend the particles are one cell up and to the right
    // this will probably break if bins are not exactly the size of a cell as I am directly using the diameter of the particles and should use something else, idk what
    let offset_pos = pos + vec2f(PARTICLE_DIAM, PARTICLE_DIAM);
    let grid_pos_x = u32(offset_pos.x / PARTICLE_DIAM) / GRID_CELL_SIZE_PARTICLE;
    let grid_pos_y = u32(offset_pos.y / PARTICLE_DIAM) / GRID_CELL_SIZE_PARTICLE;

    return grid_pos_x + (grid_pos_y * NUM_BINS_WITH_PADDING);
}

fn get_bin_index(row: u32, col: u32) -> u32 {
    return col + (NUM_BINS_WITH_PADDING * row);
}

// returns the bin this thread should process
fn get_my_bin(id: u32) -> u32 {
    let dispatch_number: u32 = sim_options.current_dispatch;

    let dispatch_metadata: vec4<u32> = sim_options.dispatch_metadata[dispatch_number];
    // check if within bounds
    if (id >= dispatch_metadata.y) {
        return 0xFFFFFFFFu;
    }
    // index = init offset of the array + id
    let index = dispatch_metadata.x + id;
    return dispatches[index];
}

fn collide(particle_a: ptr<function, ParticlePhysics>, particle_b: ptr<function, ParticlePhysics>) {
    const RESPONSE_COEF: f32 = 0.5;
    const MIN_DIST: f32 = PARTICLE_DIAM;
    const MIN_DIST_SQUARED: f32 = MIN_DIST * MIN_DIST;
    const AVOID_NAN: f32 = 0.0001;

    var collision_axis: vec2f = (*particle_a).pos - (*particle_b).pos;

    let dist_squared: f32 = dot(collision_axis, collision_axis);

    if (dist_squared < MIN_DIST_SQUARED && dist_squared > AVOID_NAN) {
        let dist: f32 = sqrt(dist_squared);
        collision_axis = collision_axis / dist;

        let delta: f32 = 0.5 * RESPONSE_COEF * (dist - MIN_DIST);

        (*particle_a).pos -= collision_axis * (0.5 * delta);

        (*particle_b).pos += collision_axis * (0.5 * delta);
    }
}

fn collide_v3(particle_a: ptr<function, ParticlePhysics>, particle_b: ptr<function, ParticlePhysics>) {
    const MIN_DIST: f32 = PARTICLE_DIAM;
    const MIN_DIST_SQUARED: f32 = MIN_DIST * MIN_DIST;
    const EPS: f32 = 0.0001;
    const RESPONSE_COEF: f32 = 1.0;

    let axis: vec2f = (*particle_a).pos - (*particle_b).pos;
    let dist_squared: f32 = dot(axis, axis);

    if (dist_squared < MIN_DIST_SQUARED && dist_squared > EPS) {
        let dist: f32 = sqrt(dist_squared);
        let delta: f32 = RESPONSE_COEF * 0.5 * (MIN_DIST - dist);
        let col_vec: vec2f = (axis / dist) * delta;
        (*particle_a).pos += col_vec;
        (*particle_b).pos -= col_vec;
    }
}

fn collide_v2(particle_a: ptr<function, ParticlePhysics>, particle_b: ptr<function, ParticlePhysics>) {
    const MIN_DIST: f32 = PARTICLE_DIAM;
    const MIN_DIST_SQUARED: f32 = MIN_DIST * MIN_DIST;
    const AVOID_NAN: f32 = 0.0001;

    let axis: vec2f = (*particle_a).pos - (*particle_b).pos;
    let dist_squared: f32 = dot(axis, axis);

    if (dist_squared < MIN_DIST_SQUARED && dist_squared > AVOID_NAN) {
        let dist: f32 = sqrt(dist_squared);
        let delta: f32 = MIN_DIST - dist;
        let col_vec: vec2f = (axis / dist) * (delta * 0.5);
        let ac: f32 = 0.5;
        let bc: f32 = 0.5;
        (*particle_a).pos += col_vec * ac;
        (*particle_b).pos -= col_vec * bc;
    }
}

// FIXME: avoid cloning the particle and just get its reference??
fn collide_same_bins(bin: u32) {
    let bin_start: u32 = bin_indices[bin];
    let bin_end: u32 = bin_indices[bin + 1u];

    for (var p_a: u32 = bin_start; p_a < bin_end; p_a = p_a + 1u) {
        let particle_a_index: u32 = bin_particles[p_a];
        var particle_a: ParticlePhysics = particles[particle_a_index];

        for (var p_b: u32 = bin_start; p_b < bin_end; p_b = p_b + 1u) {
            if (p_a == p_b) {
                continue;
            }

            let particle_b_index: u32 = bin_particles[p_b];
            var particle_b: ParticlePhysics = particles[particle_b_index];

            collide(&particle_a, &particle_b);

            particles[particle_b_index] = particle_b;
        }

        particles[particle_a_index] = particle_a;
    }
}

// FIXME: avoid cloning the particle and just get its reference??
fn collide_bins(bin_a: u32, bin_b: u32) {
    let bin_start_a: u32 = bin_indices[bin_a];
    let bin_end_a: u32 = bin_indices[bin_a + 1u];

    let bin_start_b: u32 = bin_indices[bin_b];
    let bin_end_b: u32 = bin_indices[bin_b + 1u];

    for (var p_a: u32 = bin_start_a; p_a < bin_end_a; p_a = p_a + 1u) {
        let particle_a_index: u32 = bin_particles[p_a];
        var particle_a: ParticlePhysics = particles[particle_a_index];

        for (var p_b: u32 = bin_start_b; p_b < bin_end_b; p_b = p_b + 1u) {
            let particle_b_index: u32 = bin_particles[p_b];
            var particle_b: ParticlePhysics = particles[particle_b_index];

            collide(&particle_a, &particle_b);
            particles[particle_b_index] = particle_b;
        }

        particles[particle_a_index] = particle_a;
    }
}

@compute @workgroup_size(256, 1, 1)
fn step(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let id = invocation_id.x;
    let bin = get_my_bin(id);

    if bin != 0xFFFFFFFFu {
        // collide with all the surrounding bins
        let bin_above = get_bin_index_above(bin);
        collide_bins(bin, bin_above - 1);
        collide_bins(bin, bin_above);
        collide_bins(bin, bin_above + 1);

        collide_bins(bin, bin - 1);
        collide_same_bins(bin);
        collide_bins(bin, bin + 1);
                    
        let bin_below = get_bin_index_below(bin);
        collide_bins(bin, bin_below - 1);
        collide_bins(bin, bin_below);
        collide_bins(bin, bin_below + 1);
    }
}
