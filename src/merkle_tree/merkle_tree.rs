use std::{collections::HashMap, hash::Hash, path::PathBuf, ptr::NonNull};

pub struct Tree<K: Eq + Hash + Clone, D> {
    root: TreeNode<K, D>,
}
impl<K: Eq + Hash + Clone, D> Tree<K, D> {
    pub fn new(root_segment: K, data: D) -> Tree<K, D> {
        Tree {
            root: TreeNode {
                parent: Option::None,
                children: HashMap::new(),
                segment: root_segment,
                data,
            },
        }
    }

    pub fn get(&self, segments: &[K]) -> &D {
        &self.root.get(&segments).data
    }

    pub fn insert(&mut self, segments: &[K], data: D) {
        self.root.insert(&segments, data);
    }

    pub fn update(&mut self, segments: &[K], data: D) {
        self.root.update(&segments, data);
    }

    pub fn remove(&mut self, segments: &[K]) {
        self.root.remove(&segments);
    }
}
// SAFETY: All modifications require a mutable reference, therefore Tree is Send/Sync if its parts are Send/Sync.
unsafe impl<K: Send + Eq + Hash + Clone, D: Send> Send for Tree<K, D> {}
unsafe impl<K: Sync + Eq + Hash + Clone, D: Sync> Sync for Tree<K, D> {}

struct TreeNode<K: Eq + Hash + Clone, D> {
    parent: Option<NonNull<TreeNode<K, D>>>,
    children: HashMap<K, NonNull<TreeNode<K, D>>>,
    segment: K,
    data: D,
}
impl<K: Eq + Hash + Clone, D> TreeNode<K, D> {
    fn get(&self, segments: &[K]) -> &Self {
        if segments.len() == 0 {
            return self;
        }

        let next_node = *self.children.get(&segments[0]).expect("not such node");
        return unsafe { next_node.as_ref().get(&segments[1..]) };
    }

    fn insert(&mut self, segments: &[K], data: D) {
        if segments.len() == 1 {
            let new_node = TreeNode {
                parent: Some(self.into()),
                children: HashMap::new(),
                segment: segments[0].clone(),
                data,
            };
            let child_ptr = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(new_node))) };
            self.children.insert(segments[0].clone(), child_ptr);
            return;
        }

        let mut next_node = *self.children.get(&segments[0]).expect("not such node");
        unsafe { next_node.as_mut().insert(&segments[1..], data) };
    }

    fn update(&mut self, segments: &[K], data: D) {
        if segments.len() == 0 {
            let _ = std::mem::replace(&mut self.data, data);
            return;
        }

        let mut next_node = *self.children.get(&segments[0]).expect("not such node");
        unsafe { next_node.as_mut().update(&segments[1..], data) };
    }

    fn remove(&mut self, segments: &[K]) {
        if segments.len() == 1 {
            let child_ptr = self.children.remove(&segments[0]).expect("not such node");
            // SAFETY: This ensures the Box will be properly dropped after this scope, deallocating the memory.
            let _ = unsafe { Box::from_raw(child_ptr.as_ptr()) };
            return;
        }

        let mut next_node = *self.children.get(&segments[0]).expect("not such node");
        unsafe { next_node.as_mut().remove(&segments[1..]) };
    }
}

impl<K: Eq + Hash + Clone, D> Drop for TreeNode<K, D> {
    fn drop(&mut self) {
        for (_, child_ptr) in self.children.drain() {
            // SAFETY: This ensures the Box will be properly dropped after this scope, deallocating the memory.
            let _ = unsafe { Box::from_raw(child_ptr.as_ptr()) };
        }
        // Parent does not need to be dropped because it does not own the child.
        // The parent's existence is managed by its own scope/lifetime.
    }
}

pub struct File {
    path: PathBuf,
    last_modified: u64,
    file_size: u64,
    hash: Vec<u8>,
}
pub struct Directory {
    path: PathBuf,
    last_modified: u64,
    /// In case the directory is empty uses the path as hash.
    hash: Vec<u8>,
}

pub enum Entry {
    Directory(Directory),
    File(File),
}
