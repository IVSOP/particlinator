use std::{collections::HashMap, sync::Arc};

use crate::{SimulationState, common::*, egui::*};
use bevy_math::*;
use egui_wgpu::{ScreenDescriptor, wgpu};
use log::warn;
use wgpu::{util::DeviceExt, *};
use winit::window::*;

// Using egui in the current architecture for inputs is a mess so I just do this
pub enum InputEvent {
    Reset,
    SetColors,
    PauseOrUnpause,
    LockOrUnlock,
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

pub fn create_uniform_buffer(uniform: &Uniform, device: &Device) -> Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Uniform Buffer"),
        contents: bytemuck::cast_slice(&[uniform.clone()]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    })
}

pub fn create_uniform_staging_buffer_write(device: &Device) -> Buffer {
    device.create_buffer(&BufferDescriptor {
        label: Some("Uniform staging buffer"),
        size: std::mem::size_of::<Uniform>() as u64,
        usage: BufferUsages::MAP_WRITE | BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    })
}

pub fn create_ssbo_buffer(particles: &[ParticlePhysics], device: &Device) -> Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("SSBO"),
        contents: bytemuck::cast_slice(&particles),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
    })
}

pub fn create_particles_staging_buffer_read(device: &Device) -> Buffer {
    device.create_buffer(&BufferDescriptor {
        label: Some("Staging buffer"),
        size: std::mem::size_of::<ParticlePhysics>() as u64 * MAX_PARTICLES as u64,
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

pub fn create_particles_staging_buffer_write(device: &Device) -> Buffer {
    device.create_buffer(&BufferDescriptor {
        label: Some("Staging buffer"),
        size: std::mem::size_of::<ParticlePhysics>() as u64 * MAX_PARTICLES as u64,
        usage: BufferUsages::MAP_WRITE | BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    })
}

pub fn create_instances_staging_buffer_write(device: &Device) -> Buffer {
    device.create_buffer(&BufferDescriptor {
        label: Some("Staging buffer"),
        size: std::mem::size_of::<ParticleInstance>() as u64 * MAX_PARTICLES as u64,
        usage: BufferUsages::MAP_WRITE | BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    })
}

pub fn create_bin_indices_staging_buffer_write(device: &Device) -> Buffer {
    device.create_buffer(&BufferDescriptor {
        label: Some("Bin indices staging buffer"),
        size: std::mem::size_of::<u32>() as u64 * TOTAL_NUM_BIN_INDICES as u64,
        usage: BufferUsages::MAP_WRITE | BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    })
}

pub fn create_bin_particles_staging_buffer_write(device: &Device) -> Buffer {
    device.create_buffer(&BufferDescriptor {
        label: Some("Bin particles staging buffer"),
        size: std::mem::size_of::<u32>() as u64 * MAX_PARTICLES as u64,
        usage: BufferUsages::MAP_WRITE | BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    })
}

pub fn create_bin_indices_buffer(device: &Device) -> Buffer {
    device.create_buffer(&BufferDescriptor {
        label: Some("Bin indices buffer"),
        size: std::mem::size_of::<u32>() as u64 * TOTAL_NUM_BIN_INDICES as u64,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

pub fn create_bin_particles_buffer(device: &Device) -> Buffer {
    device.create_buffer(&BufferDescriptor {
        label: Some("Bin particles buffer"),
        size: std::mem::size_of::<u32>() as u64 * MAX_PARTICLES as u64,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

pub fn upload_dispatch(device: &Device, dispatch: &[u32]) -> Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("SSBO"),
        contents: bytemuck::cast_slice(dispatch),
        usage: BufferUsages::STORAGE,
    })
}

pub struct Renderer {
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

    // the renderer having this is cursed
    // it's hard to make a good architecture here, as the renderer also contains many buffers etc
    pub num_particles: u32,

    pub uniform_bind_group: BindGroup,

    pub texture_bind_group: BindGroup,

    pub ssbo_bind_group: BindGroup,

    pub particles_staging_buffer_read: Buffer,
    pub particles_staging_buffer_write: Buffer,
    pub instances_staging_buffer_write: Buffer,
    pub ssbo_buffer: Buffer,

    pub collision_compute_pipeline: ComputePipeline,
    pub update_compute_pipeline: ComputePipeline,
    pub dispatch_bind_group: BindGroup,
    pub uniform: Uniform,
    pub uniform_buffer: Buffer,
    pub uniform_staging_buffer_write: Buffer,
    pub compute_groups: [u32; 9], // how many workgroups needed for each step. just THREADS_PER_GROUP.div_ceil(of that dispatch)

    pub egui_renderer: EguiRenderer,

    pub bin_indices_buffer: Buffer,
    pub bin_indices_staging_buffer_write: Buffer,
    pub bin_particles_staging_buffer_write: Buffer,
    pub bin_particles_buffer: Buffer,
    pub bin_bind_group: BindGroup,

    pub render_menu: bool,
}

impl Renderer {
    pub async fn new(
        window: Arc<Window>,
        particle_instances: &[ParticleInstance],
        particle_physics: &[ParticlePhysics],
        num_particles: u32,
    ) -> Renderer {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::VERTEX_WRITABLE_STORAGE,
                    ..Default::default()
                },
                None,
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

        let info = adapter.get_info();
        warn!("Backend: {:?}", info.backend);

        let size = window.inner_size();

        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        //////////////////// SSBO ////////////////////

        let ssbo_buffer = create_ssbo_buffer(&particle_physics, &device);

        let ssbo_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("SSBO Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let ssbo_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SSBO Bind Group"),
            layout: &ssbo_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ssbo_buffer.as_entire_binding(),
            }],
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
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        //////////////////// SHADERS ////////////////////

        let shader =
            device.create_shader_module(wgpu::include_wgsl!("../assets/shaders/simple.wgsl"));
        let collision_compute_shader =
            device.create_shader_module(wgpu::include_wgsl!("../assets/shaders/bin_solver.wgsl"));
        let update_compute_shader = device.create_shader_module(wgpu::include_wgsl!(
            "../assets/shaders/gravity_verlet_rectangle.wgsl"
        ));

        //////////////////// TEXTURE ////////////////////

        let circle_image = image::open("assets/textures/circle.png")
            .expect("Failed to load circle.png")
            .to_rgba8();
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

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        //////////////////// DISPATCH BUFFERS ////////////////////
        // TODO: make this into an array of arrays instead of this mess

        // populate the buffers and send them to the gpu
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
        // _check_dispatches(&dispatches);
        let dispatch_buffer = upload_dispatch(&device, &dispatches.concat());
        let dispatch_metadata = create_dispatch_metadata(&dispatches);

        let dispatch_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Dispatch Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let dispatch_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Dispatch Bind Group"),
            layout: &dispatch_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: dispatch_buffer.as_entire_binding(),
            }],
        });

        //////////////////// UNIFORM ////////////////////

        let uniform_bind_group_layout: BindGroupLayout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Uniform bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let uniform = Uniform {
            window_size_px: WINDOW_SIZE_X,
            particle_radius_px: PARTICLE_RADIUS,
            num_particles,
            current_dispatch: 0,
            dispatch_metadata: dispatch_metadata
                .iter()
                .flat_map(|&(a, b)| [a, b, 0, 0])
                .collect::<Vec<u32>>()
                .try_into()
                .expect("Array length mismatch"),
        };
        let uniform_buffer = create_uniform_buffer(&uniform, &device);
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        //////////////////// BINS ////////////////////

        let bin_indices_buffer = create_bin_indices_buffer(&device);
        let bin_particles_buffer = create_bin_particles_buffer(&device);

        let bin_bind_group_layout: BindGroupLayout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Bin bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let bin_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bin Bind Group"),
            layout: &bin_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: bin_indices_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: bin_particles_buffer.as_entire_binding(),
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

        let collision_compute_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Collision compute pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Collision Compute Pipeline Layout"),
                        bind_group_layouts: &[
                            &ssbo_bind_group_layout,
                            &uniform_bind_group_layout,
                            &dispatch_bind_group_layout,
                            &bin_bind_group_layout,
                        ],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &collision_compute_shader,
                entry_point: Some("step"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        let update_compute_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Update compute pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Update Compute Pipeline Layout"),
                        bind_group_layouts: &[&ssbo_bind_group_layout, &uniform_bind_group_layout],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &update_compute_shader,
                entry_point: Some("step"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        //////////////////// STAGING BUFFERS ////////////////////

        let particles_staging_buffer_read = create_particles_staging_buffer_read(&device);
        let particles_staging_buffer_write = create_particles_staging_buffer_write(&device);
        let instances_staging_buffer_write = create_instances_staging_buffer_write(&device);
        let bin_indices_staging_buffer_write = create_bin_indices_staging_buffer_write(&device);
        let bin_particles_staging_buffer_write = create_bin_particles_staging_buffer_write(&device);
        let uniform_staging_buffer_write = create_uniform_staging_buffer_write(&device);

        //////////////////// EGUI ////////////////////

        let egui_renderer = EguiRenderer::new(&device, surface_format, None, 1, &window);

        let state = Renderer {
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
            num_particles,

            uniform_bind_group,
            texture_bind_group,
            ssbo_bind_group,

            particles_staging_buffer_read,
            particles_staging_buffer_write,
            instances_staging_buffer_write,
            ssbo_buffer,

            collision_compute_pipeline,
            update_compute_pipeline,
            dispatch_bind_group,
            uniform,
            uniform_buffer,
            uniform_staging_buffer_write,
            compute_groups: dispatches
                .iter()
                .map(|dispatch| (dispatch.len().div_ceil(THREADS_PER_GROUP as usize)) as u32)
                .collect::<Vec<u32>>()
                .try_into()
                .expect("Array length mismatch"),

            egui_renderer,

            bin_indices_buffer,
            bin_indices_staging_buffer_write,
            bin_particles_buffer,
            bin_particles_staging_buffer_write,
            bin_bind_group,

            render_menu: true,
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

    pub fn render(
        &mut self,
        sim_state: SimulationState, // cursed, but since the renderer handles egui it also needs to know this
        lock_state: bool, // cursed, but since the renderer handles egui it also needs to know this
    ) -> Option<InputEvent> {
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
            pixels_per_point: self.window.scale_factor() as f32, // * state.scale_factor,
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

        render_pass.draw_indexed(0..6, 0, 0..(self.num_particles as u32));

        // End the render pass.
        drop(render_pass);

        let mut input: Option<InputEvent> = None;
        if self.render_menu {
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

                    match sim_state {
                        SimulationState::Paused => {
                            if ui.button("Unpause").clicked() {
                                input = Some(InputEvent::PauseOrUnpause);
                            }
                        }
                        SimulationState::Running => {
                            if ui.button("Pause").clicked() {
                                input = Some(InputEvent::PauseOrUnpause);
                            }
                        }
                    }

                    if lock_state {
                        if ui.button("Unlock FPS").clicked() {
                            input = Some(InputEvent::LockOrUnlock);
                        }
                    } else {
                        if ui.button("Lock FPS").clicked() {
                            input = Some(InputEvent::LockOrUnlock);
                        }
                    }
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
        let bytes_to_read = std::mem::size_of::<ParticlePhysics>() * self.num_particles as usize;
        encoder.copy_buffer_to_buffer(
            &self.ssbo_buffer,
            0,
            &self.particles_staging_buffer_read,
            0,
            bytes_to_read as u64,
        );
        self.queue.submit([encoder.finish()]);

        // Map staging buffer for reading
        let slice = self.particles_staging_buffer_read.slice(..);
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
        self.particles_staging_buffer_read.unmap();
        particles
    }

    pub fn write_particles(&self, particles: &[ParticlePhysics]) {
        let bytes_to_write = self.num_particles as usize * std::mem::size_of::<ParticlePhysics>();
        // Map staging buffer for writing
        let slice = self.particles_staging_buffer_write.slice(..);
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
        mapped[..bytes_to_write]
            .copy_from_slice(&bytemuck::cast_slice(particles)[..bytes_to_write]);
        drop(mapped);
        self.particles_staging_buffer_write.unmap();

        // Copy from staging buffer to ssbo_buffer
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(
            &self.particles_staging_buffer_write,
            0,
            &self.ssbo_buffer,
            0,
            bytes_to_write as u64,
        );
        self.queue.submit([encoder.finish()]);
    }

    pub fn write_instances(&self, instances: &[ParticleInstance]) {
        let bytes_to_write = self.num_particles as usize * std::mem::size_of::<ParticleInstance>();
        // Map staging buffer for writing
        let slice = self.instances_staging_buffer_write.slice(..);
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
        mapped[..bytes_to_write]
            .copy_from_slice(&bytemuck::cast_slice(instances)[..bytes_to_write]);
        drop(mapped);
        self.instances_staging_buffer_write.unmap();

        // Copy from staging buffer to ssbo_buffer
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(
            &self.instances_staging_buffer_write,
            0,
            &self.instances_buffer,
            0,
            bytes_to_write as u64,
        );
        self.queue.submit([encoder.finish()]);
    }

    pub fn write_bin_indices(&self, indices: &[u32]) {
        self.queue
            .write_buffer(&self.bin_indices_buffer, 0, bytemuck::cast_slice(indices));
    }

    pub fn write_bin_particles(&self, particles: &[u32]) {
        self.queue.write_buffer(
            &self.bin_particles_buffer,
            0,
            bytemuck::cast_slice(particles),
        );
    }

    pub fn set_uniform(&self, uniform: &Uniform, encoder: &mut CommandEncoder) {
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniform]));
    }

    pub fn gpu_update(&mut self) {
        let mut encoder = self.device.create_command_encoder(&Default::default());
        self.uniform.num_particles = self.num_particles;
        self.set_uniform(&self.uniform, &mut encoder);

        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor::default());
        // Set the pipeline that we want to use
        compute_pass.set_pipeline(&self.update_compute_pipeline);
        // Set the bind group that we want to use
        compute_pass.set_bind_group(0, &self.ssbo_bind_group, &[]);
        compute_pass.set_bind_group(1, &self.uniform_bind_group, &[]);

        compute_pass.dispatch_workgroups(self.num_particles.div_ceil(THREADS_PER_GROUP), 1, 1);
        drop(compute_pass);

        self.queue.submit([encoder.finish()]);
    }

    pub fn gpu_bin_solver(&mut self, bin_indices: &[u32], bin_particles: &[u32]) {
        let mut uniform = self.uniform.clone();

        self.write_bin_indices(bin_indices);
        self.write_bin_particles(bin_particles);

        // _check_bins(bin_indices, bin_particles);

        for dispatch in 0..9 {
            let mut encoder = self.device.create_command_encoder(&Default::default());
            uniform.current_dispatch = dispatch;
            self.set_uniform(&uniform, &mut encoder);

            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor::default());
            // Set the pipeline that we want to use
            compute_pass.set_pipeline(&self.collision_compute_pipeline);
            // Set the bind group that we want to use
            compute_pass.set_bind_group(0, &self.ssbo_bind_group, &[]);
            compute_pass.set_bind_group(1, &self.uniform_bind_group, &[]);
            compute_pass.set_bind_group(2, &self.dispatch_bind_group, &[]);
            compute_pass.set_bind_group(3, &self.bin_bind_group, &[]);

            compute_pass.dispatch_workgroups(self.compute_groups[dispatch as usize], 1, 1);
            drop(compute_pass);

            self.queue.submit([encoder.finish()]);
        }
    }

    pub fn basic_gpu_solver(&mut self) {
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor::default());

        // Set the pipeline that we want to use
        compute_pass.set_pipeline(&self.collision_compute_pipeline);
        // Set the bind group that we want to use
        compute_pass.set_bind_group(0, &self.ssbo_bind_group, &[]);
        compute_pass.set_bind_group(1, &self.uniform_bind_group, &[]);

        compute_pass.dispatch_workgroups(COMPUTE_GROUPS, 1, 1);
        drop(compute_pass);

        let command_buffer = encoder.finish();
        self.queue.submit([command_buffer]);
    }

    pub fn add_particles(&mut self, particles: &[ParticlePhysics]) {
        let num_particles_before = self.num_particles;
        let num_particles_after = self.num_particles + particles.len() as u32;
        let num_new_particles = num_particles_after - num_particles_before;

        let bytes_to_write = num_new_particles as usize * std::mem::size_of::<ParticlePhysics>();
        // Map staging buffer for writing
        let slice = self.particles_staging_buffer_write.slice(..);
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
        mapped[..bytes_to_write]
            .copy_from_slice(&bytemuck::cast_slice(particles)[..bytes_to_write]);
        drop(mapped);
        self.particles_staging_buffer_write.unmap();

        // Copy from staging buffer to ssbo_buffer
        let destination_offset =
            num_particles_before as usize * std::mem::size_of::<ParticlePhysics>();
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(
            &self.particles_staging_buffer_write,
            0,
            &self.ssbo_buffer,
            destination_offset as u64,
            bytes_to_write as u64,
        );
        self.queue.submit([encoder.finish()]);

        self.num_particles = num_particles_after;
    }
}

