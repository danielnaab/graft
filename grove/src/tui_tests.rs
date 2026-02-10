//! Unit tests for TUI module
//!
//! These tests verify TUI logic without requiring a real terminal.

use super::*;
use crossterm::event::KeyCode;
use grove_core::{RepoPath, RepoRegistry, RepoStatus, Result};
use std::collections::HashMap;

/// Mock registry for testing TUI logic without real git operations
struct MockRegistry {
    repos: Vec<RepoPath>,
    statuses: HashMap<RepoPath, RepoStatus>,
}

impl MockRegistry {
    fn empty() -> Self {
        Self {
            repos: vec![],
            statuses: HashMap::new(),
        }
    }

    fn with_repos(count: usize) -> Self {
        let repos: Vec<RepoPath> = (0..count)
            .map(|i| RepoPath::new(&format!("/tmp/repo{i}")).unwrap())
            .collect();
        let statuses = repos
            .iter()
            .map(|r| (r.clone(), RepoStatus::new(r.clone())))
            .collect();
        Self { repos, statuses }
    }

    fn with_status(repo_path: &str, mut status: RepoStatus) -> Self {
        let path = RepoPath::new(repo_path).unwrap();
        status.path = path.clone();
        let mut statuses = HashMap::new();
        statuses.insert(path.clone(), status);
        Self {
            repos: vec![path],
            statuses,
        }
    }
}

impl RepoRegistry for MockRegistry {
    fn list_repos(&self) -> Vec<RepoPath> {
        self.repos.clone()
    }

    fn get_status(&self, path: &RepoPath) -> Option<&RepoStatus> {
        self.statuses.get(path)
    }

    fn refresh_all(&mut self) -> Result<grove_core::RefreshStats> {
        Ok(grove_core::RefreshStats {
            successful: self.repos.len(),
            failed: 0,
        })
    }
}

// Test 1: Keybinding handling - quit keys
#[test]
fn handles_quit_with_q_key() {
    let mut app = App::new(MockRegistry::empty());
    assert!(!app.should_quit, "Should not quit initially");

    app.handle_key(KeyCode::Char('q'));
    assert!(app.should_quit, "Should quit after pressing 'q'");
}

#[test]
fn handles_quit_with_esc_key() {
    let mut app = App::new(MockRegistry::empty());
    assert!(!app.should_quit, "Should not quit initially");

    app.handle_key(KeyCode::Esc);
    assert!(app.should_quit, "Should quit after pressing Esc");
}

// Test 2: Navigation with empty list doesn't panic
#[test]
fn navigation_with_empty_list_does_not_panic() {
    let mut app = App::new(MockRegistry::empty());

    // These should not panic even with empty list
    app.next();
    app.previous();

    // Selection should remain None or 0
    assert!(
        app.list_state.selected().is_none() || app.list_state.selected() == Some(0),
        "Selection should be None or 0 for empty list"
    );
}

// Test 3: Navigation wraps at boundaries
#[test]
fn navigation_wraps_from_last_to_first() {
    let mut app = App::new(MockRegistry::with_repos(3));

    // Start at last item (index 2)
    app.list_state.select(Some(2));

    // Press down/next - should wrap to first item
    app.next();
    assert_eq!(
        app.list_state.selected(),
        Some(0),
        "Should wrap from last to first"
    );
}

#[test]
fn navigation_wraps_from_first_to_last() {
    let mut app = App::new(MockRegistry::with_repos(3));

    // Start at first item (index 0)
    app.list_state.select(Some(0));

    // Press up/previous - should wrap to last item
    app.previous();
    assert_eq!(
        app.list_state.selected(),
        Some(2),
        "Should wrap from first to last"
    );
}

#[test]
fn navigation_moves_down_normally() {
    let mut app = App::new(MockRegistry::with_repos(3));

    app.list_state.select(Some(0));
    app.next();
    assert_eq!(app.list_state.selected(), Some(1));

    app.next();
    assert_eq!(app.list_state.selected(), Some(2));
}

#[test]
fn navigation_moves_up_normally() {
    let mut app = App::new(MockRegistry::with_repos(3));

    app.list_state.select(Some(2));
    app.previous();
    assert_eq!(app.list_state.selected(), Some(1));

    app.previous();
    assert_eq!(app.list_state.selected(), Some(0));
}

// Test 4: Format repo line with error
#[test]
fn formats_error_status_correctly() {
    let path = "/tmp/broken-repo".to_string();
    let mut status = RepoStatus::new(RepoPath::new("/tmp/broken-repo").unwrap());
    status.error = Some("Failed to open repository".to_string());

    let line = format_repo_line(path.clone(), Some(&status));

    // Verify line contains error text
    assert_eq!(line.spans.len(), 3, "Should have 3 spans: path, space, error");

    // Check that error message is included
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
    assert!(
        text.contains("error: Failed to open repository"),
        "Should contain error message, got: {text}"
    );
    assert!(text.contains(&path), "Should contain repo path");
}

