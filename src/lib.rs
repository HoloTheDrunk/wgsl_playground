#![allow(unused)]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
#![feature(let_chains)]

//! # wgsl_playground
//! Simple WGSL shader hot-reloading playground.

mod mouse;
mod shader_graph;
mod texture;
mod timer;

use std::path::Path;

use mouse::{Mouse, MouseData, MouseUniform};

use serde::Deserialize;
use texture::Texture;
use winit::platform::x11::WindowBuilderExtX11;

use {
    bytemuck::Zeroable,
    notify::{
        event::{AccessKind, AccessMode},
        EventKind, RecursiveMode, Watcher,
    },
    seq_macro::seq,
    wgpu::util::DeviceExt,
    winit::{
        dpi::{PhysicalPosition, PhysicalSize},
        event::*,
        event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
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

struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,

    assets_folder: std::path::PathBuf,

    render_pipeline: wgpu::RenderPipeline,
    render_pipeline_layout: wgpu::PipelineLayout,
    blit_pipeline: wgpu::RenderPipeline,
    blit_pipeline_layout: wgpu::PipelineLayout,

    diffuse_texture: Texture,
    diffuse_bind_group: wgpu::BindGroup,

    file_watcher: notify::RecommendedWatcher,
    fs_event_receiver: std::sync::mpsc::Receiver<()>,
    start_time: std::time::Instant,
    previous_update_time: std::time::Instant,
    time_buffer: wgpu::Buffer,
    time_bind_group: wgpu::BindGroup,
    time_deltas_last_second: Vec<f32>,

    mouse: Mouse,

    window: &'a Window,

    debug_buffer: wgpu::Buffer,
    debug_buffer_used: bool,
}

impl<'a> State<'a> {
    async fn new(window: &'a Window, config: &Config) -> Self {
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
                    memory_hints: wgpu::MemoryHints::Performance,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .expect("Should find compatible device");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .or(surface_caps.formats.get(0))
            .copied()
            .expect("Surface is incompatible with adapter.");

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: if surface_caps
                .present_modes
                .contains(&wgpu::PresentMode::Immediate)
            {
                wgpu::PresentMode::Immediate
            } else {
                surface_caps.present_modes[0]
            },
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: Vec::new(),
            desired_maximum_frame_latency: 2,
        };

        let assets_folder = Path::new(&config.assets_folder).to_path_buf();

        // FBO
        let diffuse_texture =
            Texture::create_diffuse_texture(&device, &surface_config, "Diffuse Texture FBO");

        let diffuse_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Diffuse Bind Group"),
            layout: &diffuse_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
        });

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

        // Mouse
        let mouse = Mouse::new(&device, MouseData::new(1000));

        // Render pipeline
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&time_bind_group_layout, &mouse.bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = Self::create_render_pipeline(
            &device,
            &surface_config,
            assets_folder.join("shader.wgsl").as_path(),
            &render_pipeline_layout,
            "Render Pipeline",
        )
        .expect("Shader should compile");

        // Render pipeline
        let blit_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&diffuse_bind_group_layout],
            push_constant_ranges: &[],
        });

        let blit_pipeline = Self::create_render_pipeline(
            &device,
            &surface_config,
            assets_folder.join("blit.wgsl").as_path(),
            &blit_pipeline_layout,
            "Blit Pipeline",
        )
        .expect("Shader should compile");

        // File Watcher
        let (tx, rx) = std::sync::mpsc::channel();
        let mut file_watcher =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                if let Ok(notify::Event {
                    kind: EventKind::Access(AccessKind::Close(AccessMode::Write)),
                    ..
                }) = res
                {
                    tx.send(()).unwrap();
                }
            })
            .expect("Should startup watcher");
        file_watcher
            .watch(
                assets_folder.join("shader.wgsl").as_path(),
                RecursiveMode::NonRecursive,
            )
            .expect("Should start watching file");

        // Debug buffer
        let debug_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Debug Buffer"),
            size: (surface_config.width * surface_config.height * 4) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let debug_buffer_used = false;

        Self {
            surface,
            device,
            queue,
            surface_config,
            size,
            assets_folder,
            render_pipeline,
            blit_pipeline,
            blit_pipeline_layout,
            diffuse_texture,
            diffuse_bind_group,
            debug_buffer,
            debug_buffer_used,
            file_watcher,
            fs_event_receiver: rx,
            start_time: std::time::Instant::now(),
            previous_update_time: std::time::Instant::now(),
            time_buffer,
            time_bind_group,
            render_pipeline_layout,
            time_deltas_last_second: Vec::new(),
            mouse,
            window,
        }
    }

    fn create_render_pipeline(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        shader: &Path,
        render_pipeline_layout: &wgpu::PipelineLayout,
        label: &str,
    ) -> Result<wgpu::RenderPipeline, wgpu::CompilationInfo> {
        let shader_code = shader_graph::ShaderGraph::try_from_final(shader)
            .expect("Shader code should be available at path")
            .finish()
            .expect("Shader code should compile successfully");

        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_code.into()),
        });
        if let Some(_) = pollster::block_on(device.pop_error_scope()) {
            let comp_info = pollster::block_on(shader.get_compilation_info());
            return Err(comp_info);
        }

        Ok(
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                cache: None,
                label: Some(label),
                layout: Some(render_pipeline_layout),
                vertex: wgpu::VertexState {
                    compilation_options: Default::default(),
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    compilation_options: Default::default(),
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
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            }),
        )
    }

    pub fn window(&self) -> &Window {
        self.window
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);

            self.diffuse_texture = Texture::create_diffuse_texture(
                &self.device,
                &self.surface_config,
                "Diffuse Texture",
            );
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        false
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

        // Time
        self.queue.write_buffer(
            &self.time_buffer,
            0,
            bytemuck::cast_slice(&[self.start_time.elapsed().as_secs_f32()]),
        );
        self.previous_update_time = std::time::Instant::now();

        // Mouse
        self.mouse.update(&self.queue);
        let mut data = MouseUniform::from_data(&self.mouse.data).normalize(glam::Vec2::new(
            self.size.width as f32,
            self.size.height as f32,
        ));
        self.queue
            .write_buffer(&self.mouse.buffer, 0, bytemuck::cast_slice(&[data]));

        // File watcher
        if self.fs_event_receiver.try_recv().is_ok() {
            // Drain channel
            while let Ok(_) = self.fs_event_receiver.try_recv() {}

            if let Ok(new_pipeline) = Self::create_render_pipeline(
                &self.device,
                &self.surface_config,
                self.assets_folder.join("shader.wgsl").as_path(),
                &self.render_pipeline_layout,
                "Render Pipeline",
            ) {
                self.render_pipeline = new_pipeline;
            }
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Intermediate render
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Intermediate Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.diffuse_texture.view,
                    // view: &output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::RED),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);

            render_pass.set_bind_group(0, &self.time_bind_group, &[]);
            render_pass.set_bind_group(1, &self.mouse.bind_group, &[]);

            render_pass.draw(0..3, 0..1);
        }

        // let tex = &self.diffuse_texture.texture;
        // let d_copy = tex.as_image_copy();
        // let b_copy = wgpu::ImageCopyBuffer {
        //     buffer: &self.debug_buffer,
        //     layout: wgpu::ImageDataLayout {
        //         offset: 0,
        //         bytes_per_row: tex
        //             .format()
        //             .target_pixel_byte_cost()
        //             .map(|cost| cost * tex.width())
        //             .map(|px| px - px % 256 + 256),
        //         rows_per_image: None,
        //     },
        // };
        // let extent = wgpu::Extent3d {
        //     width: 20,
        //     height: 20,
        //     depth_or_array_layers: 1,
        // };
        //
        // encoder.copy_texture_to_buffer(d_copy, b_copy, extent);

        // Blit
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Blit Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::RED),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.blit_pipeline);

            render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);

            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // let buffer_slice = self.debug_buffer.slice(..);
        // let (sender, receiver) = std::sync::mpsc::channel();
        // buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
        //
        // self.device.poll(wgpu::Maintain::wait()).panic_on_timeout();
        //
        // if let Ok(Ok(())) = receiver.recv() {
        //     let data = buffer_slice.get_mapped_range();
        //     let result = bytemuck::cast_slice::<_, [u8; 4]>(&data)
        //         .iter()
        //         .map(|p| [p[2], p[1], p[0], p[3]])
        //         .collect::<Vec<_>>();
        //
        //     drop(data);
        //     self.debug_buffer.unmap();
        //
        //     if !self.debug_buffer_used {
        //         println!(
        //             "{:?}",
        //             result
        //                 .iter()
        //                 .filter(|px| px.iter().any(|&c| c != 0))
        //                 .collect::<Vec<_>>()
        //         );
        //     }
        //
        //     self.debug_buffer_used = true;
        // } else {
        //     panic!("Failed to map debug buffer!");
        // }

        Ok(())
    }
}

