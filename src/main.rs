use std::{sync::Arc, time::{Duration, Instant}};

use bytemuck::{Pod, Zeroable};
use log::*;
use wgpu::{util::DeviceExt, *};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalSize, Size},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::*,
};
use bevy_math::*;

const WINDOW_SIZE: f32 = 1000.0;
const PARTICLE_RADIUS: f32 = 5.0;
const PARTICLES_X: u32 = 100;
const PARTICLES_Y: u32 = 100;
const MAX_PARTICLES: u32 = PARTICLES_X * PARTICLES_Y * 4;
const FPS: f64 = 60.0;
const DELTA: f64 = 1.0 / FPS;
const DELTA_SQUARED: f64 = 1.0 / FPS;

struct State {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,

    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    instances_buffer: Buffer,
    current_num_particles: u32,

    _uniform_bind_group_layout: BindGroupLayout,
    uniform_bind_group: BindGroup,

    _texture_bind_group_layout: BindGroupLayout,
    texture_bind_group: BindGroup,

    _ssbo_bind_group_layout: BindGroupLayout,
    ssbo_bind_group: BindGroup,

    staging_buffer_read: Buffer,
    staging_buffer_write: Buffer,
    ssbo_buffer: Buffer,
}

/// The CPU-side structure that describes a single vertex of the triangle.
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
    position: Vec2,
    uv: Vec2,
}

/// The CPU-side structure that describes an instance
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Instance {
    color: Vec4,
}

/// Struct sent to the SSBO to be shared with the physics compute shader
#[derive(Clone, Copy, Pod, Zeroable, Debug, Default)]
#[repr(C)]
pub struct ParticlePhysics {
    pub pos: Vec2,
    pub old_pos: Vec2,
    pub accel: Vec2,
}

impl Vertex {
    // const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
    //     0 => Float32x2,
    //     1 => Float32x2,
    // ];

    fn desc() -> &'static [wgpu::VertexBufferLayout<'static>] {
        &[
            // per vertex
            VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                step_mode: VertexStepMode::Vertex,
                attributes: &[
                    // xy
                    VertexAttribute {
                        format: VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 0,
                    },
                    // uv
                    VertexAttribute {
                        format: VertexFormat::Float32x2,
                        offset: 2 * 4,
                        shader_location: 1,
                    },
                ],
            },
            // per instance
            VertexBufferLayout {
                array_stride: size_of::<Instance>() as wgpu::BufferAddress,
                step_mode: VertexStepMode::Instance,
                attributes: &[
                    // color
                    VertexAttribute {
                        format: VertexFormat::Float32x4,
                        offset: 0,
                        shader_location: 2,
                    },
                ],
            },
        ]
    }
}

static VERTICES: [Vertex; 4] = [
    Vertex {
        position: Vec2::new(-0.5, -0.5),
        uv: Vec2::new(0.0, 0.0),
    },
    Vertex {
        position: Vec2::new(0.5, -0.5),
        uv: Vec2::new(1.0, 0.0),
    },
    Vertex {
        position: Vec2::new(0.5, 0.5),
        uv: Vec2::new(1.0, 1.0),
    },
    Vertex {
        position: Vec2::new(-0.5, 0.5),
        uv: Vec2::new(0.0, 1.0),
    },
];

static INDICES: [u32; 6] = [0, 1, 2, 2, 3, 0];

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Uniform {
    window_size_px: f32,
    particle_radius_px: f32,
}

fn create_uniform_buffer(window_size_px: f32, particle_radius_px: f32, device: &Device) -> Buffer {
    let uniform = Uniform {
        window_size_px,
        particle_radius_px,
    };

    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Uniform Buffer"),
        contents: bytemuck::cast_slice(&[uniform]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    })
}

pub fn create_ssbo_buffer(particles: &[ParticlePhysics], device: &Device) -> Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("SSBO"),
        contents: bytemuck::cast_slice(&particles),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC
    })
}

