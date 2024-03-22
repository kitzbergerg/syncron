use std::{
    collections::{BTreeMap, BTreeSet},
    ptr::NonNull,
    time::UNIX_EPOCH,
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
                last_modified: data.get_last_modified(),
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
        // TODO: Add hashes to return as well. We can then use the hashes to detect a move.
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
    last_modified: u64,
    // TODO: this can probably be removed
    data: MerkleEntry,
}
impl<K: Eq + Ord + Clone + AsRef<[u8]>> TreeNode<K> {
    fn recompute_node(&mut self) {
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
        self.last_modified = UNIX_EPOCH.elapsed().unwrap().as_secs();
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
                last_modified: data.get_last_modified(),
                data,
            };
            let child_ptr = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(new_node))) };
            self.children.insert(segments[0].clone(), child_ptr);
        } else {
            let mut next_node = *self.children.get(&segments[0]).expect("not such node");
            unsafe { next_node.as_mut().insert(&segments[1..], data) };
        }

        self.recompute_node();
    }

    fn update(&mut self, segments: &[K], data: MerkleEntry) {
        if segments.is_empty() {
            self.data = data;
            return;
        }

        let mut next_node = *self.children.get(&segments[0]).expect("not such node");
        unsafe { next_node.as_mut().update(&segments[1..], data) };
        self.recompute_node();
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
        self.recompute_node();
    }

    pub fn find_difference<'a>(&'a self, other: &'a Self) -> Option<(Vec<&'a K>, Vec<&'a K>)> {
        if self.hash == other.hash {
            // if hashes are the same, we have the same content
            return None;
        }

        // if one of both has no children, we found a leaf
        let a_empty = self.children.is_empty();
        let b_empty = other.children.is_empty();
        if a_empty && b_empty {
            if self.last_modified > other.last_modified {
                return Some((vec![&self.segment], vec![]));
            } else {
                return Some((vec![], vec![&other.segment]));
            }
        }
        if a_empty {
            return Some((vec![], other.children.keys().collect()));
        }
        if b_empty {
            return Some((self.children.keys().collect(), vec![]));
        }

        // find the set difference in keys
        let s1 = self.children.keys().collect::<BTreeSet<_>>();
        let s2 = other.children.keys().collect::<BTreeSet<_>>();
        let mut diff1 = s1.difference(&s2).copied().collect::<Vec<_>>();
        let mut diff2 = s2.difference(&s1).copied().collect::<Vec<_>>();

        // if the keys stayed the same, check the children for changes
        s1.intersection(&s2)
            .map(|key| {
                let child1 = unsafe { self.children.get(key).unwrap().as_ref() };
                let child2 = unsafe { other.children.get(key).unwrap().as_ref() };
                (child1, child2)
            })
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
