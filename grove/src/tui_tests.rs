//! Unit tests for TUI module
//!
//! These tests verify TUI logic without requiring a real terminal.

use super::*;
use crossterm::event::KeyCode;
use grove_core::{
    CommitInfo, FileChange, FileChangeStatus, RepoDetail, RepoDetailProvider, RepoPath,
    RepoRegistry, RepoStatus, Result,
};
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

/// Mock detail provider for testing
struct MockDetailProvider {
    detail: RepoDetail,
    error: Option<String>,
}

impl MockDetailProvider {
    fn empty() -> Self {
        Self {
            detail: RepoDetail::empty(),
            error: None,
        }
    }

    fn with_detail(detail: RepoDetail) -> Self {
        Self {
            detail,
            error: None,
        }
    }

    fn failing(msg: &str) -> Self {
        Self {
            detail: RepoDetail::empty(),
            error: Some(msg.to_string()),
        }
    }
}

impl RepoDetailProvider for MockDetailProvider {
    fn get_detail(&self, _path: &RepoPath, _max_commits: usize) -> Result<RepoDetail> {
        if let Some(msg) = &self.error {
            Err(grove_core::CoreError::GitError {
                details: msg.clone(),
            })
        } else {
            Ok(self.detail.clone())
        }
    }
}

// ===== Existing tests updated with MockDetailProvider =====

// Test 1: Keybinding handling - quit keys
#[test]
fn handles_quit_with_q_key() {
    let mut app = App::new(MockRegistry::empty(), MockDetailProvider::empty());
    assert!(!app.should_quit, "Should not quit initially");

    app.handle_key(KeyCode::Char('q'));
    assert!(app.should_quit, "Should quit after pressing 'q'");
}

#[test]
fn handles_quit_with_esc_key() {
    let mut app = App::new(MockRegistry::empty(), MockDetailProvider::empty());
    assert!(!app.should_quit, "Should not quit initially");

    app.handle_key(KeyCode::Esc);
    assert!(app.should_quit, "Should quit after pressing Esc");
}

// Test 2: Navigation with empty list doesn't panic
#[test]
fn navigation_with_empty_list_does_not_panic() {
    let mut app = App::new(MockRegistry::empty(), MockDetailProvider::empty());

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
    let mut app = App::new(MockRegistry::with_repos(3), MockDetailProvider::empty());

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
    let mut app = App::new(MockRegistry::with_repos(3), MockDetailProvider::empty());

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
    let mut app = App::new(MockRegistry::with_repos(3), MockDetailProvider::empty());

    app.list_state.select(Some(0));
    app.next();
    assert_eq!(app.list_state.selected(), Some(1));

    app.next();
    assert_eq!(app.list_state.selected(), Some(2));
}

#[test]
fn navigation_moves_up_normally() {
    let mut app = App::new(MockRegistry::with_repos(3), MockDetailProvider::empty());

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
    assert_eq!(
        line.spans.len(),
        3,
        "Should have 3 spans: path, space, error"
    );

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
    assert!(
        text.contains("○"),
        "Should contain clean indicator (circle)"
    );
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
    let mut app = App::new(MockRegistry::with_repos(3), MockDetailProvider::empty());
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
    let mut app = App::new(MockRegistry::with_repos(3), MockDetailProvider::empty());
    app.list_state.select(Some(0));

    // Press j (vim down)
    app.handle_key(KeyCode::Char('j'));
    assert_eq!(app.list_state.selected(), Some(1), "'j' should move down");

    // Press k (vim up)
    app.handle_key(KeyCode::Char('k'));
    assert_eq!(app.list_state.selected(), Some(0), "'k' should move up");
}

