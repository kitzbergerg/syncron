use blake3::{Hash, OUT_LEN};
use std::{
    fs::File,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Debug)]
pub struct MerkleFile {
    path: PathBuf,
    last_modified: u64,
    hash: Hash,
}
impl MerkleFile {
    fn from_path(path: PathBuf) -> Self {
        let file = File::open(&path).expect("unable to open file");
        let metadata = file.metadata().expect("unable to read metadata");
        let last_modified = metadata
            .modified()
            .expect("unable to read last modified")
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // TODO: open and read file data only once. Possibly copy impl of update_mmap_rayon.
        let mut hasher = blake3::Hasher::new();
        hasher.update_mmap_rayon(&path).expect("unable to hash");
        let hash = hasher.finalize();

        Self {
            path,
            last_modified,
            hash,
        }
    }
}

#[derive(Debug)]
pub struct Directory {
    path: PathBuf,
}
impl Directory {
    fn from_path(path: PathBuf) -> Self {
        Self { path }
    }
}

#[derive(Debug)]
pub enum MerkleEntry {
    File(MerkleFile),
    Directory(Directory),
}
impl MerkleEntry {
    pub fn from_path(path: PathBuf) -> Self {
        if path.is_file() {
            return Self::File(MerkleFile::from_path(path));
        }
        if path.is_dir() {
            return Self::Directory(Directory::from_path(path));
        }
        println!("Funny file: {path:?}");
        unimplemented!()
    }

    pub fn get_path(&self) -> &Path {
        match &self {
            Self::Directory(dir) => &dir.path,
            Self::File(file) => &file.path,
        }
    }
    pub fn get_hash(&self) -> Hash {
        match self {
            Self::File(file) => file.hash,
            Self::Directory(_) => Hash::from_bytes([0; OUT_LEN]), // default value that will be recomputed in tree
        }
    }
    pub fn get_last_modified(&self) -> u64 {
        match self {
            Self::File(file) => file.last_modified,
            Self::Directory(_) => 0, // default value that will be recomputed in tree
        }
    }
}
