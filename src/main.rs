//! # Core components
//!
//! ## Data structure representing filesystem (as merkle tree; possibly multithreaded)
//! This is the central component and should have the following functionality:
//!  - Create/Update/Move/Delete files.
//!  - Create/Update/Move/Delete folders.
//!  - Compare two trees to find differences. Use timestamps to merge trees. When remote is newer update local tree and trigger event to update filesystem (think about what happens if scanner doesn't detect file since it's not downloaded yet; would result in scanner saying its deleted; maybe create tmp files such as hash.tmp, that way scanner know it will eventually be there?). When local is newer send changes to remote and trigger event to upload files. Events will be sent to channel and handled by the filesystem component (meaning eventual consistency as merkle tree might have files that are not yet in the filesystem).
//!  - Periodically sync/merge tree with remote. Probably include some debouncing here.
//!
//! ## Local filesystem
//! The local filesystem should represent the merkle tree as closely as possible (or be at least eventually consistent).
//! It should have the following functionality:
//!  - Merge filesystem changes into the tree. Changes are detected by periodic scans and file watchers.
//!      - Periodic scans should be smart. They should utilize the last modified timestamp in the tree to check frequently change directories more often. These scans will run every 2min-5h (times might change based factors such as performance, size of filesystem) depending on the last modified timestamp.
//!      - File watchers should be smart and watch files and directories the user is currently working in. There is usually a limited number of detected events and a limited number of directories/files that can be watched at any given time. Therefore file watchers will be set up only for directories the user has recently made changes in (based on last modified). Careful here to not detect change by binary itself.
//!  - Replicate events from the tree into the filesystem.
//!  - Download files from remote on events.
//!  - Upload files to remote on events.

//! # Operation
//! The binaries of the server and client should look the same. So both should be able to operate as client and server.
//! During the first iteration there will only be 1 client and 1 server. During later iterations both might act as client and server at the same time.
//! A possibility later on would be to add n-to-n file sync, so not just between two binaries.

//! # Implementation iterations
//! 1. Manually scan. Add .gitignore feature. Small filesystem. Goal: Get a feel for it
//! 2. Build merkle tree. Goal: Keep state of directory scan
//! 3. Increase size and complexity of filesystem. Try around with how change detection happens for many changes (e.g when script instead of users modify files). Switch to periodic scans. Goal: Improve performance and robustness of filesystem scans
//! 4. Remote in server mode, local in client mode. No changes on server, client only sends changes to server. Goal: Proof of concept
//! 5. Changes on remote as well. Goal: Full simple feature-set
//! 6. Run both local and remote in client-server mode. Goal: Switch to decentralized approach
//! 7. Add multithreading (for scans, file upload/download, possibly merkle tree). Goal: performance improvement
//! 8. Add .secure file handling automatic encryption of specified files. n-to-n filesync. Think about conflict resolution without using timestamps (problem with out of sync clocks or concurrent modifications). Goal: It's cool

//! # Implementation details
//! Use update_mmap from blake3 for file hashing (requires mmap feature). Use update_mmap_rayon (requires rayon feature) for large files.
//! Test whether to use rayon or tokio (and possibly io_uring for linux and IoRing for windows) to scan directories and build index.
//! Test memmap2 vs async IO when syncing files. Requires locking files for safety.

use std::path::Path;

use crate::{filesystem::scan::scan_directory_recursive, merkle_tree::merkle_tree::Directory};

mod filesystem;
mod merkle_tree;

const TEST_DIR: &str = "C:\\Dev\\Rust\\syncron";
fn main() {
    let path = Path::new(TEST_DIR);
    scan_directory_recursive(path);
}