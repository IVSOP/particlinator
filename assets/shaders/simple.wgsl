struct Uniform {
    window_size_px: f32,
    particle_radius_px: f32,
};

struct ParticlePhysics {
    pos: vec2f,
    old_pos: vec2f,
    accel: vec2f,
};

@group(0) @binding(0) var<storage, read_write> particles: array<ParticlePhysics>;

// uniform
@group(1) @binding(0) var<uniform> sim_options: Uniform;

// texture and sampler
@group(2) @binding(0) var circleSampler: sampler;
@group(2) @binding(1) var circleTexture: texture_2d<f32>;

struct Vertex {
    @location(0) position: vec2f,
    @location(1) uv: vec2f,
};

struct Instance {
    @location(2) color: vec4f
};

// Information passed from the vertex shader to the fragment shader.
struct VertexOutput {
    // The clip-space position of the vertex.
    @builtin(position) clip_position: vec4f,
    // TODO: @location needed??????
    @location(0) uv: vec2f,
    @location(1) @interpolate(flat) color: vec4f,
};

// @vertex
// fn vertex(@builtin(instance_index) instance_id: u32, vertex: Vertex, instance: Instance) -> VertexOutput {
//     var vertex_output: VertexOutput;

//     // why the fuck is this a vec4 wtf
//     vertex_output.clip_position = vec4f(vertex.position + vec2f(0.0, f32(instance_id) / 3.0), 0.0, 1.0);
//     vertex_output.uv = vertex.uv;
//     vertex_output. color = instance.color;
//     return vertex_output;
// }

@vertex
fn vertex(@builtin(instance_index) instance_id: u32, vertex: Vertex, instance: Instance) -> VertexOutput {
    var vertex_output: VertexOutput;
    let simulation_pos = particles[instance_id].pos;

    // the input coordinates are [0, window_size_px], both in x and y
    // resize the particle
    // TODO: move it to the correct position too
    let normalized_radius = (sim_options.particle_radius_px / (sim_options.window_size_px * 0.5)) * 2.0;
    let offset_position = vertex.position + vec2f(0.5); // offset so that (0, 0) does not have the center in the corner. this is prob wrong
    let scaled_position = offset_position * normalized_radius;
    let normalized_center = (simulation_pos / sim_options.window_size_px) * 2.0 - 1.0;
    let translated_position = scaled_position + normalized_center;

    // why the fuck is this a vec4 wtf
    vertex_output.clip_position = vec4f(translated_position , 0.0, 1.0);
    vertex_output.uv = vertex.uv;
    vertex_output.color = instance.color;
    return vertex_output;
}

@fragment
fn fragment(vertex_output: VertexOutput) -> @location(0) vec4f {
    return vertex_output.color * textureSample(circleTexture, circleSampler, vertex_output.uv);
}