pub fn create_dispatch_metadata(dispatches: &[Vec<u32>; 9]) -> [(u32, u32); 9] {
    let mut metadata = [(0u32, 0u32); 9];
    let mut current_offset = 0u32;

    for (i, dispatch) in dispatches.iter().enumerate() {
        let length = dispatch.len() as u32;
        metadata[i] = (current_offset, length);
        current_offset += length;
    }

    metadata
}

fn _check_dispatches(dispatches: &[Vec<u32>; 9]) {
    warn!("CHECKING DISPATCHES");
    // bin_number, dispatch_number
    let mut seen_bins: HashMap<u32, u32> = HashMap::new();
    for (i, dispatch) in dispatches.iter().enumerate() {
        for bin in dispatch.iter() {
            if seen_bins.contains_key(bin) {
                println!(
                    "Collision in bin {bin} between {i} and {}",
                    seen_bins.get(bin).unwrap()
                );
            } else {
                seen_bins.insert(*bin, i as u32);
            }
        }
    }
    println!("{}", seen_bins.len());
    warn!("FINISHED");
}

fn _check_bins(bin_indices: &[u32], bin_particles: &[u32]) {
    warn!("CHECKING BINS");
    // particle_number, (row, col)
    let mut seen_particles: HashMap<u32, (u32, u32)> = HashMap::new();

    for col in 1..=(NUM_BINS_WITH_PADDING - 2) {
        for row in 1..=(NUM_BINS_WITH_PADDING - 2) {
            let bin = get_bin_index(row, col);
            let start = bin_indices[bin as usize];
            let end = bin_indices[(bin + 1) as usize];

            for j in start.clone()..end {
                let particle_id = bin_particles[j as usize];
                if seen_particles.contains_key(&particle_id) {
                    println!(
                        "Collision in bin {particle_id} between bins ({row}, {col}) and {:?}",
                        seen_particles.get(&particle_id).unwrap()
                    );
                    panic!();
                } else {
                    seen_particles.insert(particle_id, (row, col));
                }
            }
        }
    }
    println!("{}", seen_particles.len());
    warn!("FINISHED");
}
