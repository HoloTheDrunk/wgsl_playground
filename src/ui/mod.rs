mod element;
mod shapes;

use std::fmt::Debug;

use glam::Vec2;

pub trait SdfObject: Debug {
    fn dist(&self, pos: Vec2) -> f32;
    fn fn_call(&self) -> String;
}

pub struct Ui {
    shapes: Vec<Box<dyn SdfObject>>,
}
