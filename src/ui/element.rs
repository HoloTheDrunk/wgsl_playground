use {
    glam::{FloatExt as _, Vec2},
    indoc::{formatdoc, indoc},
    std::collections::VecDeque,
};

use super::SdfObject;

#[derive(Clone, Copy, Debug)]
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
            Self::Merge => merge(lhs, rhs),
            Self::Intersect => intersect(lhs, rhs),
            Self::Subtract => subtract(lhs, rhs),
            Self::Interpolate { amount } => interpolate(lhs, rhs, amount),
            Self::RoundMerge { radius } => round_merge(lhs, rhs, radius),
            Self::RoundIntersect { radius } => round_intersect(lhs, rhs, radius),
            Self::RoundSubtract { radius } => round_subtract(lhs, rhs, radius),
        }
    }
}

impl Operation {
    pub fn wgsl_counterpart(&self) -> (&'static str, Option<Vec<String>>) {
        match self {
            Self::Merge => ("merge", None),
            Self::Intersect => ("intersect", None),
            Self::Subtract => ("subtract", None),
            Self::Interpolate { amount } => ("interpolate", Some(vec![format!("{amount}")])),
            Self::RoundMerge { radius } => ("round_merge", Some(vec![format!("{radius}")])),
            Self::RoundIntersect { radius } => ("round_intersect", Some(vec![format!("{radius}")])),
            Self::RoundSubtract { radius } => ("round_subtract", Some(vec![format!("{radius}")])),
        }
    }
}

#[derive(Debug)]
pub enum Element {
    Node {
        first: Box<Element>,
        children: Vec<(Element, Operation)>,
    },
    Leaf(Box<dyn SdfObject>),
}

impl SdfObject for Element {
    fn dist(&self, pos: glam::Vec2) -> f32 {
        match self {
            Element::Node { first, children } => {
                children.iter().fold(first.dist(pos), |dist, (elem, op)| {
                    op.run(dist, elem.dist(pos))
                })
            }
            Element::Leaf(sdf_object) => sdf_object.dist(pos),
        }
    }

    // TODO: Organize this better to avoid having to do this.
    fn fn_call(&self) -> String {
        unreachable!("Don't call this")
    }
}

impl Element {
    pub fn to_wgsl_function(&self, label: &str) -> String {
        let formula = self.to_wgsl_expr();
        formatdoc! {
            "fn {label}(pos: vec2f) -> f32 {{
                return {formula};
            }}"
        }
    }

    // PERF: horribly inefficient but might not matter
    fn to_wgsl_expr(&self) -> String {
        match self {
            Element::Node { first, children } => {
                let mut res: VecDeque<_> = vec![first.to_wgsl_expr()].into();

                for (child, op) in children.iter() {
                    let (fun, args) = op.wgsl_counterpart();

                    res.push_front(format!("{fun}("));

                    res.push_back(format!(", {}", child.to_wgsl_expr()));

                    if let Some(args) = args {
                        res.push_back(format!(", {}", args.join(", ")));
                    }

                    res.push_back(")".to_owned());
                }

                res.into_iter().collect::<String>()
            }
            Element::Leaf(sdf_object) => sdf_object.fn_call(),
        }
    }
}

#[macro_export]
#[macro_use]
macro_rules! element {
    ((Node $first:tt [ $(
        ($child:tt $op:expr)
    ),* $(,)?])) => {
        Element::Node {
            first: Box::new(element!($first)),
            children: vec![$(
                (element!($child), $op)
            ),*]
        }
    };

    ((Leaf $shape:tt)) => {
        Element::Leaf(Box::new(element!($shape)))
    };

    ((Leaf $shape:expr)) => {
        Element::Leaf(Box::new($shape))
    };

    ((Shape $shape:path : $(($arg:expr))*)) => {
        <$shape>::new($($arg),*)
    };

    () => {()};
}
pub use element;

#[cfg(test)]
mod test {
    use super::*;

    use crate::ui::shapes::*;

    #[test]
    fn ui_macro() {
        let elem = element! {
            (Node (Node (Leaf Circle::default()) []) [
                ((Leaf (Shape Circle : (Vec2::ZERO) (1.))) Operation::Merge),
            ])
        };

        dbg!(&elem);
        println!("{}", elem.to_wgsl_function("element_test"));
    }
}
