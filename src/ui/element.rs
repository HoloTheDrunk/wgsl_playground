use glam::{FloatExt as _, Vec2};

use super::SdfObject;

#[derive(Clone, Copy)]
pub enum Operation {
    Merge,
    Intersect,
    Subtract,
    Interpolate { amount: f32 },
    RoundMerge { radius: f32 },
    RoundIntersect { radius: f32 },
    RoundSubtract { radius: f32 },
}

#[inline]
fn merge(lhs: f32, rhs: f32) -> f32 {
    lhs.min(rhs)
}

#[inline]
fn intersect(lhs: f32, rhs: f32) -> f32 {
    lhs.max(rhs)
}

#[inline]
fn subtract(lhs: f32, rhs: f32) -> f32 {
    lhs.max(-rhs)
}

#[inline]
fn interpolate(lhs: f32, rhs: f32, amount: f32) -> f32 {
    lhs.lerp(rhs, amount)
}

fn round_merge(lhs: f32, rhs: f32, radius: f32) -> f32 {
    let intersection_space = Vec2::new(lhs - radius, rhs - radius).min(Vec2::ZERO);
    let inside_distance = -intersection_space.length();

    let simple_merge = merge(lhs, rhs);
    let outside_distance = simple_merge.max(radius);

    inside_distance + outside_distance
}

fn round_intersect(lhs: f32, rhs: f32, radius: f32) -> f32 {
    let intersection_space = Vec2::new(lhs + radius, rhs + radius).min(Vec2::ZERO);
    let inside_distance = intersection_space.length();

    let simple_intersect = intersect(lhs, rhs);
    let outside_distance = simple_intersect.min(-radius);

    inside_distance + outside_distance
}

#[inline]
fn round_subtract(lhs: f32, rhs: f32, radius: f32) -> f32 {
    return round_intersect(lhs, -rhs, radius);
}

impl Operation {
    fn run(self, lhs: f32, rhs: f32) -> f32 {
        match self {
            Operation::Merge => merge(lhs, rhs),
            Operation::Intersect => intersect(lhs, rhs),
            Operation::Subtract => subtract(lhs, rhs),
            Operation::Interpolate { amount } => interpolate(lhs, rhs, amount),
            Operation::RoundMerge { radius } => round_merge(lhs, rhs, radius),
            Operation::RoundIntersect { radius } => round_intersect(lhs, rhs, radius),
            Operation::RoundSubtract { radius } => round_subtract(lhs, rhs, radius),
        }
    }
}

pub enum Element {
    Node { children: Vec<(Element, Operation)> },
    Leaf(Box<dyn SdfObject>),
}

impl SdfObject for Element {
    fn dist(&self, pos: glam::Vec2) -> f32 {
        match self {
            Element::Node { children } => {
                let Some(first) = children.first().map(|(elem, op)| (elem.dist(pos), op)) else {
                    return 0.;
                };

                let res = children
                    .iter()
                    .skip(1)
                    .fold(first, |(dist, op), (elem, next_op)| {
                        (op.run(dist, elem.dist(pos)), next_op)
                    });

                res.0
            }
            Element::Leaf(sdf_object) => sdf_object.dist(pos),
        }
    }
}

macro_rules! element {
    ((Node [$(
        ($op:expr => $child:tt)
    ),* $(,)?])) => {
        Element::Node {
            children: vec![$(
                ($child, $op)
            ),*]
        }
    };

    ((Leaf $shape:expr)) => {
        Element::Leaf(Box::new($shape))
    };

    () => {()};
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::ui::shapes::*;

    #[test]
    fn ui_macro() {
        let elem = element! {
            (Node [
                (Operation::Merge => (Leaf Circle::default())),
            ])
        };
    }
}
