#![allow(unused)]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

//! # cloth_sim
//! This simple cloth simulation engine aims to showcase a minimal example of
//! decent compute shader cloth simulation for educational purposes.

mod camera;
mod model;
mod shader;
mod texture;

use winit::event_loop::{ControlFlow, EventLoopWindowTarget};

use {
    camera::{Camera, CameraData},
    model::{DrawModel, Model, Vertex},
    texture::Texture,
};

use {
    seq_macro::seq,
    wgpu::util::DeviceExt,
    winit::{
        dpi::{PhysicalPosition, PhysicalSize},
        event::*,
        event_loop::EventLoop,
        keyboard::{KeyCode, PhysicalKey},
        window::{Window, WindowBuilder},
    },
};

pub trait InputEventProcessor {
    fn process_events(&mut self, event: &WindowEvent) -> bool;
}

pub trait GpuBuffer<Data> {
    type Init;

    fn init_buffer(device: &wgpu::Device, init: &Self::Init) -> GpuBufferData<Data>;
    fn write_buffer(&self, queue: &wgpu::Queue);
}

pub struct GpuBufferData<Data> {
    data: Data,
    buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

// TODO: Find a better name
pub trait Updateable {
    fn update(&mut self, queue: &wgpu::Queue);
}

macro_rules! mat4_vertex_attribute {
    ($shader_location_start:literal .. $shader_location_end:literal) => {{
        assert_eq!($shader_location_start + 4, $shader_location_end, "Range must be exactly 4 long");

        seq!(N in $shader_location_start..$shader_location_end {&[#(
            wgpu::VertexAttribute {
                offset: (N - $shader_location_start) * std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                format: wgpu::VertexFormat::Float32x4,
                shader_location: N,
            },
        )*]})
    }};
}

struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,

    render_pipelines: Vec<wgpu::RenderPipeline>,
    current_render_pipeline: usize,

    obj_model: Model,

    depth_texture: Texture,

    camera: Camera,

    start_time: std::time::Instant,
    previous_update_time: std::time::Instant,
    time_buffer: wgpu::Buffer,
    time_bind_group: wgpu::BindGroup,
    time_deltas_last_second: Vec<f32>,

    cursor: Option<PhysicalPosition<f64>>,
    window: &'a Window,
}

impl<'a> State<'a> {
    async fn new(window: &'a Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        // Surface
        let surface = instance.create_surface(window).unwrap();

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
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .or(surface_caps.formats.get(0))
            .copied()
            .expect("Surface is incompatible with adapter.");

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: Vec::new(),
            desired_maximum_frame_latency: 2,
        };

        // Depth texture
        let depth_texture = Texture::create_depth_texture(&device, &config, "Depth Texture");

        // Camera
        let camera = Camera::new(
            &device,
            CameraData {
                eye: (0., 1., 2.).into(),
                target: glam::Vec3::ZERO,
                up: glam::Vec3::Y,
                aspect: config.width as f32 / config.height as f32,
                fovy: 45.,
                znear: 0.1,
                zfar: 100.,
            },
        );

        // Time uniform
        let time_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Time Buffer"),
            contents: bytemuck::cast_slice(&[0.0f32]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let time_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Time Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let time_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Time Bind Group"),
            layout: &time_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: time_buffer.as_entire_binding(),
            }],
        });

        // Render pipeline
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera.bind_group_layout, &time_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipelines = [shader]
            .into_iter()
            .map(|shader| {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Render Pipeline"),
                    layout: Some(&render_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: "vs_main",
                        buffers: &[model::ModelVertex::desc()],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: "fs_main",
                        targets: &[Some(wgpu::ColorTargetState {
                            format: config.format,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: Some(wgpu::Face::Back),
                        polygon_mode: wgpu::PolygonMode::Fill,
                        unclipped_depth: false,
                        conservative: false,
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: Texture::DEPTH_FORMAT,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::Less,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState {
                        count: 1,
                        mask: !0,
                        alpha_to_coverage_enabled: false,
                    },
                    multiview: None,
                })
            })
            .collect();

        // Model
        let obj_model = match Model::from_file("plane.obj", &device, &queue).await {
            Ok(v) => v,
            Err(e) => panic!("{e:?}"),
        };

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipelines,
            current_render_pipeline: 0,
            obj_model,
            depth_texture,
            camera,
            start_time: std::time::Instant::now(),
            previous_update_time: std::time::Instant::now(),
            time_buffer,
            time_bind_group,
            time_deltas_last_second: Vec::new(),
            cursor: None,
            window,
        }
    }

    pub fn window(&self) -> &Window {
        self.window
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            self.depth_texture =
                Texture::create_depth_texture(&self.device, &self.config, "Depth Texture");
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        self.camera.process_events(event)
    }

    fn update(&mut self) {
        let delta_time = self.previous_update_time.elapsed().as_secs_f32();

        // FPS logging
        self.time_deltas_last_second.push(delta_time);
        let sum_deltas = self.time_deltas_last_second.iter().sum::<f32>();
        if sum_deltas > 1. {
            let deltas = self.time_deltas_last_second.len();
            println!(
                "fps: {} ({deltas} / {sum_deltas})",
                deltas as f32 / sum_deltas,
            );
            self.time_deltas_last_second.clear();
        }

        // Camera
        self.camera.update(&self.queue);

        // Time
        self.queue.write_buffer(
            &self.time_buffer,
            0,
            bytemuck::cast_slice(&[self.start_time.elapsed().as_secs_f32()]),
        );

        self.previous_update_time = std::time::Instant::now();
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::RED),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(
                &self.render_pipelines[self.current_render_pipeline % self.render_pipelines.len()],
            );

            render_pass.set_bind_group(0, &self.camera.bind_group, &[]);
            render_pass.set_bind_group(1, &self.time_bind_group, &[]);

            render_pass.draw_model(&self.obj_model);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

pub async fn run() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut state = State::new(&window).await;

    event_loop
        .run(move |event, control_flow| handle_event(&mut state, event, control_flow))
        .unwrap();
}

fn handle_event(state: &mut State<'_>, event: Event<()>, control_flow: &EventLoopWindowTarget<()>) {
    match event {
        Event::WindowEvent {
            window_id,
            ref event,
        } if window_id == state.window().id() && !state.input(event) => {
            match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            state: ElementState::Pressed,
                            physical_key: PhysicalKey::Code(KeyCode::Escape),
                            ..
                        },
                    ..
                } => control_flow.exit(),
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            state: ElementState::Pressed,
                            physical_key: PhysicalKey::Code(KeyCode::Space),
                            ..
                        },
                    ..
                } => state.current_render_pipeline += 1,
                WindowEvent::CursorMoved {
                    device_id: _,
                    position,
                } => {
                    state.cursor = Some(*position);
                }
                WindowEvent::Resized(physical_size) => {
                    state.resize(*physical_size);
                }
                WindowEvent::RedrawRequested => {
                    state.update();
                    match state.render() {
                        Ok(_) => {}
                        // Reconfigure the surface if lost
                        Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                        // The system is out of memory, we should probably quit
                        Err(wgpu::SurfaceError::OutOfMemory) => control_flow.exit(),
                        // All other errors (Outdated, Timeout) should be resolved by the next frame
                        Err(e) => eprintln!("{:?}", e),
                    }
                }
                _ => {}
            };
        }
        _ => {}
    };

    state.window().request_redraw();
}
