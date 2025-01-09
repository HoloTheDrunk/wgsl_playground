pub use lib::*;

mod lib {
    use std::{collections::VecDeque, fmt::Debug, ops::Deref};

    // We start by defining what information a tree should be able to give.
    // Here, a tree is represented recursively, no matter the actual data layout of the
    // implementation.
    pub trait Tree {
        /// Type of the data stored within nodes.
        type Value;

        /// Value of the current tree root.
        fn value(&self) -> &Self::Value;
        /// Child subtrees.
        fn children(&self) -> impl DoubleEndedIterator<Item = &Self>;
    }

    /// Depth-first iterator over a [Tree] data structure.
    pub struct Dfs<'c, T: Deref>
    where
        <T as Deref>::Target: Tree,
    {
        /// The current node.
        data: &'c <T as Deref>::Target,
        /// LIFO stack of Nodes seen and to be visited.
        to_visit: Vec<&'c <T as Deref>::Target>,
        done: bool,
    }

    impl<'c, T: Deref> Iterator for Dfs<'c, T>
    where
        <T as Deref>::Target: Tree,
    {
        type Item = &'c <T as Deref>::Target;

        fn next(&mut self) -> Option<Self::Item> {
            // Add the children in reverse order so that the leftmost one is on top of the stack.
            self.to_visit.extend(self.data.children().rev());

            // No children left to visit means we've gone through every node and we're currently at
            // the last leaf.
            if self.to_visit.is_empty() {
                // The `done` flag is necessary since we need to output the last node currently
                // held in the `data` field.
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
        /// Depth-first iterator over the nodes of the given tree.
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
        /// Breadth-first iterator over the nodes of the given tree.
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
        <T as Deref>::Target: Tree,
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
    // "Ego" because it has a mandatory first element (head). I am very funny. Trust.
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

    // Don't abuse macros or metaprogramming in general in actual code.
    macro_rules! ego {
        ((Node ($meta:expr) $first:tt $($child:tt)+)) => {
            EgoTree::Node {
                meta: $meta,
                first: Box::new(ego!($first)),
                children: vec![$(ego!($child)),+],
            }
        };

        ((Node ($meta:expr) $first:tt)) => {
            EgoTree::Node {
                meta: $meta,
                first: Box::new(ego!($first)),
                children: vec![],
            }
        };

        ((Leaf ($meta:expr))) => {
            EgoTree::Leaf {
                meta: $meta,
            }
        };

        () => {()};
    }

    fn simple_ego() -> EgoTree<&'static str> {
        ego! {
            (Node ("A")
                (Node ("B")
                    (Leaf ("D")))
                (Leaf ("C")))
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
        let res = meta_traversal(simple_ego().dfs());
        assert_eq!("ABDC", res.join(""));
    }

    #[test]
    fn simple_bfs() {
        let res = meta_traversal(simple_ego().bfs());
        assert_eq!("ABCD", res.join(""));
    }
}
