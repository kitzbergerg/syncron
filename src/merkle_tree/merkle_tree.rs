use std::{collections::HashMap, path::PathBuf};

struct File {
    path: PathBuf,
    last_modified: u64,
    file_size: u64,
    hash: Vec<u8>,
}
pub struct Directory {
    path: PathBuf,
    last_modified: u64,
    // in case the entries are empty use path instead for the hash
    hash: Vec<u8>,
    // use the path as key to allow for quick checking of changed files
    entries: HashMap<PathBuf, Entry>,
}
impl Directory {
    pub fn new() -> Directory {
        unimplemented!()
    }
}

enum Entry {
    Directory(Directory),
    File(File),
}
