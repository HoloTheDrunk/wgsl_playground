mod element;
mod shapes;

use glam::Vec2;

pub trait SdfObject {
    fn dist(&self, pos: Vec2) -> f32;
}

pub struct Ui {
    shapes: Vec<Box<dyn SdfObject>>,
}
