use blake3::Hash;
use std::{
    fs::File,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Debug)]
pub struct MerkleFile {
    pub path: PathBuf,
    pub last_modified: u64,
    pub file_size: u64,
    pub hash: Hash,
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
        hasher.update(path.file_name().unwrap().as_encoded_bytes());
        hasher.update_mmap_rayon(&path).expect("unable to hash");
        let hash = hasher.finalize();

        Self {
            path,
            last_modified,
            file_size: metadata.len(),
            hash,
        }
    }
}

#[derive(Debug)]
pub struct MerkleDir {
    pub path: PathBuf,
    pub last_modified: u64,
    pub hash: Hash,
}
impl MerkleDir {
    pub fn from_path(path: PathBuf) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(path.file_name().unwrap().as_encoded_bytes());

        Self {
            path,
            last_modified: 0,
            hash: hasher.finalize(),
        }
    }
}

#[derive(Debug)]
pub enum MerkleEntry {
    Directory(MerkleDir),
    File(MerkleFile),
}
impl MerkleEntry {
    pub fn from_path(path: PathBuf) -> Self {
        if path.is_file() {
            return Self::File(MerkleFile::from_path(path));
        }
        if path.is_dir() {
            return Self::Directory(MerkleDir::from_path(path));
        }
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
            Self::Directory(dir) => dir.hash, // default value that will be recomputed in tree
            Self::File(file) => file.hash,
        }
    }
}
