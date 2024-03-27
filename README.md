# Syncron: A Rust-based filesystem syncronization tool

Syncron is a highly efficient, eventually consistent filesystem synchronization tool using Rust. This tool is designed to be scalable, supporting everything from simple one-to-one client-server setups to complex decentralized, n-to-n file synchronization networks.

https://github.com/kitzbergerg/syncron/assets/33102273/7585b4de-f4a5-4e5f-b0f7-3d13e3583af7

Note: Syncron is a hobby project in early development. See [Implementation Phases](#implementation-phases) for development status.

## Core components

Syncron is split into three main components:

-   data structure: Using a merkle tree syncron is able to perform efficient comparison and merging of filesystems.
-   filesystem: This component handles operations on the filesystem like scanning directories, file reads/writes and file watchers.
-   cron tasks: To be able to sync the filesystem change detection is necessary. This part handles periodic events like starting filesystem scans, setting up file watchers based on user behaviour and regular comparison against the remote service.

## Implementation Phases

-   [x] Manually scan. Add .gitignore feature. Small filesystem.  
         Goal: Get a feel for it
-   [ ] Build merkle tree.  
         Goal: Keep state of directory scan
-   [ ] Increase size and complexity of filesystem. Try around with how change detection happens for many changes (e.g when script instead of users modify files). Switch to periodic scans.  
         Goal: Improve performance and robustness of filesystem scans
-   [ ] Add file watchers for frequently change directories.  
         Goal: Faster sync for important areas
-   [ ] Remote in server mode, local in client mode. No changes on server, client only sends changes to server.  
         Goal: Proof of concept
-   [ ] Add multithreading (for scans, file upload/download, possibly merkle tree).  
         Goal: performance improvement
-   [ ] Changes on remote as well.  
         Goal: Full simple feature-set
-   [ ] Run both local and remote in client-server mode.  
         Goal: Switch to decentralized approach
-   [ ] Add .secure file handling automatic encryption of specified files. n-to-n file sync. Think about conflict resolution without using timestamps (problem with out of sync clocks or concurrent modifications).  
         Goal: It's cool
