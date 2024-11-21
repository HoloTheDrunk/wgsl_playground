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
}
