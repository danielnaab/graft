//! Runtime abstraction for managing detached process sessions.
//!
//! Provides the [`SessionRuntime`] trait and a [`TmuxRuntime`] implementation
//! that manages sessions via tmux. Future backends (Docker, SSH) can implement
//! the same trait.

use std::path::Path;
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("runtime not available: {0}")]
    NotAvailable(String),
    #[error("session already exists: {0}")]
    SessionExists(String),
    #[error("session not found: {0}")]
    SessionNotFound(String),
    #[error("runtime command failed: {0}")]
    CommandFailed(String),
}

/// A named, detached process session managed by an external runtime.
pub trait SessionRuntime {
    /// Launch a new session running `command` in `working_dir`.
    fn launch(
        &self,
        session_id: &str,
        command: &str,
        working_dir: &Path,
    ) -> Result<(), RuntimeError>;

    /// Check whether a session exists.
    fn exists(&self, session_id: &str) -> Result<bool, RuntimeError>;

    /// Attach the current terminal to a session (blocks until detach).
    fn attach(&self, session_id: &str) -> Result<(), RuntimeError>;

    /// Terminate a session.
    fn stop(&self, session_id: &str) -> Result<(), RuntimeError>;
}

/// Tmux-backed session runtime.
///
/// Each method delegates to a `tmux` subprocess. [`TmuxRuntime::new`] verifies
/// that tmux is installed by running `tmux -V`.
pub struct TmuxRuntime;

impl TmuxRuntime {
    /// Create a new `TmuxRuntime`, verifying that tmux is available.
    ///
    /// Returns `RuntimeError::NotAvailable` if `tmux -V` fails.
    pub fn new() -> Result<Self, RuntimeError> {
        let output = Command::new("tmux")
            .arg("-V")
            .output()
            .map_err(|e| RuntimeError::NotAvailable(format!("tmux not found: {e}")))?;

        if !output.status.success() {
            return Err(RuntimeError::NotAvailable(
                "tmux -V returned non-zero exit code".to_string(),
            ));
        }

        Ok(Self)
    }
}

/// Format a session ID for tmux's `-t` flag with exact matching.
///
/// Tmux does prefix/fnmatch matching by default: `-t foo` would match
/// sessions named `foo`, `foobar`, `foo-v2`, etc. Prefixing with `=`
/// forces an exact match: `-t =foo` matches only `foo`.
fn exact_target(session_id: &str) -> String {
    format!("={session_id}")
}

impl SessionRuntime for TmuxRuntime {
    fn launch(
        &self,
        session_id: &str,
        command: &str,
        working_dir: &Path,
    ) -> Result<(), RuntimeError> {
        // Check for existing session first
        if self.exists(session_id)? {
            return Err(RuntimeError::SessionExists(session_id.to_string()));
        }

        let output = Command::new("tmux")
            .args([
                "new-session",
                "-d",
                "-s",
                session_id,
                "-c",
                &working_dir.to_string_lossy(),
                command,
            ])
            .output()
            .map_err(|e| {
                RuntimeError::CommandFailed(format!("failed to launch tmux session: {e}"))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RuntimeError::CommandFailed(format!(
                "tmux new-session failed: {stderr}"
            )));
        }

        Ok(())
    }

    fn exists(&self, session_id: &str) -> Result<bool, RuntimeError> {
        let target = exact_target(session_id);
        let output = Command::new("tmux")
            .args(["has-session", "-t", &target])
            .output()
            .map_err(|e| {
                RuntimeError::CommandFailed(format!("failed to check tmux session: {e}"))
            })?;

        Ok(output.status.success())
    }

    fn attach(&self, session_id: &str) -> Result<(), RuntimeError> {
        if !self.exists(session_id)? {
            return Err(RuntimeError::SessionNotFound(session_id.to_string()));
        }

        let target = exact_target(session_id);
        let status = Command::new("tmux")
            .args(["attach-session", "-t", &target])
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .map_err(|e| {
                RuntimeError::CommandFailed(format!("failed to attach tmux session: {e}"))
            })?;

        if !status.success() {
            return Err(RuntimeError::CommandFailed(
                "tmux attach-session returned non-zero exit code".to_string(),
            ));
        }

        Ok(())
    }

    fn stop(&self, session_id: &str) -> Result<(), RuntimeError> {
        if !self.exists(session_id)? {
            return Err(RuntimeError::SessionNotFound(session_id.to_string()));
        }

        let target = exact_target(session_id);
        let output = Command::new("tmux")
            .args(["kill-session", "-t", &target])
            .output()
            .map_err(|e| {
                RuntimeError::CommandFailed(format!("failed to stop tmux session: {e}"))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RuntimeError::CommandFailed(format!(
                "tmux kill-session failed: {stderr}"
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmux_available() -> bool {
        Command::new("tmux")
            .arg("-V")
            .output()
            .is_ok_and(|o| o.status.success())
    }

    #[test]
    #[ignore] // requires tmux
    fn tmux_runtime_new_succeeds() {
        if !tmux_available() {
            eprintln!("skipping: tmux not available");
            return;
        }
        let runtime = TmuxRuntime::new();
        assert!(runtime.is_ok());
    }

    #[test]
    #[ignore] // requires tmux
    fn tmux_launch_and_exists() {
        if !tmux_available() {
            eprintln!("skipping: tmux not available");
            return;
        }
        let runtime = TmuxRuntime::new().unwrap();
        let session_id = "graft-test-launch-exists";

        // Clean up any leftover session
        let _ = runtime.stop(session_id);

        runtime
            .launch(session_id, "sleep 60", Path::new("/tmp"))
            .unwrap();
        assert!(runtime.exists(session_id).unwrap());

        runtime.stop(session_id).unwrap();
        assert!(!runtime.exists(session_id).unwrap());
    }

    #[test]
    #[ignore] // requires tmux
    fn tmux_launch_duplicate_fails() {
        if !tmux_available() {
            eprintln!("skipping: tmux not available");
            return;
        }
        let runtime = TmuxRuntime::new().unwrap();
        let session_id = "graft-test-duplicate";

        // Clean up any leftover session
        let _ = runtime.stop(session_id);

        runtime
            .launch(session_id, "sleep 60", Path::new("/tmp"))
            .unwrap();

        let result = runtime.launch(session_id, "sleep 60", Path::new("/tmp"));
        assert!(matches!(result, Err(RuntimeError::SessionExists(_))));

        // Cleanup
        let _ = runtime.stop(session_id);
    }

    #[test]
    #[ignore] // requires tmux
    fn tmux_stop_nonexistent_fails() {
        if !tmux_available() {
            eprintln!("skipping: tmux not available");
            return;
        }
        let runtime = TmuxRuntime::new().unwrap();
        let result = runtime.stop("graft-test-nonexistent-session");
        assert!(matches!(result, Err(RuntimeError::SessionNotFound(_))));
    }
}