// Test 5: Format complete status with all fields
#[test]
fn formats_complete_status_with_all_fields() {
    let path = "/tmp/test-repo".to_string();
    let mut status = RepoStatus::new(RepoPath::new("/tmp/test-repo").unwrap());
    status.branch = Some("main".to_string());
    status.is_dirty = true;
    status.ahead = Some(3);
    status.behind = Some(2);

    let line = format_repo_line(path.clone(), Some(&status));

    // Verify spans include all status components
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

    assert!(text.contains(&path), "Should contain repo path");
    assert!(text.contains("[main]"), "Should contain branch name");
    assert!(text.contains("●"), "Should contain dirty indicator");
    assert!(text.contains("↑3"), "Should contain ahead count");
    assert!(text.contains("↓2"), "Should contain behind count");
}

#[test]
fn formats_clean_status_correctly() {
    let path = "/tmp/clean-repo".to_string();
    let mut status = RepoStatus::new(RepoPath::new("/tmp/clean-repo").unwrap());
    status.branch = Some("develop".to_string());
    status.is_dirty = false;
    status.ahead = None;
    status.behind = None;

    let line = format_repo_line(path, Some(&status));
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

    assert!(text.contains("[develop]"), "Should contain branch name");
    assert!(text.contains("○"), "Should contain clean indicator (circle)");
    assert!(
        !text.contains("↑"),
        "Should not show ahead when count is None"
    );
    assert!(
        !text.contains("↓"),
        "Should not show behind when count is None"
    );
}

// Test 6: Format detached HEAD
#[test]
fn formats_detached_head_state() {
    let path = "/tmp/detached-repo".to_string();
    let mut status = RepoStatus::new(RepoPath::new("/tmp/detached-repo").unwrap());
    status.branch = None; // Detached HEAD
    status.is_dirty = false;

    let line = format_repo_line(path, Some(&status));
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

    assert!(
        text.contains("[detached]"),
        "Should show [detached] for None branch, got: {text}"
    );
}

// Test 7: Format loading state
#[test]
fn formats_loading_state() {
    let path = "/tmp/loading-repo".to_string();
    let line = format_repo_line(path.clone(), None);

    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

    assert!(text.contains(&path), "Should contain repo path");
    assert!(
        text.contains("[loading...]"),
        "Should show [loading...] for None status, got: {text}"
    );
}

// Test 8: Keybinding ignores unknown keys
#[test]
fn ignores_unknown_keybindings() {
    let mut app = App::new(MockRegistry::with_repos(3));
    app.list_state.select(Some(1));

    // Press unknown keys
    app.handle_key(KeyCode::Char('x'));
    app.handle_key(KeyCode::Char('a'));
    app.handle_key(KeyCode::F(1));

    // State should not change
    assert_eq!(
        app.list_state.selected(),
        Some(1),
        "Unknown keys should not change selection"
    );
    assert!(!app.should_quit, "Unknown keys should not quit");
}

// Test 9: j/k vim-style navigation
#[test]
fn handles_vim_style_navigation() {
    let mut app = App::new(MockRegistry::with_repos(3));
    app.list_state.select(Some(0));

    // Press j (vim down)
    app.handle_key(KeyCode::Char('j'));
    assert_eq!(
        app.list_state.selected(),
        Some(1),
        "'j' should move down"
    );

    // Press k (vim up)
    app.handle_key(KeyCode::Char('k'));
    assert_eq!(app.list_state.selected(), Some(0), "'k' should move up");
}

#[test]
fn handles_arrow_key_navigation() {
    let mut app = App::new(MockRegistry::with_repos(3));
    app.list_state.select(Some(0));

    // Press Down arrow
    app.handle_key(KeyCode::Down);
    assert_eq!(
        app.list_state.selected(),
        Some(1),
        "Down arrow should move down"
    );

    // Press Up arrow
    app.handle_key(KeyCode::Up);
    assert_eq!(
        app.list_state.selected(),
        Some(0),
        "Up arrow should move up"
    );
}

// Test 10: Only show ahead/behind when >0
#[test]
fn hides_zero_ahead_behind_counts() {
    let path = "/tmp/test".to_string();
    let mut status = RepoStatus::new(RepoPath::new("/tmp/test").unwrap());
    status.branch = Some("main".to_string());
    status.ahead = Some(0);
    status.behind = Some(0);

    let line = format_repo_line(path, Some(&status));
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

    // Zero counts should be filtered out (check the actual implementation)
    // Current implementation uses .filter(|&n| n > 0)
    assert!(
        !text.contains("↑0"),
        "Should not show ahead when count is 0"
    );
    assert!(
        !text.contains("↓0"),
        "Should not show behind when count is 0"
    );
}

#[test]
fn shows_nonzero_ahead_behind_counts() {
    let path = "/tmp/test".to_string();
    let mut status = RepoStatus::new(RepoPath::new("/tmp/test").unwrap());
    status.branch = Some("main".to_string());
    status.ahead = Some(5);
    status.behind = Some(3);

    let line = format_repo_line(path, Some(&status));
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

    assert!(text.contains("↑5"), "Should show ahead count of 5");
    assert!(text.contains("↓3"), "Should show behind count of 3");
}