#[test]
fn handles_arrow_key_navigation() {
    let mut app = App::new(MockRegistry::with_repos(3), MockDetailProvider::empty());
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

// ===== New Slice 2 tests =====

// Focus management tests
#[test]
fn starts_with_repo_list_focused() {
    let app = App::new(MockRegistry::with_repos(3), MockDetailProvider::empty());
    assert_eq!(app.active_pane, ActivePane::RepoList);
}

#[test]
fn enter_switches_to_detail_pane() {
    let mut app = App::new(MockRegistry::with_repos(3), MockDetailProvider::empty());
    app.handle_key(KeyCode::Enter);
    assert_eq!(app.active_pane, ActivePane::Detail);
}

#[test]
fn tab_switches_to_detail_pane() {
    let mut app = App::new(MockRegistry::with_repos(3), MockDetailProvider::empty());
    app.handle_key(KeyCode::Tab);
    assert_eq!(app.active_pane, ActivePane::Detail);
}

#[test]
fn q_in_detail_returns_to_list() {
    let mut app = App::new(MockRegistry::with_repos(3), MockDetailProvider::empty());
    app.active_pane = ActivePane::Detail;

    app.handle_key(KeyCode::Char('q'));
    assert_eq!(app.active_pane, ActivePane::RepoList);
    assert!(!app.should_quit, "q in detail should NOT quit the app");
}

#[test]
fn esc_in_detail_returns_to_list() {
    let mut app = App::new(MockRegistry::with_repos(3), MockDetailProvider::empty());
    app.active_pane = ActivePane::Detail;

    app.handle_key(KeyCode::Esc);
    assert_eq!(app.active_pane, ActivePane::RepoList);
    assert!(!app.should_quit, "Esc in detail should NOT quit the app");
}

#[test]
fn enter_in_detail_returns_to_list() {
    let mut app = App::new(MockRegistry::with_repos(3), MockDetailProvider::empty());
    app.active_pane = ActivePane::Detail;

    app.handle_key(KeyCode::Enter);
    assert_eq!(app.active_pane, ActivePane::RepoList);
}

#[test]
fn tab_in_detail_returns_to_list() {
    let mut app = App::new(MockRegistry::with_repos(3), MockDetailProvider::empty());
    app.active_pane = ActivePane::Detail;

    app.handle_key(KeyCode::Tab);
    assert_eq!(app.active_pane, ActivePane::RepoList);
}

// Detail scroll tests
#[test]
fn j_in_detail_scrolls_down() {
    let mut app = App::new(MockRegistry::with_repos(3), MockDetailProvider::empty());
    app.active_pane = ActivePane::Detail;

    assert_eq!(app.detail_scroll, 0);
    app.handle_key(KeyCode::Char('j'));
    assert_eq!(app.detail_scroll, 1);
    app.handle_key(KeyCode::Char('j'));
    assert_eq!(app.detail_scroll, 2);
}

#[test]
fn k_in_detail_does_not_go_below_zero() {
    let mut app = App::new(MockRegistry::with_repos(3), MockDetailProvider::empty());
    app.active_pane = ActivePane::Detail;

    assert_eq!(app.detail_scroll, 0);
    app.handle_key(KeyCode::Char('k'));
    assert_eq!(app.detail_scroll, 0, "Scroll should not go below 0");
}

// Cache invalidation tests
#[test]
fn navigation_invalidates_detail_cache() {
    let mut app = App::new(MockRegistry::with_repos(3), MockDetailProvider::empty());

    // Simulate having a cached detail for index 0
    app.cached_detail = Some(RepoDetail::empty());
    app.cached_detail_index = Some(0);

    // Navigate to next repo
    app.handle_key(KeyCode::Char('j'));
    assert_eq!(app.list_state.selected(), Some(1));

    // Ensure detail will be refreshed (index no longer matches)
    app.ensure_detail_loaded();
    assert_eq!(app.cached_detail_index, Some(1));
}

// Detail rendering tests
#[test]
fn build_detail_lines_no_selection() {
    let app = App::new(MockRegistry::empty(), MockDetailProvider::empty());
    // No cached detail
    let lines = app.build_detail_lines();
    let text: String = lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref().to_string()))
        .collect::<String>();
    assert!(text.contains("No repository selected"));
}

