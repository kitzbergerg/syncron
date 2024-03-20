use std::path::{Path, PathBuf};

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use jwalk::WalkDirGeneric;

pub fn walk_directory(path: &Path) {
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

fn walk_dir(path: &Path) -> WalkDirGeneric<(JwalkState, ())> {
    // Build global .gitignore
    let gitignore_global = match GitignoreBuilder::new(path).build_global() {
        (_, Some(_err)) => panic!("error building global .gitignore"),
        (gitignore, None) if gitignore.is_empty() => None,
        (gitignore, None) => Some(gitignore),
    };

    // Get .gitignore from parent dirs if there is a .git repo
    let mut gitignore_files = Vec::new();
    let is_in_git_repo = path
        .ancestors()
        .skip(1)
        .inspect(|ancestor| add_ignore_if_exists(&ancestor, &mut gitignore_files))
        .any(|ancestor| ancestor.join(".git").is_dir());
    if is_in_git_repo {
        gitignore_files.reverse();
    } else {
        gitignore_files.clear()
    }

    let initial_state = JwalkState {
        gitignore_global,
        gitignore_files,
        is_in_git_repo,
    };

    WalkDirGeneric::<(JwalkState, ())>::new(path)
        .root_read_dir_state(initial_state)
        .skip_hidden(false)
        .process_read_dir(|_, path, read_dir_state, children| {
            // When there is a new git repo all previous .gitignore are not relevant any more
            if path.join(".git").is_dir() {
                read_dir_state.gitignore_files.clear();
                read_dir_state.is_in_git_repo = true;
            }
            // Check current dir for ignore files
            if read_dir_state.is_in_git_repo {
                add_ignore_if_exists(path, &mut read_dir_state.gitignore_files);
            }

            // Remove ignored files and directories
            children.retain(|dir_entry_result| {
                dir_entry_result
                    .as_ref()
                    .map(|dir_entry| should_retain_path(dir_entry.path(), read_dir_state))
                    .unwrap_or(false)
            });
        })
}

/// Checks if the path should be walked further.
fn should_retain_path(path: PathBuf, read_dir_state: &mut JwalkState) -> bool {
    if !read_dir_state.is_in_git_repo {
        return true;
    }

    let ignored_at_layer = read_dir_state
        .gitignore_files
        .iter()
        .rev()
        .enumerate()
        .find_map(|(layer, glob)| {
            if glob.matched(&path, path.is_dir()).is_ignore() {
                Some(layer)
            } else {
                None
            }
        });
    let whitelisted_at_layer = read_dir_state
        .gitignore_files
        .iter()
        .rev()
        .enumerate()
        .find_map(|(layer, glob)| {
            if glob.matched(&path, path.is_dir()).is_whitelist() {
                Some(layer)
            } else {
                None
            }
        });
    let is_ignored_by_global = read_dir_state
        .gitignore_global
        .clone()
        .is_some_and(|global| global.matched(&path, path.is_dir()).is_ignore());

    match (ignored_at_layer, whitelisted_at_layer) {
        // If the file is not ignored and not whitelisted the global config decides.
        (None, None) => !is_ignored_by_global,
        // If the file is not ignored, but whitelisted it is included.
        (None, Some(_)) => true,
        // If the file is ignored, but not whitelisted it is ignored.
        (Some(_), None) => false,
        // If the file is ignored and whitelisted we need to check which happens first (i.e. if the whitelisting happens after the ignoring it is included).
        (Some(ignore_layer), Some(whitelist_layer)) => whitelist_layer < ignore_layer,
    }
}

/// TODO: add .syncignore files (similar to .git)
fn add_ignore_if_exists(path: &Path, gitignore_files: &mut Vec<Gitignore>) {
    let gitignore_file = path.join(".gitignore");
    let mut gitignore_builder = GitignoreBuilder::new(path);
    if gitignore_file.is_file() {
        if let Some(err) = gitignore_builder.add(gitignore_file) {
            panic!("error adding .gitignore: {err}");
        }
    }
    let gitignore = gitignore_builder
        .build()
        .expect("unable to build gitignore patterns");
    gitignore_files.push(gitignore);
}

#[derive(Debug, Default, Clone)]
struct JwalkState {
    gitignore_global: Option<Gitignore>,
    gitignore_files: Vec<Gitignore>,
    is_in_git_repo: bool,
}
