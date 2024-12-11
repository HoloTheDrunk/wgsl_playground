pub use lib::*;

mod lib {
    use std::{collections::VecDeque, fmt::Debug, ops::Deref};

    pub trait Tree {
        type Value;

        fn value(&self) -> &Self::Value;
        fn children(&self) -> impl DoubleEndedIterator<Item = &Self>;
    }

    pub struct Dfs<'c, T: Deref>
    where
        <T as Deref>::Target: Tree,
    {
        data: &'c <T as Deref>::Target,
        to_visit: Vec<&'c <T as Deref>::Target>,
        done: bool,
    }

    impl<'c, T: Deref> Iterator for Dfs<'c, T>
    where
        <T as Deref>::Target: Tree + Debug,
    {
        type Item = &'c <T as Deref>::Target;

        fn next(&mut self) -> Option<Self::Item> {
            self.to_visit.extend(self.data.children().rev());

            if self.to_visit.is_empty() {
                if self.done {
                    return None;
                }
                self.done = true;
                return Some(self.data);
            }

            let res = self.data;

            self.data = self.to_visit.pop().unwrap();

            Some(res)
        }
    }

    pub trait ToDfs: Tree {
        fn dfs(&self) -> Dfs<&Self>;
    }

    impl<T: Tree> ToDfs for T {
        fn dfs(&self) -> Dfs<&Self> {
            Dfs {
                data: self,
                to_visit: Vec::new(),
                done: false,
            }
        }
    }

    pub struct Bfs<'c, T: Deref>
    where
        <T as Deref>::Target: Tree,
    {
        data: &'c <T as Deref>::Target,
        to_visit: VecDeque<&'c <T as Deref>::Target>,
        done: bool,
    }

    pub trait ToBfs: Tree {
        fn bfs(&self) -> Bfs<&Self>;
    }

    impl<T: Tree> ToBfs for T {
        fn bfs(&self) -> Bfs<&Self> {
            Bfs {
                data: self,
                to_visit: VecDeque::new(),
                done: false,
            }
        }
    }

    impl<'c, T: Deref> Iterator for Bfs<'c, T>
    where
        <T as Deref>::Target: Tree + Debug,
    {
        type Item = &'c <T as Deref>::Target;

        fn next(&mut self) -> Option<Self::Item> {
            self.to_visit.extend(self.data.children());

            if self.to_visit.is_empty() {
                if self.done {
                    return None;
                }
                self.done = true;
                return Some(self.data);
            }

            let res = self.data;

            self.data = self.to_visit.pop_front().unwrap();

            Some(res)
        }
    }
}

#[cfg(test)]
mod test {
    use super::lib::{ToBfs, ToDfs, Tree};

    #[derive(Debug)]
    enum EgoTree<Meta> {
        Node {
            meta: Meta,
            first: Box<Self>,
            children: Vec<Self>,
        },
        Leaf {
            meta: Meta,
        },
    }

    impl<M> Tree for EgoTree<M> {
        type Value = M;

        fn value(&self) -> &Self::Value {
            match self {
                EgoTree::Node { meta, .. } => meta,
                EgoTree::Leaf { meta } => meta,
            }
        }

        fn children(&self) -> impl DoubleEndedIterator<Item = &Self> {
            let children = match self {
                EgoTree::Node {
                    children, first, ..
                } => Some(std::iter::once(&**first).chain(children.iter())),
                EgoTree::Leaf { .. } => None,
            };

            std::iter::empty().chain(children).flatten()
        }
    }

    #[macro_export]
    macro_rules! ego {
        ((Node ($meta:expr) $first:tt [ $(
            $child:tt
        ),* $(,)?])) => {
            EgoTree::Node {
                meta: $meta,
                first: Box::new(ego!($first)),
                children: vec![$(ego!($child)),*],
            }
        };

        ((Leaf ($meta:expr))) => {
            EgoTree::Leaf {
                meta: $meta,
            }
        };

        () => {()};
    }
    pub use ego;

    fn simple_ego() -> EgoTree<&'static str> {
        ego! {
            (Node ("A")
                (Node ("B")
                    (Leaf ("D")) [])
                [(Leaf ("C"))])
        }
    }

    fn meta_traversal<'c>(
        iter: impl Iterator<Item = &'c EgoTree<&'static str>>,
    ) -> Vec<&'static str> {
        let mut vec = Vec::new();
        for node in iter {
            let label = match node {
                EgoTree::Node { meta, .. } => meta,
                EgoTree::Leaf { meta, .. } => meta,
            };
            vec.push(*label);
        }
        return vec;
    }

    #[test]
    fn simple_dfs() {
        meta_traversal(simple_ego().dfs());
    }

    #[test]
    fn simple_bfs() {
        meta_traversal(simple_ego().bfs());
    }
}
