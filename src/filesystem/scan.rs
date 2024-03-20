use std::{fs, path::Path};

use rayon::iter::{ParallelBridge, ParallelIterator};

pub fn scan_directory_recursive(path: &Path) {
    fs::read_dir(path)
        .expect("unable to read dir")
        .par_bridge()
        .map(|file| file.expect("unable to read file"))
        .for_each(|file| {
            let filetype = file.file_type().expect("unable to get filetype");
            if filetype.is_dir() {
                scan_directory_recursive(&file.path());
                return;
            }
            if filetype.is_file() {
                let filepath = file.path();
                let mut hasher = blake3::Hasher::new();
                hasher.update_mmap_rayon(&filepath).expect("unable to hash");
                let hash = hasher.finalize();

                println!("File: {:?}, Hash: {}", filepath, hash.to_hex())
            }
        });
}
