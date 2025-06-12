struct ParticlePhysics {
    pos: vec2f,
    old_pos: vec2f,
    accel: vec2f,
};

struct Uniform {
    window_size_px: f32,
    particle_radius_px: f32,
    num_particles: u32, // THIS IS ZERO AND UNUSED!!
    current_dispatch: u32,
    dispatch_metadata: array<vec4<u32>, 9>, // [offset, len, useless, useless]. vec4 for alignment reasons
};


@group(0) @binding(0) var<storage, read_write> particles: array<ParticlePhysics>;

@group(1) @binding(0) var<uniform> sim_options: Uniform;

@group(2) @binding(0) var<storage, read> dispatches: array<u32>;

@group(3) @binding(0) var<storage, read> bin_indices: array<u32>;
@group(3) @binding(1) var<storage, read> bin_particles: array<u32>;

fn get_bin(id: u32) -> u32 {

    let dispatch_number: u32 = sim_options.current_dispatch;

    let dispatch_metadata: vec4<u32> = sim_options.dispatch_metadata[dispatch_number];
    // check if within bounds
    if (id > dispatch_metadata.y) {
        return 0xFFFFFFFFu;
    }
    // index = init offset of the array + id
    let index = dispatch_metadata.x + id;
    return dispatches[index];
}

@compute @workgroup_size(64, 1, 1)
fn step(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let id = invocation_id.x;
    let bin = get_bin(id);

    if bin != 0xFFFFFFFFu {
        let start = bin_indices[bin];
        let end = bin_indices[bin + 1];
        var i = start;
        for (; i < end; i++) {
            let particle_id = bin_particles[i];
            var particle = particles[particle_id];
            particle.pos.y -= 1.0;
            particles[particle_id] = particle;
        }

    }
        // var particle = particles[invocation_id.x];
        // particle.pos.y -= 1.0;
        // particles[invocation_id.x] = particle;
}