pub fn create_staging_buffer_read(device: &Device) -> Buffer {
    device.create_buffer(&BufferDescriptor {
        label: Some("Staging buffer"),
        size: std::mem::size_of::<ParticlePhysics>() as u64 * MAX_PARTICLES as u64,
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

pub fn create_staging_buffer_write(device: &Device) -> Buffer {
    device.create_buffer(&BufferDescriptor {
        label: Some("Staging buffer"),
        size: std::mem::size_of::<ParticlePhysics>() as u64 * MAX_PARTICLES as u64,
        usage: BufferUsages::MAP_WRITE | BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    })
}

impl State {
    async fn new(window: Arc<Window>) -> State {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::VERTEX_WRITABLE_STORAGE,
                ..Default::default()
            })
            .await
            .unwrap();

        let size = window.inner_size();

        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];



        let particles = {
            let mut vec = Vec::new();
            for x in 0..PARTICLES_X {
                for y in 0..PARTICLES_Y {
                    let pos = Vec2::new(x as f32 * (WINDOW_SIZE / PARTICLES_X as f32), y as f32 * (WINDOW_SIZE / PARTICLES_Y as f32));
                    let particle = ParticlePhysics {
                        pos,
                        old_pos: pos,
                        accel: Vec2::new(0.0, 0.0),
                    };
                    vec.push(particle);
                }
            }
            vec
        };

        let ssbo_buffer = create_ssbo_buffer(&particles, &device);

        let ssbo_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SSBO Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: false,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let ssbo_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SSBO Bind Group"),
            layout: &ssbo_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ssbo_buffer.as_entire_binding(),
                },
            ],
        });


        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let instances = {
            let mut vec = Vec::new();
            for x in 0..PARTICLES_X {
                for y in 0..PARTICLES_Y {
                    let instance = Instance {
                        color: Vec4::new(x as f32 / PARTICLES_X as f32, y as f32 / PARTICLES_Y as f32, (x + y) as f32 / 100.0, 1.0),
                    };
                    vec.push(instance);
                }
            }
            vec
        };

        let instances_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instances Buffer"),
            contents: bytemuck::cast_slice(&instances),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../assets/shaders/simple.wgsl").into()),
        });

        let uniform_bind_group_layout: BindGroupLayout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
        });

        let uniform_buffer = create_uniform_buffer(WINDOW_SIZE, PARTICLE_RADIUS, &device);
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }
            ],
        });

        let circle_image = image::open("assets/textures/circle.png").expect("Failed to load circle.png").to_rgba8();
        let dimensions = circle_image.dimensions();

        // Create texture
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Circle Texture"),
            size: wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb, // Matches PNG RGBA format
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &circle_image,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0), // 4 bytes per pixel (RGBA)
                rows_per_image: Some(dimensions.1),
            },
            wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 1,
            },
        );

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Circle Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Texture Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Bind Group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
            ],
        });

        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Pipeline Layout"),
                    bind_group_layouts: &[
                        &ssbo_bind_group_layout,
                        &uniform_bind_group_layout,
                        &texture_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                }),
            ),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vertex".into(),
                buffers: Vertex::desc(),
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fragment".into(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None, // TODO: ????
        });

        let staging_buffer_read = create_staging_buffer_read(&device);
        let staging_buffer_write = create_staging_buffer_write(&device);

        let state = State {
            window,
            device,
            queue,
            size,
            surface,
            surface_format,

            pipeline,
            vertex_buffer,
            index_buffer,
            instances_buffer,
            current_num_particles: instances.len() as u32,

            _uniform_bind_group_layout: uniform_bind_group_layout,
            uniform_bind_group,
            _texture_bind_group_layout: texture_bind_group_layout,
            texture_bind_group,
            _ssbo_bind_group_layout: ssbo_bind_group_layout,
            ssbo_bind_group,

            staging_buffer_read,
            staging_buffer_write,
            ssbo_buffer,
        };

        // Configure surface for the first time
        state.configure_surface();

        state
    }

    fn get_window(&self) -> &Window {
        &self.window
    }

    fn configure_surface(&self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            // Request compatibility with the sRGB-format texture view we‘re going to create later.
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;

        // reconfigure the surface
        self.configure_surface();
    }

    fn render(&mut self) {
        // Create texture view
        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                // Without add_srgb_suffix() the image we will be working with
                // might not be "gamma correct".
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        // Renders a GREEN screen
        let mut encoder = self.device.create_command_encoder(&Default::default());
        // Create the renderpass which will clear the screen.
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });


        render_pass.set_pipeline(&self.pipeline);
        
        // TODO: does this send the buffer every frame?
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instances_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint32);

        render_pass.set_bind_group(0, &self.ssbo_bind_group, &[]);
        render_pass.set_bind_group(1, &self.uniform_bind_group, &[]);
        render_pass.set_bind_group(2, &self.texture_bind_group, &[]);


        render_pass.draw_indexed(0..6, 0, 0..(self.current_num_particles as u32));

        // End the render pass.
        drop(render_pass);

        // Submit the command in the queue to execute
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();
    }

    fn read_particles(&self) -> Vec<ParticlePhysics> {
        // Copy from ssbo_buffer to staging_buffer
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let bytes_to_read = std::mem::size_of::<ParticlePhysics>() * self.current_num_particles as usize;
        encoder.copy_buffer_to_buffer(
            &self.ssbo_buffer,
            0,
            &self.staging_buffer_read,
            0,
            bytes_to_read as u64,
        );
        self.queue.submit([encoder.finish()]);

        // Map staging buffer for reading
        let slice = self.staging_buffer_read.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
        self.device.poll(PollType::Wait).unwrap();
        receiver.recv().unwrap().expect("Failed to map buffer");

        // Read data
        let data = slice.get_mapped_range();
        // TODO: can I avoid reading the entire thing??
        let particle_slice = &data[0..bytes_to_read as usize];
        let particles: Vec<ParticlePhysics> = bytemuck::cast_slice(particle_slice).to_vec();
        drop(data);
        self.staging_buffer_read.unmap();
        particles
    }

    fn write_particles(&self, particles: &[ParticlePhysics]) {
        let bytes_to_write = self.current_num_particles as usize * std::mem::size_of::<ParticlePhysics>();
        // Map staging buffer for writing
        let slice = self.staging_buffer_write.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Write, move |result| {
            sender.send(result).unwrap();
        });
        self.device.poll(PollType::Wait).unwrap();
        receiver.recv().unwrap().expect("Failed to map buffer");

        // Write data to staging buffer
        let mut mapped = slice.get_mapped_range_mut();
        mapped[..bytes_to_write].copy_from_slice(&bytemuck::cast_slice(particles)[..bytes_to_write]);
        drop(mapped);
        self.staging_buffer_write.unmap();

        // Copy from staging buffer to ssbo_buffer
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(
            &self.staging_buffer_write,
            0,
            &self.ssbo_buffer,
            0,
            bytes_to_write as u64,
        );
        self.queue.submit([encoder.finish()]);
    }
}

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
