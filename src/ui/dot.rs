use crate::qol::map;

use super::{
    arbre::{Container, ContainerVariant, Geometry, Node, NodeVariant, Sdf},
    element::Operation,
};
use std::{collections::HashMap, ops::Deref};

pub trait DumpDotNode {
    fn name(&self) -> String;
    fn attrs(&self) -> DumpDotNodeAttrs;
    fn children(&self) -> Vec<&dyn DumpDotNode>;
}

#[derive(Default)]
pub struct DumpDotNodeAttrs {
    label: Option<String>,
}

impl DumpDotNodeAttrs {
    fn to_dot(&self) -> String {
        [self.label.clone()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl<T: Deref<Target = Node>> DumpDotNode for T {
    fn name(&self) -> String {
        self.deref().name()
    }

    fn attrs(&self) -> DumpDotNodeAttrs {
        self.deref().attrs()
    }

    fn children(&self) -> Vec<&dyn DumpDotNode> {
        self.deref().children()
    }
}

impl DumpDotNode for Node {
    fn name(&self) -> String {
        match &self.variant {
            NodeVariant::Sdf(sdf) => sdf.name(),
            NodeVariant::Container(container) => todo!(),
            NodeVariant::Geometry(geometry) => todo!(),
        }
    }

    fn attrs(&self) -> DumpDotNodeAttrs {
        match &self.variant {
            NodeVariant::Sdf(sdf) => sdf.attrs(),
            NodeVariant::Container(container) => container.attrs(),
            NodeVariant::Geometry(geometry) => geometry.attrs(),
        }
    }

    fn children(&self) -> Vec<&dyn DumpDotNode> {
        match &self.variant {
            NodeVariant::Sdf(sdf) => sdf.children(),
            NodeVariant::Container(container) => container.children(),
            NodeVariant::Geometry(geometry) => geometry.children(),
        }
    }
}

impl DumpDotNode for Sdf {
    fn name(&self) -> String {
        match self {
            Sdf::Operation { op, .. } => op.to_string(),
            Sdf::Shape(sdf_shape) => sdf_shape.variant.name().to_owned(),
            Sdf::Text(text) => text.clone(),
        }
    }

    fn attrs(&self) -> DumpDotNodeAttrs {
        DumpDotNodeAttrs {
            label: Some(format!("Sdf {}", self.name())),
        }
    }

    fn children(&self) -> Vec<&dyn DumpDotNode> {
        match self {
            Sdf::Operation { lhs, rhs, .. } => vec![&**lhs, &**rhs],
            _ => Vec::new(),
        }
    }
}

impl DumpDotNode for Container {
    fn name(&self) -> String {
        match self.variant {
            ContainerVariant::List { .. } => "List",
            ContainerVariant::Clip { .. } => "Clip",
        }
        .into()
    }

    fn attrs(&self) -> DumpDotNodeAttrs {
        DumpDotNodeAttrs {
            label: Some(self.name()),
        }
    }

    fn children(&self) -> Vec<&dyn DumpDotNode> {
        match &self.variant {
            ContainerVariant::List { vec, .. } => {
                vec.iter().map(|node| node as &dyn DumpDotNode).collect()
            }
            ContainerVariant::Clip { child, .. } => vec![&*child],
        }
    }
}

impl DumpDotNode for Geometry {
    fn name(&self) -> String {
        "Geometry".into()
    }

    fn attrs(&self) -> DumpDotNodeAttrs {
        DumpDotNodeAttrs {
            label: Some(self.name()),
        }
    }

    fn children(&self) -> Vec<&dyn DumpDotNode> {
        Vec::new()
    }
}

struct NodeDotDumper {
    seq: u32,
    nodes: HashMap<String, (u32, String)>,
}

impl NodeDotDumper {
    pub fn new() -> Self {
        Self {
            seq: 0,
            nodes: HashMap::new(),
        }
    }
    pub fn dump_dot(&mut self, item: impl DumpDotNode) -> String {
        let id = self.seq;
        self.seq += 1;

        let name = item.name();
        let attrs = item.attrs().to_dot();

        format!("\"{name}_{id}\" [{attrs}]")
    }
}

pub struct DotDumper {}

#[cfg(test)]
mod test {
    use super::*;

    use crate::ui::arbre::test::tree;

    #[test]
    fn tree_dot() {
        let mut dumper = NodeDotDumper::new();
        let dots = tree()
            .passes
            .iter()
            .map(|pass| dumper.dump_dot(pass))
            .collect::<Vec<_>>();
        dbg!(dots);
    }
}
