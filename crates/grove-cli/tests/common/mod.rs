//! Shared test helpers for integration tests.

use std::fs;
use std::path::Path;
use std::process::Command;

/// Initialize a git repository with an initial commit, optionally make it dirty.
pub fn init_git_repo(path: &Path, content: &str, make_dirty: bool) {
    Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(path)
        .output()
        .unwrap();

    fs::write(path.join("README.md"), content).unwrap();
    Command::new("git")
        .args(["add", "README.md"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(path)
        .output()
        .unwrap();

    if make_dirty {
        fs::write(path.join("README.md"), format!("{content} modified")).unwrap();
    }
}
