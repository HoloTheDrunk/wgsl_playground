#![allow(unused)]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
#![feature(let_chains)]

//! # wgsl_playground
//! "Simple" WGSL shader hot-reloading playground.

pub mod mouse;
mod qol;
pub mod shader_graph;
pub mod texture;
pub mod timer;
pub mod tree;
pub mod ui;
pub mod utils;

use {
    mouse::{Mouse, MouseData, MouseUniform},
    shader_graph::ShaderGraph,
    texture::{Texture, TexturePair},
    ui::{font::Font, prelude::Element, Ui, UiTheme, UiThemeBorders, UiThemeColors},
    utils::{FileWatcher, SceneTime},
};

use std::path::{Path, PathBuf};

use {
    bytemuck::Zeroable,
    glam::Vec2,
    notify::{
        event::{AccessKind, AccessMode},
        EventKind, RecursiveMode, Watcher,
    },
    seq_macro::seq,
    serde::Deserialize,
    wgpu::{util::DeviceExt, Color},
    winit::platform::x11::WindowBuilderExtX11,
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

struct Pipeline {
    shader: shader_graph::ShaderGraph,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::PipelineLayout,
}

struct State<'a> {
    window: &'a Window,
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,

    assets_folder: std::path::PathBuf,

    ui: Ui,
    hide_ui: bool,

    render_pipelines: Vec<Pipeline>,
    ui_pipeline: Pipeline,
    blit_pipeline: Pipeline,

    texture_pair: TexturePair,

    file_watcher: FileWatcher,

    time: SceneTime,
    mouse: Mouse,
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
            .or(surface_caps.formats.first())
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

        // FBOs
        let texture_pair = TexturePair::new(&device, &surface_config);

        // Time uniform
        let time = SceneTime::new(&device);

        // Mouse
        let mouse = Mouse::new(&device, MouseData::new(1000));

        // Render pipeline
        let render_pipelines = config
            .shader_paths
            .iter()
            .map(|path| {
                let path = match &path[path.len() - 5..] {
                    ".wgsl" => path.to_owned(),
                    _ => format!("{path}.wgsl"),
                };

                let render_pipeline_shader =
                    shader_graph::ShaderGraph::try_from_file(assets_folder.join(&path).as_path())
                        .unwrap_or_else(|_| {
                            panic!("Shader code should be available at path '{path}'")
                        });

                let render_pipeline_layout =
                    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Render Pipeline Layout"),
                        bind_group_layouts: &[
                            &texture_pair.get().0.bind_group_layout,
                            &time.bind_group_layout,
                            &mouse.bind_group_layout,
                        ],
                        push_constant_ranges: &[],
                    });

                Pipeline {
                    pipeline: Self::create_render_pipeline(
                        &device,
                        &surface_config,
                        &render_pipeline_shader,
                        &render_pipeline_layout,
                        format!("Render Pipeline ({path})").as_str(),
                    )
                    .unwrap_or_else(|_| panic!("Shader should compile ({path})")),
                    shader: render_pipeline_shader,
                    layout: render_pipeline_layout,
                }
            })
            .collect::<Vec<_>>();

        // UI pipeline
        let ui = Ui {
            theme: UiTheme {
                colors: UiThemeColors {
                    primary: Color {
                        r: 0.,
                        g: 0.,
                        b: 0.,
                        a: 1.,
                    },
                    secondary: Color {
                        r: 1.,
                        g: 1.,
                        b: 1.,
                        a: 0.9,
                    },
                    tertiary: Color::default(),
                },
                borders: UiThemeBorders {
                    enabled: true,
                    offset: 0.,
                    width: 0.005,
                },
                font: Font::load(
                    &device,
                    &queue,
                    Path::new("assets/fonts/NotoSansMono-Black.ttf"),
                ),
            },
            tree: element! {
                (Node
                    (Node
                        (Leaf (Shape ui::shapes::Rectangle : (Vec2::new(0.09, 0.5)) (Vec2::new(0.1, 0.6))))
                        [])
                    [
                        (
                            (Leaf (Shape ui::shapes::Circle : (Vec2::new(0.25, 0.05)) (0.15)))
                            ui::element::Operation::RoundMerge { radius: 0.1 }
                        ),
                        (
                            (Leaf (Shape ui::shapes::Rectangle : (Vec2::new(0.5, 0.05)) (Vec2::new(0.15, 0.01))))
                            ui::element::Operation::RoundMerge { radius: 0.01 }
                        ),
                        (
                            (Leaf (Shape ui::shapes::Rectangle : (Vec2::new(0.5, 0.1)) (Vec2::new(0.15, 0.01))))
                            ui::element::Operation::RoundMerge { radius: 0.01 }
                        )
                    ])
            },
        };

        let ui_pipeline_shader_graph = ShaderGraph::try_from_code(
            ui.wgsl_shader(),
            Path::new("assets"),
            "ui_shader".to_owned(),
        )
        .expect("Shader code should form a valid graph");
        let ui_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Ui Pipeline Layout"),
            bind_group_layouts: &[
                &texture_pair.get().0.bind_group_layout,
                // atlas metadata
                &ui.theme.font.bind_group_layout,
                // atlas texture
                &ui.theme.font.texture_bind.bind_group_layout,
            ],
            push_constant_ranges: &[],
        });
        let ui_pipeline = Pipeline {
            pipeline: Self::create_render_pipeline(
                &device,
                &surface_config,
                &ui_pipeline_shader_graph,
                &ui_pipeline_layout,
                "UI Pipeline",
            )
            .expect("UI shader should compile"),
            shader: ui_pipeline_shader_graph,
            layout: ui_pipeline_layout,
        };

        // Blit pipeline
        let blit_pipeline_shader =
            shader_graph::ShaderGraph::try_from_file(assets_folder.join("blit.wgsl").as_path())
                .expect("Shader code should be available at path");
        let blit_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&texture_pair.get().0.bind_group_layout],
            push_constant_ranges: &[],
        });

        let blit_pipeline = Pipeline {
            pipeline: Self::create_render_pipeline(
                &device,
                &surface_config,
                &blit_pipeline_shader,
                &blit_pipeline_layout,
                "Blit Pipeline",
            )
            .expect("Shader should compile"),
            shader: blit_pipeline_shader,
            layout: blit_pipeline_layout,
        };

        // File Watcher
        let mut file_watcher = FileWatcher::init();
        for path in render_pipelines.iter().flat_map(|pipeline| {
            pipeline.shader.ids().filter_map(|rc| match &**rc {
                shader_graph::NodeId::Path(path_buf) => Some(path_buf.clone()),
                shader_graph::NodeId::Label(_) => None,
            })
        }) {
            file_watcher.watch(path.as_path());
        }
        file_watcher.watch(assets_folder.join("blit.wgsl").as_path());

        Self {
            window,
            surface,
            device,
            queue,
            surface_config,
            size,
            assets_folder,
            ui,
            hide_ui: false,
            render_pipelines,
            ui_pipeline,
            blit_pipeline,
            texture_pair,
            file_watcher,
            time,
            mouse,
        }
    }

    fn create_render_pipeline(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        shader_graph: &shader_graph::ShaderGraph,
        render_pipeline_layout: &wgpu::PipelineLayout,
        label: &str,
    ) -> Result<wgpu::RenderPipeline, wgpu::CompilationInfo> {
        let shader_code = shader_graph
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

            self.texture_pair = TexturePair::new(&self.device, &self.surface_config);
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    fn update(&mut self) {
        let delta_time = self.time.previous_update.elapsed().as_secs_f32();

        // FPS logging
        self.time.deltas_last_second.push(delta_time);
        let sum_deltas = self.time.deltas_last_second.iter().sum::<f32>();
        if sum_deltas > 1. {
            let deltas = self.time.deltas_last_second.len();
            println!(
                "fps: {} ({deltas} / {sum_deltas})",
                deltas as f32 / sum_deltas,
            );
            self.time.deltas_last_second.clear();
        }

        // Time
        self.queue.write_buffer(
            &self.time.buffer,
            0,
            bytemuck::cast_slice(&[self.time.start.elapsed().as_secs_f32()]),
        );
        self.time.previous_update = std::time::Instant::now();

        // Mouse
        self.mouse.update(&self.queue);
        let mut data = MouseUniform::new(
            &self.mouse.data,
            glam::Vec2::new(self.size.width as f32, self.size.height as f32),
        );
        self.queue
            .write_buffer(&self.mouse.buffer, 0, bytemuck::cast_slice(&[data]));

        // File watcher
        if let Ok(updated_paths) = self.file_watcher.event_receiver.try_recv() {
            // Drain channel
            while self.file_watcher.event_receiver.try_recv().is_ok() {}

            for pipeline in self.render_pipelines.iter_mut() {
                let last = pipeline
                    .shader
                    .last()
                    .expect("Shader should have at least one node at this point");

                let shader_graph::NodeId::Path(last_path) = &*last.id else {
                    unreachable!("User render pipelines should use file shaders exclusively");
                };
                if !&updated_paths.contains(&last_path.canonicalize().unwrap_or_else(|_| {
                    panic!(
                        "Shader path should be canonicalizable: '{}'",
                        last_path.to_str().unwrap()
                    )
                })) {
                    continue;
                }

                let shader = shader_graph::ShaderGraph::try_from_file(last_path.as_path())
                    .expect("Shader graph should be possible to build");

                if let Ok(new_pipeline) = Self::create_render_pipeline(
                    &self.device,
                    &self.surface_config,
                    &shader,
                    &pipeline.layout,
                    format!(
                        "Render Pipeline ({})",
                        last_path
                            .to_str()
                            .expect("Last node path should already be valid")
                    )
                    .as_str(),
                ) {
                    pipeline.pipeline = new_pipeline;
                }
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

        // Intermediate renders
        for render_pipeline in self.render_pipelines.iter() {
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Intermediate Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.texture_pair.get().1.texture.view,
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

                render_pass.set_pipeline(&render_pipeline.pipeline);

                render_pass.set_bind_group(0, &self.texture_pair.get().0.bind_group, &[]);
                render_pass.set_bind_group(1, &self.time.bind_group, &[]);
                render_pass.set_bind_group(2, &self.mouse.bind_group, &[]);

                render_pass.draw(0..3, 0..1);
            }

            self.texture_pair.swap();
        }

        // UI
        'ui: {
            if self.hide_ui {
                break 'ui;
            }

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("UI Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.texture_pair.get().1.texture.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.ui_pipeline.pipeline);

            render_pass.set_bind_group(0, &self.texture_pair.get().0.bind_group, &[]);
            render_pass.set_bind_group(1, &self.ui.theme.font.bind_group, &[]);
            render_pass.set_bind_group(2, &self.ui.theme.font.texture_bind.bind_group, &[]);

            render_pass.draw(0..3, 0..1);

            self.texture_pair.swap();
        }

        // Blit
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Blit Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.blit_pipeline.pipeline);

            render_pass.set_bind_group(0, &self.texture_pair.get().0.bind_group, &[]);

            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[derive(Deserialize)]
pub struct Config {
    window_size: (u32, u32),
    window_title: String,

    fps_limit: Option<u32>,

    assets_folder: String,
    shader_paths: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            window_size: (600, 600),
            window_title: "WGSL Playground".to_string(),
            fps_limit: Some(60),
            assets_folder: "assets".to_string(),
            shader_paths: vec!["shader".to_string()],
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
            let delta = state.time.previous_update.elapsed().as_secs_f32();
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
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            state: ElementState::Pressed,
                            physical_key: PhysicalKey::Code(key_code),
                            ..
                        },
                    ..
                } => match key_code {
                    KeyCode::Space => state.hide_ui = !state.hide_ui,
                    _ => (),
                },
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
