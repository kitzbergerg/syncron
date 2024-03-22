use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    hash::Hash,
    path::{Path, PathBuf},
    ptr::NonNull,
    time::UNIX_EPOCH,
};

use blake3::Hash as BHash;

use crate::filesystem::data::MerkleEntry;

pub struct MerkleTree<K: AsRef<[u8]>> {
    root: TreeNode<K>,
}
impl<K: Eq + Ord + Clone + Hash + AsRef<[u8]>> MerkleTree<K> {
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

    pub fn get_hash(&self, segments: &[K]) -> &BHash {
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

    pub fn find_difference<'a>(
        &'a self,
        other: &'a Self,
    ) -> Option<(Vec<&'a Path>, Vec<&'a Path>)> {
        let (diff1, diff2) = match self.root.find_difference(&other.root) {
            Some(inner) => inner,
            None => return None,
        };

        // TODO: figure out what to do with moves
        let keys1 = diff1.keys().collect::<HashSet<_>>();
        let keys2 = diff2.keys().collect::<HashSet<_>>();
        let moved_entries = keys1.intersection(&keys2).collect::<HashSet<_>>();
        moved_entries.iter().for_each(|hash| {
            let (l1, entry1) = diff1.get(**hash).unwrap();
            let (l2, entry2) = diff2.get(**hash).unwrap();
            if l1 < l2 {
                println!("{:?} moved to {:?}", entry1.get_path(), entry2.get_path());
            } else {
                println!("{:?} moved to {:?}", entry2.get_path(), entry1.get_path());
            };
        });

        Some((
            diff1.values().map(|(_, entry)| entry.get_path()).collect(),
            diff2.values().map(|(_, entry)| entry.get_path()).collect(),
        ))
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
    hash: BHash,
    last_modified: u64,
    // TODO: this can probably be removed
    data: MerkleEntry,
}
impl<K: Eq + Ord + Clone + Hash + AsRef<[u8]>> TreeNode<K> {
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

    fn find_difference<'a>(
        &'a self,
        other: &'a Self,
    ) -> Option<(
        HashMap<&'a BHash, (u64, &'a MerkleEntry)>,
        HashMap<&'a BHash, (u64, &'a MerkleEntry)>,
    )> {
        if self.hash == other.hash {
            // if hashes are the same, we have the same content
            return None;
        }

        // if one of both has no children, we found a leaf
        let a_empty = self.children.is_empty();
        let b_empty = other.children.is_empty();
        if a_empty && b_empty {
            if self.last_modified > other.last_modified {
                return Some((
                    HashMap::from([(&self.hash, (self.last_modified, &self.data))]),
                    HashMap::new(),
                ));
            } else {
                return Some((
                    HashMap::new(),
                    HashMap::from([(&other.hash, (self.last_modified, &other.data))]),
                ));
            }
        }
        if a_empty {
            return Some((
                HashMap::new(),
                other
                    .children
                    .values()
                    .map(|value| {
                        let node = unsafe { value.as_ref() };
                        (&node.hash, (node.last_modified, &node.data))
                    })
                    .collect(),
            ));
        }
        if b_empty {
            return Some((
                self.children
                    .values()
                    .map(|value| {
                        let node = unsafe { value.as_ref() };
                        (&node.hash, (node.last_modified, &node.data))
                    })
                    .collect(),
                HashMap::new(),
            ));
        }

        find_diff_in_children(&self.children, &other.children)
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

fn find_diff_in_children<'a, K: Eq + Ord + Hash + Clone + AsRef<[u8]>>(
    self_children: &'a BTreeMap<K, NonNull<TreeNode<K>>>,
    other_children: &'a BTreeMap<K, NonNull<TreeNode<K>>>,
) -> Option<(
    HashMap<&'a BHash, (u64, &'a MerkleEntry)>,
    HashMap<&'a BHash, (u64, &'a MerkleEntry)>,
)> {
    let mut diff1 = HashMap::new();
    let mut diff2 = HashMap::new();
    let mut common = Vec::new(); // To store common keys for further processing

    let mut iter_self = self_children.iter();
    let mut iter_other = other_children.iter();

    let mut next_self = iter_self.next();
    let mut next_other = iter_other.next();

    while next_self.is_some() || next_other.is_some() {
        match (next_self, next_other) {
            (Some((key_self, value_self)), Some((key_other, value_other))) => {
                match key_self.cmp(key_other) {
                    std::cmp::Ordering::Less => {
                        // key_self is unique to self
                        let node = unsafe { value_self.as_ref() };
                        diff1.insert(&node.hash, (node.last_modified, &node.data));
                        next_self = iter_self.next();
                    }
                    std::cmp::Ordering::Greater => {
                        // key_other is unique to other
                        let node = unsafe { value_other.as_ref() };
                        diff2.insert(&node.hash, (node.last_modified, &node.data));
                        next_other = iter_other.next();
                    }
                    std::cmp::Ordering::Equal => {
                        // key is present in both maps, check for differences in values
                        common.push((unsafe { value_self.as_ref() }, unsafe {
                            value_other.as_ref()
                        }));
                        next_self = iter_self.next();
                        next_other = iter_other.next();
                    }
                }
            }
            (Some((_, value_self)), None) => {
                // Remaining keys in self
                let node = unsafe { value_self.as_ref() };
                diff1.insert(&node.hash, (node.last_modified, &node.data));
                next_self = iter_self.next();
            }
            (None, Some((_, value_other))) => {
                // Remaining keys in other
                let node = unsafe { value_other.as_ref() };
                diff2.insert(&node.hash, (node.last_modified, &node.data));
                next_other = iter_other.next();
            }
            (None, None) => break,
        }
    }

    // Process common keys to find differences
    common
        .iter()
        .filter_map(|(child1, child2)| child1.find_difference(child2))
        .for_each(|(mut child1, mut child2)| {
            diff1.extend(child1);
            diff2.extend(child2);
        });

    Some((diff1, diff2))
}
