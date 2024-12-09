use glam::Vec2;

use super::SdfObject;

#[derive(Debug)]
pub struct Circle {
    pub center: Vec2,
    pub radius: f32,
}

impl Default for Circle {
    fn default() -> Self {
        Self {
            center: Vec2::ZERO,
            radius: 1.,
        }
    }
}

impl SdfObject for Circle {
    fn dist(&self, pos: Vec2) -> f32 {
        pos.distance(self.center) - self.radius
    }

    fn fn_call(&self) -> String {
        format!(
            "disc(pos, vec2f({:?}, {:?}), {:?})",
            self.center.x, self.center.y, self.radius
        )
    }
}

impl Circle {
    pub fn new(center: Vec2, radius: f32) -> Self {
        Self { center, radius }
    }
}

#[derive(Debug)]
pub struct Rectangle {
    pub center: Vec2,
    pub half_size: Vec2,
}

impl SdfObject for Rectangle {
    fn dist(&self, pos: Vec2) -> f32 {
        let edge_distances = (pos - self.center).abs() - self.half_size;
        let outside_distance = edge_distances.max(Vec2::ZERO).length();
        let inside_distance = edge_distances.x.max(edge_distances.y).min(0.);
        outside_distance + inside_distance
    }

    fn fn_call(&self) -> String {
        format!(
            "rectangle(pos, vec2f({:?}, {:?}), vec2f({:?}, {:?}))",
            self.center.x, self.center.y, self.half_size.x, self.half_size.y,
        )
    }
}

impl Rectangle {
    pub fn new(center: Vec2, half_size: Vec2) -> Self {
        Self { center, half_size }
    }
}

#[derive(Debug)]
pub struct Text {
    pub string: String,
}

impl SdfObject for Text {
    fn dist(&self, pos: Vec2) -> f32 {
        todo!()
    }

    fn fn_call(&self) -> String {
        todo!()
    }
}
