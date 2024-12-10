use std::ops::Deref;

pub trait Tree {
    type Value;

    fn value(&self) -> &Self::Value;
    fn children(&self) -> impl Iterator<Item = &Self>;
}

pub struct Dfs<'c, T: Deref>
where
    <T as Deref>::Target: Tree,
{
    data: T,
    to_visit: Vec<&'c <T as Deref>::Target>,
}

impl<'c, T: Deref> Iterator for Dfs<'c, T>
where
    <T as Deref>::Target: Tree,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.to_visit.len() == 0 {
            self.to_visit.extend(self.data.children());
        }

        todo!()
    }
}

pub trait ToDFS: Tree {
    fn dfs(&self) -> Dfs<&Self>;
}

// pub struct Bfs<State, Tree> {
//     state: State,
//     data: Tree,
// }
//
// pub trait ToBFS {
//     type State;
//
//     fn bfs(&self, state: Self::State) -> Bfs<Self::State, &Self>;
// }

enum EgoTree<Meta, Value> {
    Node {
        meta: Meta,
        first: Box<Self>,
        children: Vec<Self>,
    },
    Leaf {
        meta: Meta,
        value: Value,
    },
}

impl<M, V> Tree for EgoTree<M, V> {
    type Value = M;

    fn value(&self) -> &Self::Value {
        match self {
            EgoTree::Node { meta, .. } => meta,
            EgoTree::Leaf { meta, value } => meta,
        }
    }

    fn children(&self) -> impl Iterator<Item = &Self> {
        let children = match self {
            EgoTree::Node { children, .. } => Some(children.iter()),
            EgoTree::Leaf { .. } => None,
        };

        std::iter::empty().chain(children).flatten()
    }
}

impl<M, V> ToDFS for EgoTree<M, V> {
    fn dfs(&self) -> Dfs<&Self> {
        Dfs {
            data: self,
            to_visit: Vec::new(),
        }
    }
}
