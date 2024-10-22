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
}

impl InputEventProcessor for Mouse {
    fn process_events(&mut self, event: &WindowEvent) -> bool {
        self.data.process_events(event)
    }
}

pub struct MouseData {
    pub pos: PhysicalPosition<f64>,
    pub state: MouseState,
    pub hold_timer_ms: SimpleTimer,
}

impl InputEventProcessor for MouseData {
    fn process_events(&mut self, event: &WindowEvent) -> bool {
        let WindowEvent::MouseInput { state, button, .. } = event else {
            return false;
        };

        println!("{:?}", event);

        match self.state {
            _ if state == &ElementState::Released => self.state = MouseState::Idle,
            MouseState::Idle => {
                self.hold_timer_ms.start();
                self.state = MouseState::Clicked(button.clone());
            }
            MouseState::Clicked(mouse_button) if button == &mouse_button => {
                if self.hold_timer_ms.is_finished() {
                    self.state = MouseState::Held(mouse_button);
                }
            }
            MouseState::Held(mouse_button) if button == &mouse_button => (),
            _ => self.state = MouseState::Clicked(button.clone()),
        };

        true
    }
}

pub enum MouseState {
    Idle,
    Clicked(MouseButton),
    Held(MouseButton),
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct MouseUniform {
    pos: glam::Vec2,
    state: u32,
}

impl MouseUniform {
    pub fn from_data(data: MouseData) -> Self {
        Self {
            pos: glam::Vec2::new(data.pos.x as f32, data.pos.y as f32),
            state: match data.state {
                MouseState::Idle => 0,
                MouseState::Clicked(_) => 1,
                MouseState::Held(_) => 2,
            },
        }
    }
}
