use bytemuck::Zeroable;
use macros::generate_wgsl_enum;

use crate::{timer::SimpleTimer, GpuBuffer, GpuBufferData, InputEventProcessor, Updateable};

use {
    bytemuck::Pod,
    wgpu::util::DeviceExt,
    winit::{
        dpi::PhysicalPosition,
        event::*,
        keyboard::{KeyCode, PhysicalKey},
    },
};

pub struct Mouse {
    pub data: MouseData,
    pub uniform: MouseUniform,
    pub buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl Mouse {
    pub fn new(device: &wgpu::Device, data: MouseData) -> Self {
        let GpuBufferData {
            data: uniform,
            buffer,
            bind_group_layout,
            bind_group,
        } = Self::init_buffer(device, &data);

        Self {
            data,
            uniform,
            buffer,
            bind_group_layout,
            bind_group,
        }
    }
}

impl InputEventProcessor for Mouse {
    fn process_events(&mut self, event: &WindowEvent) -> bool {
        self.data.process_events(event)
    }
}

impl Updateable for Mouse {
    fn update(&mut self, queue: &wgpu::Queue) {
        self.data.update(queue);
    }
}

pub struct MouseData {
    pub pos: PhysicalPosition<f64>,
    pub state: MouseState,
    pub hold_timer_ms: SimpleTimer,
}

impl MouseData {
    pub fn new(hold_timer_ms: u128) -> Self {
        Self {
            pos: Default::default(),
            state: MouseState::Idle,
            hold_timer_ms: SimpleTimer::from_ms(hold_timer_ms),
        }
    }
}

impl InputEventProcessor for MouseData {
    fn process_events(&mut self, event: &WindowEvent) -> bool {
        let WindowEvent::MouseInput { state, button, .. } = event else {
            return false;
        };

        match self.state {
            _ if state == &ElementState::Released => self.state = MouseState::Idle,
            MouseState::Clicked(mouse_button) | MouseState::Held(mouse_button)
                if button == &mouse_button => {}
            _ => {
                self.hold_timer_ms.start();
                self.state = MouseState::Clicked(button.clone());
            }
        };

        true
    }
}

impl Updateable for MouseData {
    fn update(&mut self, queue: &wgpu::Queue) {
        if let MouseState::Clicked(button) = self.state
            && self.hold_timer_ms.is_finished()
        {
            self.state = MouseState::Held(button);
        }
    }
}

#[generate_wgsl_enum("assets/generated/mouse_state.wgsl")]
#[derive(Debug)]
pub enum MouseState {
    Idle,
    Clicked(MouseButton),
    Held(MouseButton),
}

impl GpuBuffer<MouseUniform> for Mouse {
    type Init = MouseData;

    fn init_buffer(device: &wgpu::Device, data: &Self::Init) -> GpuBufferData<MouseUniform> {
        // let uniform = CameraUniform::from_config(config);
        let uniform = MouseUniform::from_data(data);
        // let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //     label: Some("Camera Buffer"),
        //     contents: bytemuck::cast_slice(&[uniform]),
        //     usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        // });
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mouse Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        // let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        //     label: Some("Camera Bind Group Layout"),
        //     entries: &[wgpu::BindGroupLayoutEntry {
        //         binding: 0,
        //         visibility: wgpu::ShaderStages::VERTEX,
        //         ty: wgpu::BindingType::Buffer {
        //             ty: wgpu::BufferBindingType::Uniform,
        //             has_dynamic_offset: false,
        //             min_binding_size: None,
        //         },
        //         count: None,
        //     }],
        // });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Mouse Uniform Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        // let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     label: Some("Camera Bind Group"),
        //     layout: &bind_group_layout,
        //     entries: &[wgpu::BindGroupEntry {
        //         binding: 0,
        //         resource: buffer.as_entire_binding(),
        //     }],
        // });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Mouse Uniform Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        GpuBufferData {
            data: uniform,
            buffer,
            bind_group_layout,
            bind_group,
        }
    }

    fn write_buffer(&self, queue: &wgpu::Queue) {
        todo!()
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C, align(16))]
pub struct MouseUniform {
    pub pos: glam::Vec2,
    pub state: u32,
    _padding: u32,
}

impl MouseUniform {
    pub fn from_data(data: &MouseData) -> Self {
        Self {
            pos: glam::Vec2::new(data.pos.x as f32, data.pos.y as f32),
            state: match data.state {
                MouseState::Idle => 0,
                MouseState::Clicked(_) => 1,
                MouseState::Held(_) => 2,
            },
            _padding: 0,
        }
    }

    pub fn normalize(mut self, size: glam::Vec2) -> Self {
        self.pos.x = self.pos.x / size.x as f32;
        self.pos.y = self.pos.y / size.y as f32;
        self
    }
}