#[test]
fn build_detail_lines_with_error() {
    let mut app = App::new(MockRegistry::with_repos(1), MockDetailProvider::empty());
    app.cached_detail = Some(RepoDetail::with_error("git failed".to_string()));
    app.cached_detail_index = Some(0);

    let lines = app.build_detail_lines();
    let text: String = lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref().to_string()))
        .collect::<String>();
    assert!(text.contains("Error: git failed"));
}

#[test]
fn build_detail_lines_with_commits_and_files() {
    let detail = RepoDetail {
        commits: vec![
            CommitInfo {
                hash: "abc1234".to_string(),
                subject: "Fix the bug".to_string(),
                author: "Alice".to_string(),
                relative_date: "2 hours ago".to_string(),
            },
            CommitInfo {
                hash: "def5678".to_string(),
                subject: "Add feature".to_string(),
                author: "Bob".to_string(),
                relative_date: "1 day ago".to_string(),
            },
        ],
        changed_files: vec![
            FileChange {
                path: "src/main.rs".to_string(),
                status: FileChangeStatus::Modified,
            },
            FileChange {
                path: "new_file.txt".to_string(),
                status: FileChangeStatus::Added,
            },
        ],
        error: None,
    };

    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::with_detail(detail),
    );
    app.cached_detail_index = Some(0);
    app.ensure_detail_loaded();

    let lines = app.build_detail_lines();
    let text: String = lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref().to_string()))
        .collect::<Vec<_>>()
        .join(" ");

    assert!(text.contains("Changed Files (2)"), "Should show file count");
    assert!(
        text.contains("src/main.rs"),
        "Should show changed file path"
    );
    assert!(text.contains("new_file.txt"), "Should show added file path");
    assert!(
        text.contains("Recent Commits (2)"),
        "Should show commit count"
    );
    assert!(text.contains("abc1234"), "Should show commit hash");
    assert!(text.contains("Fix the bug"), "Should show commit subject");
    assert!(text.contains("Alice"), "Should show commit author");
    assert!(text.contains("2 hours ago"), "Should show relative date");
}

#[test]
fn build_detail_lines_empty_repo() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::with_detail(RepoDetail::empty()),
    );
    app.cached_detail_index = Some(0);
    app.ensure_detail_loaded();

    let lines = app.build_detail_lines();
    let text: String = lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref().to_string()))
        .collect::<Vec<_>>()
        .join(" ");

    assert!(text.contains("No uncommitted changes"));
    assert!(text.contains("No commits"));
}

// File change indicator tests
#[test]
fn format_file_change_indicators() {
    assert_eq!(
        format_file_change_indicator(&FileChangeStatus::Modified),
        ("M", Color::Yellow)
    );
    assert_eq!(
        format_file_change_indicator(&FileChangeStatus::Added),
        ("A", Color::Green)
    );
    assert_eq!(
        format_file_change_indicator(&FileChangeStatus::Deleted),
        ("D", Color::Red)
    );
    assert_eq!(
        format_file_change_indicator(&FileChangeStatus::Renamed),
        ("R", Color::Cyan)
    );
    assert_eq!(
        format_file_change_indicator(&FileChangeStatus::Copied),
        ("C", Color::Cyan)
    );
    assert_eq!(
        format_file_change_indicator(&FileChangeStatus::Unknown),
        ("?", Color::Gray)
    );
}

// Provider error handling test
#[test]
fn ensure_detail_loaded_converts_provider_error_to_detail_error() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::failing("git timed out"),
    );
    app.list_state.select(Some(0));

    app.ensure_detail_loaded();

    let detail = app
        .cached_detail
        .as_ref()
        .expect("Should have cached detail");
    assert!(
        detail.error.is_some(),
        "Provider error should be converted to RepoDetail error"
    );
    assert!(
        detail.error.as_ref().unwrap().contains("git timed out"),
        "Error message should be preserved, got: {:?}",
        detail.error
    );
    assert!(detail.commits.is_empty(), "Should have no commits on error");
    assert!(
        detail.changed_files.is_empty(),
        "Should have no changed files on error"
    );
}
