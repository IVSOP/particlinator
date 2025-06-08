use std::sync::Arc;

use egui_wgpu::{wgpu, ScreenDescriptor};
use wgpu::{util::DeviceExt, *};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalSize, Size},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::*,
};
use bevy_math::*;
use crate::{common::*, egui::*};

// Using egui in the current architecture for inputs is a mess so I just do this
pub enum InputEvent {
    Reset,
    SetColors,
}


impl Vertex {
    // const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
    //     0 => Float32x2,
    //     1 => Float32x2,
    // ];

    pub fn desc() -> &'static [wgpu::VertexBufferLayout<'static>] {
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
                array_stride: size_of::<ParticleInstance>() as wgpu::BufferAddress,
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

pub fn create_uniform_buffer(window_size_px: f32, particle_radius_px: f32, num_particles: u32, device: &Device) -> Buffer {
    let uniform = Uniform {
        window_size_px,
        particle_radius_px,
        num_particles
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

pub struct State {
    pub window: Arc<Window>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub surface: wgpu::Surface<'static>,
    pub surface_format: wgpu::TextureFormat,

    pub render_pipeline: RenderPipeline,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub instances_buffer: Buffer,
    pub current_num_particles: u32,

    pub _uniform_bind_group_layout: BindGroupLayout,
    pub uniform_bind_group: BindGroup,

    pub _texture_bind_group_layout: BindGroupLayout,
    pub texture_bind_group: BindGroup,

    pub _ssbo_bind_group_layout: BindGroupLayout,
    pub ssbo_bind_group: BindGroup,

    pub staging_buffer_read: Buffer,
    pub staging_buffer_write: Buffer,
    pub ssbo_buffer: Buffer,

    pub compute_pipeline: ComputePipeline,

    pub egui_renderer: EguiRenderer,
}

impl State {
    pub async fn new(
        window: Arc<Window>,
        particle_instances: &[ParticleInstance],
        particle_physics: &[ParticlePhysics]
    ) -> State {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(
                &wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    ..Default::default()
                }
            )
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::VERTEX_WRITABLE_STORAGE,
                    ..Default::default()
                },
                None
            )
            .await
            .unwrap();

        let downlevel_capabilities = adapter.get_downlevel_capabilities();
        if !downlevel_capabilities
            .flags
            .contains(wgpu::DownlevelFlags::COMPUTE_SHADERS)
        {
            panic!("Adapter does not support compute shaders");
        }

        let size = window.inner_size();

        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        //////////////////// SSBO ////////////////////

        let ssbo_buffer = create_ssbo_buffer(&particle_physics, &device);

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

        //////////////////// VBO/IBO ////////////////////

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

        let instances_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instances Buffer"),
            contents: bytemuck::cast_slice(&particle_instances),
            usage: wgpu::BufferUsages::VERTEX,
        });

        //////////////////// SHADERS ////////////////////

        let shader = device.create_shader_module(wgpu::include_wgsl!("../assets/shaders/simple.wgsl"));
        let compute_shader = device.create_shader_module(wgpu::include_wgsl!("../assets/shaders/basic_compute.wgsl"));

        //////////////////// UNIFORM ////////////////////

        let uniform_bind_group_layout: BindGroupLayout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
        });

        let uniform_buffer = create_uniform_buffer(WINDOW_SIZE_X, PARTICLE_RADIUS, particle_instances.len() as u32, &device);
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

        //////////////////// TEXTURE ////////////////////

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

        //////////////////// PIPELINES ////////////////////

        // Create render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
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

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Compute Pipeline Layout"),
                    bind_group_layouts: &[
                        &ssbo_bind_group_layout,
                        &uniform_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                }),
            ),
            module: &compute_shader,
            entry_point: Some("step"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        //////////////////// STAGING BUFFERS ////////////////////

        let staging_buffer_read = create_staging_buffer_read(&device);
        let staging_buffer_write = create_staging_buffer_write(&device);

        //////////////////// EGUI ////////////////////

        let egui_renderer = EguiRenderer::new(&device, surface_format, None, 1, &window);

        let state = State {
            window,
            device,
            queue,
            size,
            surface,
            surface_format,

            render_pipeline,
            vertex_buffer,
            index_buffer,
            instances_buffer,
            current_num_particles: particle_instances.len() as u32,

            _uniform_bind_group_layout: uniform_bind_group_layout,
            uniform_bind_group,
            _texture_bind_group_layout: texture_bind_group_layout,
            texture_bind_group,
            _ssbo_bind_group_layout: ssbo_bind_group_layout,
            ssbo_bind_group,

            staging_buffer_read,
            staging_buffer_write,
            ssbo_buffer,

            compute_pipeline,

            egui_renderer,
        };

        // Configure surface for the first time
        state.configure_surface();

        state
    }

    pub fn get_window(&self) -> &Window {
        &self.window
    }

    pub fn configure_surface(&self) {
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

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;

        // reconfigure the surface
        self.configure_surface();
    }

    pub fn render(&mut self) -> Option<InputEvent> {
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

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [WINDOW_SIZE_X as u32, WINDOW_SIZE_X as u32],
            pixels_per_point: self.window.scale_factor() as f32
                // * state.scale_factor,
        };

        let mut encoder = self.device.create_command_encoder(&Default::default());

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

        render_pass.set_pipeline(&self.render_pipeline);
        
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

        let mut input: Option<InputEvent> = None;
        {
            self.egui_renderer.begin_frame(&self.window);
            egui::Window::new("Simulation options")
                .resizable(true)
                // .vscroll(true)
                // .default_open(false)
                .show(self.egui_renderer.context(), |ui| {
                    if ui.button("Reset").clicked() {
                        input = Some(InputEvent::Reset);
                    }

                    if ui.button("Set colors").clicked() {
                        input = Some(InputEvent::SetColors);
                    }

                    // ui.separator();
                    // ui.horizontal(|ui| {
                    //     ui.label(format!(
                    //         "Pixels per point: {}",
                    //         state.egui_renderer.context().pixels_per_point()
                    //     ));
                    //     if ui.button("-").clicked() {
                    //         state.scale_factor = (state.scale_factor - 0.1).max(0.3);
                    //     }
                    //     if ui.button("+").clicked() {
                    //         state.scale_factor = (state.scale_factor + 0.1).min(3.0);
                    //     }
                    // });
                });

                self.egui_renderer.end_frame_and_draw(
                    &self.device,
                    &self.queue,
                    &mut encoder,
                    &self.window,
                    &texture_view,
                    screen_descriptor,
                );
        }

        // Submit the command in the queue to execute
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();

        input
    }

    pub fn read_particles(&self) -> Vec<ParticlePhysics> {
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
        // FIXME: change this when egui wgpu updates the underlying wgpu version
        // self.device.poll(PollType::Wait).unwrap();
        self.device.poll(MaintainBase::Wait).panic_on_timeout();
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

    pub fn write_particles(&self, particles: &[ParticlePhysics]) {
        let bytes_to_write = self.current_num_particles as usize * std::mem::size_of::<ParticlePhysics>();
        // Map staging buffer for writing
        let slice = self.staging_buffer_write.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Write, move |result| {
            sender.send(result).unwrap();
        });
        // FIXME: change this when egui wgpu updates the underlying wgpu version
        // self.device.poll(PollType::Wait).unwrap();
        self.device.poll(MaintainBase::Wait).panic_on_timeout();
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

    pub fn basic_gpu_solver(&mut self) {
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor::default());

        // Set the pipeline that we want to use
        compute_pass.set_pipeline(&self.compute_pipeline);
        // Set the bind group that we want to use
        compute_pass.set_bind_group(0, &self.ssbo_bind_group, &[]);
        compute_pass.set_bind_group(1, &self.uniform_bind_group, &[]);

        compute_pass.dispatch_workgroups(COMPUTE_GROUPS, 1, 1);
        drop(compute_pass);

        let command_buffer = encoder.finish();
        self.queue.submit([command_buffer]);
    }
}

