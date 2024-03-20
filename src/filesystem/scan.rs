use std::path::Path;

use ignore::gitignore::Gitignore;
use jwalk::WalkDirGeneric;

pub fn scan_directory_recursive(path: &Path) {
    walk_dir(path)
        .into_iter()
        .map(|file| file.expect("unable to read file"))
        .for_each(|file| {
            if file.file_type().is_file() {
                let filepath = file.path();
                let mut hasher = blake3::Hasher::new();
                hasher.update_mmap_rayon(&filepath).expect("unable to hash");
                let hash = hasher.finalize();

                println!("File: {:?}, Hash: {}", filepath, hash.to_hex())
            }
        });
}

fn walk_dir(path: &Path) -> WalkDirGeneric<(JwalkState, bool)> {
    WalkDirGeneric::<(JwalkState, bool)>::new(path)
        .skip_hidden(false)
        .process_read_dir(|_, path, read_dir_state, children| {
            // Custom state
            let mut gitignore = ignore::gitignore::GitignoreBuilder::new(path);
            let gitignore_file = path.join(".gitignore");
            if gitignore_file.exists() {
                if let Some(err) = gitignore.add(gitignore_file) {
                    panic!("error adding .gitignore: {err}");
                }
            }
            let gitignore = gitignore
                .build()
                .expect("unable to build gitignore patterns");
            read_dir_state.glob_patterns.push(gitignore);

            // Custom filter
            children.retain(|dir_entry_result| {
                dir_entry_result
                    .as_ref()
                    .map(|dir_entry| {
                        let path = dir_entry.path();
                        let should_ignore = read_dir_state
                            .glob_patterns
                            .iter()
                            .any(|glob| glob.matched(&path, path.is_dir()).is_ignore());
                        !should_ignore
                    })
                    .unwrap_or(false)
            });
        })
}

#[derive(Debug, Default, Clone)]
struct JwalkState {
    glob_patterns: Vec<Gitignore>,
}
