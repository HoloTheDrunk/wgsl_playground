use std::collections::HashMap;

use glam::{Vec2, Vec3};

use super::{element::Operation, SdfObject};

pub struct Ui {
    pub passes: Vec<Node>,
}

pub struct Node {
    pub z: ZOrder,
    pub variant: NodeVariant,
}

pub enum ZOrder {
    Relative(u32),
    Absolute(u32),
}

pub enum NodeVariant {
    Sdf(Sdf),
    Container(Container),
    Geometry(Geometry),
}

pub enum Sdf {
    Operation {
        op: Operation,
        lhs: Box<Sdf>,
        rhs: Box<Sdf>,
    },
    Shape(SdfShape),
    // NOTE: temp
    Text(String),
}

pub struct SdfShape {
    pub variant: Box<dyn SdfObject>,
}

pub enum SdfShapeVariant {
    Rectangle(super::shapes::Rectangle),
    Circle(super::shapes::Circle),
}

pub struct Container {
    pub pos: Vec2,
    pub variant: ContainerVariant,
}

pub enum Alignment {
    Start,
    End,
}

pub enum ContainerVariant {
    List {
        vec: Vec<Node>,
        scroll: f32,
        direction: ListDirection,
        cross_axis_align: Alignment,
        main_axis_align: Alignment,
    },
    Clip {
        mask: Box<dyn SdfObject>,
        child: Box<Node>,
    },
}

pub enum ListDirection {
    Horizontal,
    Vertical,
}

pub struct Geometry {
    pub pos: Vec2,
    pub material: wgpu::RenderPipeline,
}

#[cfg(test)]
pub mod test {
    use crate::ui::{element::Operation, shapes::Rectangle};

    use super::*;

    pub fn tree() -> Ui {
        Ui {
            passes: vec![Node {
                z: ZOrder::Absolute(0),
                variant: NodeVariant::Sdf(Sdf::Operation {
                    op: Operation::Merge,
                    lhs: Box::new(Sdf::Shape(SdfShape {
                        variant: Box::new(Rectangle {
                            center: Vec2::new(0., 0.),
                            half_size: Vec2::new(0.2, 0.2),
                        }),
                    })),
                    rhs: Box::new(Sdf::Operation {
                        op: Operation::Merge,
                        lhs: Box::new(Sdf::Shape(SdfShape {
                            variant: Box::new(Rectangle {
                                center: Vec2::new(0., 0.),
                                half_size: Vec2::new(0.2, 0.2),
                            }),
                        })),
                        rhs: Box::new(Sdf::Shape(SdfShape {
                            variant: Box::new(Rectangle {
                                center: Vec2::new(0., 0.),
                                half_size: Vec2::new(0.2, 0.2),
                            }),
                        })),
                    }),
                }),
            }],
        }
    }
}
