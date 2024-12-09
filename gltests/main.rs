//! This separate test file with no harness is necessary so we can run on the main thread and
//! initialize winit and the such properly.
//! TODO: Expand this into a semi-proper testsuite system. Maybe make a utility crate later on and
//! publish it separately?

use std::path::Path;

use wgsl_playground::{shader_graph::ShaderGraph, ui::prelude::*};

use {
    glam::Vec2,
    wgpu::Color,
    winit::{dpi::PhysicalSize, event_loop::EventLoop, window::WindowBuilder},
};

fn main() {
    env_logger::init();

    let (device, queue) = setup();

    let ui = Ui {
        theme: UiTheme {
            colors: UiThemeColors {
                primary: Color {
                    r: 0.,
                    g: 1.,
                    b: 1.,
                    a: 1.,
                },
                secondary: Color {
                    r: 1.,
                    g: 0.,
                    b: 1.,
                    a: 1.,
                },
                tertiary: Color::default(),
            },
            borders: UiThemeBorders {
                enabled: true,
                offset: 0.,
                width: 0.01,
            },
            font: Font::load(
                &device,
                &queue,
                Path::new("assets/fonts/NotoSansMono-Black.ttf"),
            ),
        },
        tree: element! {
            (Node (Node (Leaf Circle::default()) []) [
                ((Leaf (Shape Circle : (Vec2::ZERO) (1.))) Operation::Merge),
            ])
        },
    };

    let shader = ui.wgsl_shader();
    println!("{}", shader);

    let shader_code =
        ShaderGraph::try_from_code(shader, Path::new("assets"), "ui_shader".to_owned())
            .expect("Shader code should form a valid graph")
            .finish()
            .expect("Shader code should compile successfully");

    {
        device.push_error_scope(wgpu::ErrorFilter::Validation);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_code.into()),
        });

        if let Some(_) = pollster::block_on(device.pop_error_scope()) {
            let comp_info = pollster::block_on(shader.get_compilation_info());
            panic!("Failed to compile shader: {comp_info:#?}");
        }
    }
}

fn setup() -> (wgpu::Device, wgpu::Queue) {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("ui::test::ui_shader")
        .with_inner_size(PhysicalSize::new(600, 600))
        .build(&event_loop)
        .unwrap();

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });

    let surface = instance.create_surface(window).unwrap();

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    }))
    .unwrap();

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            memory_hints: wgpu::MemoryHints::Performance,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            label: None,
        },
        None,
    ))
    .expect("Should find compatible device");

    (device, queue)
}
