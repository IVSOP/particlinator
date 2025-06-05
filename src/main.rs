use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use wgpu::{util::DeviceExt, *};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalSize, Size},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::*,
};

const WINDOW_SIZE: f32 = 1000.0;
const PARTICLE_RADIUS: f32 = 5.0;

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
    num_instances: u32,

    uniform_bind_group_layout: BindGroupLayout,
    uniform_bind_group: BindGroup,

    // vertex_buffer: wgpu::Buffer,
    // render_pipeline: wgpu::RenderPipeline,

    // vertex_bind_group_layout: BindGroupLayout,
    // vertex_bind_group: BindGroup,

    // // info for every instance
    // instance_bind_group_layout: BindGroupLayout,
    // instance_bind_group: BindGroup,
}

/// The CPU-side structure that describes a single vertex of the triangle.
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
}

/// The CPU-side structure that describes an instance
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Instance {
    color: [f32; 4],
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
        position: [-0.5, -0.5],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.5, -0.5],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.5],
        uv: [1.0, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5],
        uv: [0.0, 1.0],
    },
];

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

static INDICES: [u32; 6] = [0, 1, 2, 2, 3, 0];

impl State {
    async fn new(window: Arc<Window>) -> State {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();

        let size = window.inner_size();

        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let instances = &[
            Instance { color: [1.0, 0.0, 0.0, 1.0] },
            Instance { color: [0.0, 1.0, 0.0, 1.0] },
            Instance { color: [0.0, 0.0, 1.0, 1.0] },
        ];

        let instances_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(instances),
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
                    visibility: wgpu::ShaderStages::FRAGMENT,
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
            label: Some("Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }
            ],
        });

        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Pipeline Layout"),
                    bind_group_layouts: &[
                        &uniform_bind_group_layout,
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
            num_instances: instances.len() as u32,

            uniform_bind_group_layout,
            uniform_bind_group,
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
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);


        render_pass.draw_indexed(0..6, 0, 0..(self.num_instances as u32));

        // End the render pass.
        drop(render_pass);

        // Submit the command in the queue to execute
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();
    }
}

#[derive(Default)]
struct App {
    state: Option<State>,
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
            WindowEvent::RedrawRequested => {
                //////// SIMULATION ANTES

                state.render();
                // Emits a new redraw requested event.
                state.get_window().request_redraw();
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

fn main() {
    // wgpu uses `log` for all of our logging, so we initialize a logger with the `env_logger` crate.
    //
    // To change the log level, set the `RUST_LOG` environment variable. See the `env_logger`
    // documentation for more information.
    env_logger::init();

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
