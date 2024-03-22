use std::{
    collections::{BTreeMap, BTreeSet},
    ptr::NonNull,
};

use blake3::Hash;

use crate::filesystem::data::MerkleEntry;

pub struct MerkleTree<K: AsRef<[u8]>> {
    root: TreeNode<K>,
}
impl<K: Eq + Ord + Clone + AsRef<[u8]>> MerkleTree<K> {
    pub fn new(root_segment: K, data: MerkleEntry) -> MerkleTree<K> {
        MerkleTree {
            root: TreeNode {
                parent: Option::None,
                children: BTreeMap::new(),
                segment: root_segment,
                hash: data.get_hash(),
                data,
            },
        }
    }

    pub fn get(&self, segments: &[K]) -> &MerkleEntry {
        &self.root.get(segments).data
    }

    pub fn get_hash(&self, segments: &[K]) -> &Hash {
        &self.root.get(segments).hash
    }

    pub fn insert(&mut self, segments: &[K], data: MerkleEntry) {
        self.root.insert(segments, data);
    }

    pub fn update(&mut self, segments: &[K], data: MerkleEntry) {
        self.root.update(segments, data);
    }

    pub fn remove(&mut self, segments: &[K]) {
        self.root.remove(segments);
    }

    pub fn find_difference<'a>(&'a self, other: &'a Self) -> Option<(Vec<&'a K>, Vec<&'a K>)> {
        self.root.find_difference(&other.root)
    }
}
// SAFETY: All modifications require a mutable reference, therefore Tree is Send/Sync if its parts are Send/Sync.
unsafe impl<K: Send + AsRef<[u8]>> Send for MerkleTree<K> {}
unsafe impl<K: Sync + AsRef<[u8]>> Sync for MerkleTree<K> {}

struct TreeNode<K: AsRef<[u8]>> {
    parent: Option<NonNull<TreeNode<K>>>,
    children: BTreeMap<K, NonNull<TreeNode<K>>>,
    segment: K,
    /// indicates if the contents of this node (its children and/or its data) changed
    hash: Hash,
    // TODO: this can probably be removed
    data: MerkleEntry,
}
impl<K: Eq + Ord + Clone + AsRef<[u8]>> TreeNode<K> {
    fn recompute_hash(&mut self) {
        if self.children.is_empty() {
            return;
        }
        let mut hasher = blake3::Hasher::new();
        self.children.values().for_each(|child| unsafe {
            let child = child.as_ref();
            hasher.update(child.hash.as_bytes());
            hasher.update(child.segment.as_ref());
        });
        self.hash = hasher.finalize();
    }

    fn get(&self, segments: &[K]) -> &Self {
        if segments.is_empty() {
            return self;
        }

        let next_node = *self.children.get(&segments[0]).expect("not such node");
        return unsafe { next_node.as_ref().get(&segments[1..]) };
    }

    fn insert(&mut self, segments: &[K], data: MerkleEntry) {
        if segments.len() == 1 {
            let new_node = TreeNode {
                parent: Some(self.into()),
                children: BTreeMap::new(),
                segment: segments[0].clone(),
                hash: data.get_hash(),
                data,
            };
            let child_ptr = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(new_node))) };
            self.children.insert(segments[0].clone(), child_ptr);
        } else {
            let mut next_node = *self.children.get(&segments[0]).expect("not such node");
            unsafe { next_node.as_mut().insert(&segments[1..], data) };
        }

        self.recompute_hash();
    }

    fn update(&mut self, segments: &[K], data: MerkleEntry) {
        if segments.is_empty() {
            self.data = data;
            return;
        }

        let mut next_node = *self.children.get(&segments[0]).expect("not such node");
        unsafe { next_node.as_mut().update(&segments[1..], data) };
        self.recompute_hash();
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
        self.recompute_hash();
    }

    pub fn find_difference<'a>(&'a self, other: &'a Self) -> Option<(Vec<&'a K>, Vec<&'a K>)> {
        if self.hash == other.hash {
            return None;
        }
        let a_empty = self.children.is_empty();
        let b_empty = other.children.is_empty();
        if a_empty {
            if b_empty {
                // TODO: determine which changed (based on timestamp)
                return Some((vec![&self.segment], vec![&other.segment]));
            }
            return Some((vec![], other.children.keys().collect()));
        }
        if b_empty {
            return Some((self.children.keys().collect(), vec![]));
        }

        let s1 = self.children.keys().collect::<BTreeSet<_>>();
        let s2 = other.children.keys().collect::<BTreeSet<_>>();
        let mut diff1 = s1.difference(&s2).copied().collect::<Vec<_>>();
        let mut diff2 = s2.difference(&s1).copied().collect::<Vec<_>>();
        s1.intersection(&s2)
            .map(|key| {
                let child1 = unsafe { self.children.get(key).unwrap().as_ref() };
                let child2 = unsafe { other.children.get(key).unwrap().as_ref() };
                (child1, child2)
            })
            .filter(|(child1, child2)| child1.hash != child2.hash)
            .filter_map(|(child1, child2)| child1.find_difference(child2))
            .for_each(|(mut child1, mut child2)| {
                diff1.append(&mut child1);
                diff2.append(&mut child2);
            });

        Some((diff1, diff2))
    }
}

impl<K: AsRef<[u8]>> Drop for TreeNode<K> {
    fn drop(&mut self) {
        for child_ptr in self.children.values() {
            // SAFETY: This ensures the Box will be properly dropped after this scope, deallocating the memory.
            let _ = unsafe { Box::from_raw(child_ptr.as_ptr()) };
        }
        // Parent does not need to be dropped because it does not own the child.
        // The parent's existence is managed by its own scope/lifetime.
    }
}
