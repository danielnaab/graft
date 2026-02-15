//! Snapshot operations for rollback support.
//!
//! Provides file backup and restore functionality for atomic upgrades.

use graft_core::error::{GraftError, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A snapshot manager for creating backups and rollback support.
///
/// Snapshots are stored in a temporary directory with a unique ID.
/// Each snapshot contains copies of files that can be restored on failure.
pub struct SnapshotManager {
    /// Directory where snapshots are stored
    snapshot_dir: PathBuf,
    /// Active snapshots: `snapshot_id` -> paths that were backed up
    snapshots: HashMap<String, Vec<PathBuf>>,
}

impl SnapshotManager {
    /// Create a new snapshot manager.
    ///
    /// Snapshots are stored in `.graft/.snapshots/` by default.
    pub fn new() -> Result<Self> {
        Self::with_directory(".graft/.snapshots")
    }

    /// Create a new snapshot manager with a custom snapshot directory.
    ///
    /// # Arguments
    ///
    /// * `snapshot_dir` - Directory where snapshots will be stored
    pub fn with_directory(snapshot_dir: impl AsRef<Path>) -> Result<Self> {
        let snapshot_dir = snapshot_dir.as_ref().to_path_buf();
        fs::create_dir_all(&snapshot_dir)?;

        Ok(Self {
            snapshot_dir,
            snapshots: HashMap::new(),
        })
    }

    /// Create a snapshot of specified files.
    ///
    /// Files are copied to a temporary location identified by the returned
    /// snapshot ID. If a file doesn't exist, it's recorded as missing
    /// (so we know not to restore it).
    ///
    /// # Arguments
    ///
    /// * `paths` - Paths to files/directories to snapshot
    /// * `base_dir` - Base directory to resolve relative paths from
    ///
    /// # Returns
    ///
    /// Snapshot ID for later restoration or cleanup
    pub fn create_snapshot(
        &mut self,
        paths: &[impl AsRef<Path>],
        base_dir: &Path,
    ) -> Result<String> {
        // Generate unique snapshot ID
        let snapshot_id = format!("snapshot_{}", chrono::Utc::now().timestamp());
        let snapshot_path = self.snapshot_dir.join(&snapshot_id);
        fs::create_dir_all(&snapshot_path)?;

        let mut backed_up = Vec::new();

        for path_ref in paths {
            let path = path_ref.as_ref();
            let full_path = base_dir.join(path);

            if full_path.exists() {
                // Copy file to snapshot
                let snapshot_file = snapshot_path.join(path);

                // Create parent directories if needed
                if let Some(parent) = snapshot_file.parent() {
                    fs::create_dir_all(parent)?;
                }

                fs::copy(&full_path, &snapshot_file)?;
                backed_up.push(full_path);
            } else {
                // Record that file was missing (don't restore it)
                let marker_file = snapshot_path.join(format!("{}.missing", path.display()));
                if let Some(parent) = marker_file.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(marker_file, "")?;
            }
        }

        self.snapshots.insert(snapshot_id.clone(), backed_up);
        Ok(snapshot_id)
    }

    /// Restore files from a snapshot.
    ///
    /// Copies all backed-up files from the snapshot back to their original
    /// locations. Files marked as missing are left unchanged.
    ///
    /// # Arguments
    ///
    /// * `snapshot_id` - Snapshot ID from `create_snapshot`
    /// * `base_dir` - Base directory to restore files to
    pub fn restore_snapshot(&self, snapshot_id: &str, base_dir: &Path) -> Result<()> {
        let snapshot_path = self.snapshot_dir.join(snapshot_id);

        if !snapshot_path.exists() {
            return Err(GraftError::Snapshot(format!(
                "Snapshot not found: {snapshot_id}"
            )));
        }

        // Restore all files from snapshot
        restore_directory(&snapshot_path, base_dir)?;

        Ok(())
    }

    /// Delete a snapshot to free disk space.
    ///
    /// # Arguments
    ///
    /// * `snapshot_id` - Snapshot ID to delete
    pub fn delete_snapshot(&mut self, snapshot_id: &str) -> Result<()> {
        let snapshot_path = self.snapshot_dir.join(snapshot_id);

        if !snapshot_path.exists() {
            return Err(GraftError::Snapshot(format!(
                "Snapshot not found: {snapshot_id}"
            )));
        }

        fs::remove_dir_all(&snapshot_path)?;
        self.snapshots.remove(snapshot_id);

        Ok(())
    }
}

impl Default for SnapshotManager {
    fn default() -> Self {
        Self::new().expect("Failed to create snapshot manager")
    }
}

/// Recursively restore files from snapshot directory to base directory.
fn restore_directory(snapshot_dir: &Path, base_dir: &Path) -> Result<()> {
    for entry in fs::read_dir(snapshot_dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();

        // Skip .missing marker files
        if file_name.to_string_lossy().ends_with(".missing") {
            continue;
        }

        let relative_path = path
            .strip_prefix(snapshot_dir)
            .map_err(|e| GraftError::Snapshot(format!("Failed to compute relative path: {e}")))?;
        let target_path = base_dir.join(relative_path);

        if path.is_dir() {
            fs::create_dir_all(&target_path)?;
            restore_directory(&path, base_dir)?;
        } else {
            // Create parent directories if needed
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&path, &target_path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn create_and_restore_snapshot() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base_dir = temp_dir.path();
        let snapshot_dir = base_dir.join(".snapshots");

        // Create test file
        let test_file = base_dir.join("test.txt");
        fs::write(&test_file, "original content").unwrap();

        // Create snapshot
        let mut manager = SnapshotManager::with_directory(&snapshot_dir).unwrap();
        let snapshot_id = manager.create_snapshot(&["test.txt"], base_dir).unwrap();

        // Modify file
        fs::write(&test_file, "modified content").unwrap();
        assert_eq!(fs::read_to_string(&test_file).unwrap(), "modified content");

        // Restore snapshot
        manager.restore_snapshot(&snapshot_id, base_dir).unwrap();
        assert_eq!(fs::read_to_string(&test_file).unwrap(), "original content");

        // Cleanup
        manager.delete_snapshot(&snapshot_id).unwrap();
    }

    #[test]
    fn snapshot_handles_missing_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base_dir = temp_dir.path();
        let snapshot_dir = base_dir.join(".snapshots");

        // Create snapshot of non-existent file
        let mut manager = SnapshotManager::with_directory(&snapshot_dir).unwrap();
        let snapshot_id = manager.create_snapshot(&["missing.txt"], base_dir).unwrap();

        // Create the file
        let test_file = base_dir.join("missing.txt");
        fs::write(&test_file, "new content").unwrap();

        // Restore should not remove the file (it was missing in snapshot)
        manager.restore_snapshot(&snapshot_id, base_dir).unwrap();

        // File should still exist with new content (not removed)
        assert!(test_file.exists());

        manager.delete_snapshot(&snapshot_id).unwrap();
    }

    #[test]
    fn delete_nonexistent_snapshot_fails() {
        let temp_dir = tempfile::tempdir().unwrap();
        let snapshot_dir = temp_dir.path().join(".snapshots");
        let mut manager = SnapshotManager::with_directory(&snapshot_dir).unwrap();
        let result = manager.delete_snapshot("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn restore_nonexistent_snapshot_fails() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base_dir = temp_dir.path();
        let snapshot_dir = base_dir.join(".snapshots");
        let manager = SnapshotManager::with_directory(&snapshot_dir).unwrap();
        let result = manager.restore_snapshot("nonexistent", base_dir);
        assert!(result.is_err());
    }
}
