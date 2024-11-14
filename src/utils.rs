use std::path::PathBuf;

use {
    notify::{
        event::{AccessKind, AccessMode},
        EventKind, RecursiveMode, Watcher,
    },
    wgpu::util::DeviceExt,
};

pub struct FileWatcher {
    pub watcher: notify::RecommendedWatcher,
    pub event_receiver: std::sync::mpsc::Receiver<Vec<PathBuf>>,
}

impl FileWatcher {
    pub fn init() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(notify::Event {
                kind: EventKind::Access(AccessKind::Close(AccessMode::Write)),
                paths,
                ..
            }) = res
            {
                tx.send(paths).unwrap();
            }
        })
        .expect("Should init watcher");

        Self {
            watcher,
            event_receiver: rx,
        }
    }

    pub fn watch(&mut self, path: &std::path::Path) {
        self.watcher
            .watch(path, notify::RecursiveMode::NonRecursive)
            .expect("Should start watching file");
    }
}

pub struct SceneTime {
    pub start: std::time::Instant,
    pub previous_update: std::time::Instant,

    pub buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub deltas_last_second: Vec<f32>,
}

impl SceneTime {
    pub fn new(device: &wgpu::Device) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Time Buffer"),
            contents: bytemuck::cast_slice(&[0.0f32]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Time Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self {
            start: std::time::Instant::now(),
            previous_update: std::time::Instant::now(),
            buffer,
            bind_group_layout,
            bind_group,
            deltas_last_second: Vec::new(),
        }
    }
}
