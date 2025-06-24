//! WARN: BLAZINGLY FAST
//! These tree function have undergone extreme tests, most notably, [`Tree::from_paths"`] is
//! practically O(1) for time, processing about ~7000 paths in 1ms

use anyhow::Result;
use flipper_rpc::{fs::FsReadDir, transport::serial::rpc::SerialRpcTransport};
use fxhash::{FxBuildHasher, FxHashMap};
use hyperloglockless::HyperLogLog;
use std::{
    collections::VecDeque,
    ffi::OsStr,
    hash::{BuildHasher, Hash, Hasher},
    path::{Component, Path},
    rc::Rc,
};

#[derive(Debug)]
pub struct Tree {
    pub nodes: Vec<Node>,
}

#[derive(Debug)]
pub struct Node {
    pub name: Rc<OsStr>,

    // Since this type is constructed from a list of FILES, when a node has no children, it is
    // CERTIAN that the node is a file.
    pub children: FxHashMap<Rc<OsStr>, usize>,

    /// DO NOT USE THIS AS AN INDICATOR IFNODE IS A DIR/FILE IT IS WRONG
    pub size: Option<u32>,
}

impl Node {
    pub fn new(name: impl AsRef<OsStr>, size: Option<u32>) -> Self {
        Self {
            name: Rc::from(name.as_ref()),
            children: Default::default(),
            size,
        }
    }
}

impl Tree {
    pub fn add_child_to(&mut self, node: Node, parent: usize) -> usize {
        let index = self.nodes.len();

        // PERF: This clones the Rc, not the OsStr
        self.nodes[parent].children.insert(node.name.clone(), index);
        self.nodes.push(node);

        index
    }

    pub fn find_child_by_name(&self, parent: usize, name: &OsStr) -> Option<&usize> {
        self.nodes[parent].children.get(name)
    }

    pub fn new(capacity: usize) -> Self {
        let mut nodes = Vec::with_capacity(capacity);
        nodes.push(Node::new("/", None));

        Self { nodes }
    }

    pub fn from_path_and_sizes<P: AsRef<Path> + Sync>(paths: &[(P, u32)]) -> Self {
        // TODO: Tune precision to avoid underestimation (requires reallocation) and have minimal
        // overhead when hashing (kinda expensive to hash everything 4x more)
        let hll = HyperLogLog::with_hasher(12, FxBuildHasher::new());

        let hasher = FxBuildHasher::new();

        // TODO: Find a way to check for identical things here without HLL, and this will get EXACT
        for (p, _) in paths {
            let mut parent = 0;
            for comp in p.as_ref().components() {
                if let Component::Normal(name) = comp {
                    let mut h = hasher.build_hasher();

                    h.write_u64(parent);
                    name.hash(&mut h);

                    let key_hash = h.finish();

                    hll.insert(&key_hash);
                    parent = key_hash;
                }
            }
        }

        let estimate = hll.count() + 1;
        let mut tree = Self::new(estimate);

        for (p, size) in paths {
            let mut parent = 0; // start at root
            for name in p.as_ref().components() {
                if let Component::Normal(name) = name {
                    if let Some(&child_idx) = tree.find_child_by_name(parent, name) {
                        parent = child_idx;
                    } else {
                        let new_idx = tree.add_child_to(Node::new(name, Some(*size)), parent);
                        parent = new_idx;
                    }
                }
            }
        }

        tree
    }
}

///////////////////////////////

#[derive(Debug)]
pub struct RemoteTree {
    pub nodes: Vec<RemoteNode>,
}

#[derive(Debug)]
pub struct RemoteNode {
    pub name: Rc<OsStr>,
    pub children: FxHashMap<Rc<OsStr>, usize>,

    /// None if a directory
    pub size: Option<u32>,
}

impl RemoteNode {
    pub fn new(name: impl AsRef<OsStr>, size: Option<u32>) -> Self {
        Self {
            name: Rc::from(name.as_ref()),
            size,
            children: Default::default(),
        }
    }
}
impl RemoteTree {
    pub fn add_child_to(&mut self, node: RemoteNode, parent: usize) -> usize {
        let index = self.nodes.len();

        // PERF: This clones the Rc, not the OsStr
        self.nodes[parent].children.insert(node.name.clone(), index);
        self.nodes.push(node);

        index
    }

    pub fn new() -> Self {
        Self {
            nodes: vec![RemoteNode::new("/", None)],
        }
    }

    // WARNING: THIS IS HIGHLY INNEFICIENT. It re-allocates data! :scared:
    // Do not use this in prod unless you are a goober
    pub fn from_remote(
        cli: &mut SerialRpcTransport,
        root: impl AsRef<Path>,
        ignore: &'static [&'static str],
    ) -> Result<Self> {
        let mut tree = Self::new();

        // TODO: Find a proper size
        let mut queue = VecDeque::new(); // WARNING: NON-FIXED ALLOCATION
        queue.push_back((0usize, root.as_ref().to_path_buf()));

        while let Some((parent_idx, path)) = queue.pop_front() {
            let items = cli.fs_read_dir(&path, false)?;
            for item in items {
                match item {
                    flipper_rpc::rpc::res::ReadDirItem::File(name, size, _hash) => {
                        if parent_idx == 0 && ignore.contains(&name.as_str()) {
                            continue;
                        }
                        tree.add_child_to(RemoteNode::new(&name, Some(size)), parent_idx);
                    }
                    flipper_rpc::rpc::res::ReadDirItem::Dir(name) => {
                        if parent_idx == 0 && ignore.contains(&name.as_str()) {
                            continue;
                        }
                        let full_path = path.join(&name);

                        let index = tree.add_child_to(RemoteNode::new(&name, None), parent_idx);
                        queue.push_back((index, full_path));
                    }
                }
            }
        }

        Ok(tree)
    }
}