#[derive(Deserialize)]
pub struct Config {
    window_size: (u32, u32),
    window_title: String,

    fps_limit: Option<u32>,

    assets_folder: String,
    // TODO: Make shader loading dynamic
    // shader_names: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            window_size: (600, 600),
            window_title: "WGSL Playground".to_string(),
            fps_limit: Some(60),
            assets_folder: "assets".to_string(),
            // shader_names: vec!["shader".to_string()],
        }
    }
}

pub async fn run(config: Config) {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title(config.window_title.as_str())
        .with_inner_size(PhysicalSize::new(
            config.window_size.0,
            config.window_size.1,
        ))
        .build(&event_loop)
        .unwrap();

    let mut state = State::new(&window, &config).await;

    event_loop
        .run(move |event, control_flow| {
            let delta = state.previous_update_time.elapsed().as_secs_f32();
            if let Some(fps_limit) = config.fps_limit
                && delta < 1. / fps_limit as f32
            {
                std::thread::sleep(std::time::Duration::from_secs_f32(
                    1. / fps_limit as f32 - delta,
                ));
            }
            handle_event(&mut state, event, control_flow);
        })
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
                WindowEvent::MouseInput { .. } | WindowEvent::CursorMoved { .. } => {
                    state.mouse.process_events(event);
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
