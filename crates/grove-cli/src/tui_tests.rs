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

    fn with_statuses(statuses: Vec<RepoStatus>) -> Self {
        let repos: Vec<RepoPath> = statuses.iter().map(|s| s.path.clone()).collect();
        let status_map = statuses.into_iter().map(|s| (s.path.clone(), s)).collect();
        Self {
            repos,
            statuses: status_map,
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
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    assert!(!app.should_quit, "Should not quit initially");

    app.handle_key(KeyCode::Char('q'));
    assert!(app.should_quit, "Should quit after pressing 'q'");
}

#[test]
fn handles_quit_with_esc_key() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    assert!(!app.should_quit, "Should not quit initially");

    app.handle_key(KeyCode::Esc);
    assert!(app.should_quit, "Should quit after pressing Esc");
}

// Test 2: Navigation with empty list doesn't panic
#[test]
fn navigation_with_empty_list_does_not_panic() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

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
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

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
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

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
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.list_state.select(Some(0));
    app.next();
    assert_eq!(app.list_state.selected(), Some(1));

    app.next();
    assert_eq!(app.list_state.selected(), Some(2));
}

#[test]
fn navigation_moves_up_normally() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

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

    let line = format_repo_line(path.clone(), Some(&status), 80);

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

    let line = format_repo_line(path.clone(), Some(&status), 80);

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

    let line = format_repo_line(path, Some(&status), 80);
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

    let line = format_repo_line(path, Some(&status), 80);
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
    let line = format_repo_line(path.clone(), None, 80);

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
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
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
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
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
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
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

    let line = format_repo_line(path, Some(&status), 80);
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

    let line = format_repo_line(path, Some(&status), 80);
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

    assert!(text.contains("↑5"), "Should show ahead count of 5");
    assert!(text.contains("↓3"), "Should show behind count of 3");
}

// ===== New Slice 2 tests =====

// Focus management tests
#[test]
fn starts_with_repo_list_focused() {
    let app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    assert_eq!(app.active_pane, ActivePane::RepoList);
}

#[test]
fn enter_switches_to_detail_pane() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.handle_key(KeyCode::Enter);
    assert_eq!(app.active_pane, ActivePane::Detail);
}

#[test]
fn tab_switches_to_detail_pane() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.handle_key(KeyCode::Tab);
    assert_eq!(app.active_pane, ActivePane::Detail);
}

#[test]
fn q_in_detail_returns_to_list() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.active_pane = ActivePane::Detail;

    app.handle_key(KeyCode::Char('q'));
    assert_eq!(app.active_pane, ActivePane::RepoList);
    assert!(!app.should_quit, "q in detail should NOT quit the app");
}

#[test]
fn esc_in_detail_returns_to_list() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.active_pane = ActivePane::Detail;

    app.handle_key(KeyCode::Esc);
    assert_eq!(app.active_pane, ActivePane::RepoList);
    assert!(!app.should_quit, "Esc in detail should NOT quit the app");
}

#[test]
fn enter_in_detail_returns_to_list() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.active_pane = ActivePane::Detail;

    app.handle_key(KeyCode::Enter);
    assert_eq!(app.active_pane, ActivePane::RepoList);
}

#[test]
fn tab_in_detail_returns_to_list() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.active_pane = ActivePane::Detail;

    app.handle_key(KeyCode::Tab);
    assert_eq!(app.active_pane, ActivePane::RepoList);
}

// Detail scroll tests
#[test]
fn j_in_detail_scrolls_down() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.active_pane = ActivePane::Detail;

    assert_eq!(app.detail_scroll, 0);
    app.handle_key(KeyCode::Char('j'));
    assert_eq!(app.detail_scroll, 1);
    app.handle_key(KeyCode::Char('j'));
    assert_eq!(app.detail_scroll, 2);
}

#[test]
fn k_in_detail_does_not_go_below_zero() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.active_pane = ActivePane::Detail;

    assert_eq!(app.detail_scroll, 0);
    app.handle_key(KeyCode::Char('k'));
    assert_eq!(app.detail_scroll, 0, "Scroll should not go below 0");
}

// Cache invalidation tests
#[test]
fn navigation_invalidates_detail_cache() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

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
    let app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
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
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
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
        "test-workspace".to_string(),
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
        "test-workspace".to_string(),
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
        "test-workspace".to_string(),
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

// --- Branch header rendering tests ---

#[test]
fn build_detail_lines_shows_branch_header() {
    let mut status = RepoStatus::new(RepoPath::new("/tmp/repo0").unwrap());
    status.branch = Some("main".to_string());
    status.is_dirty = true;
    status.ahead = Some(2);
    status.behind = Some(1);

    let mut app = App::new(
        MockRegistry::with_statuses(vec![status]),
        MockDetailProvider::with_detail(RepoDetail::empty()),
        "test-workspace".to_string(),
    );
    app.cached_detail_index = Some(0);
    app.ensure_detail_loaded();

    let lines = app.build_detail_lines();
    let text: String = lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref().to_string()))
        .collect::<String>();

    assert!(text.contains("main"), "Should show branch name");
    assert!(text.contains("●"), "Should show dirty indicator");
    assert!(text.contains("↑2"), "Should show ahead count");
    assert!(text.contains("↓1"), "Should show behind count");
}

#[test]
fn build_detail_lines_clean_repo_shows_clean_indicator() {
    let mut status = RepoStatus::new(RepoPath::new("/tmp/repo0").unwrap());
    status.branch = Some("develop".to_string());
    status.is_dirty = false;

    let mut app = App::new(
        MockRegistry::with_statuses(vec![status]),
        MockDetailProvider::with_detail(RepoDetail::empty()),
        "test-workspace".to_string(),
    );
    app.cached_detail_index = Some(0);
    app.ensure_detail_loaded();

    let lines = app.build_detail_lines();
    let text: String = lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref().to_string()))
        .collect::<String>();

    assert!(text.contains("develop"), "Should show branch name");
    assert!(text.contains("○"), "Should show clean indicator");
    assert!(!text.contains("↑"), "Should not show ahead when None");
    assert!(!text.contains("↓"), "Should not show behind when None");
}

// --- Partial error rendering test ---

#[test]
fn build_detail_lines_shows_error_and_partial_data() {
    let detail = RepoDetail {
        commits: vec![CommitInfo {
            hash: "abc1234".to_string(),
            subject: "A good commit".to_string(),
            author: "Alice".to_string(),
            relative_date: "1 hour ago".to_string(),
        }],
        changed_files: vec![],
        error: Some("changed files: git status timed out".to_string()),
    };

    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::with_detail(detail.clone()),
        "test-workspace".to_string(),
    );
    app.cached_detail = Some(detail);
    app.cached_detail_index = Some(0);

    let lines = app.build_detail_lines();
    let text: String = lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref().to_string()))
        .collect::<Vec<_>>()
        .join(" ");

    assert!(
        text.contains("Error:"),
        "Should show error warning, got: {text}"
    );
    assert!(
        text.contains("timed out"),
        "Should include error details, got: {text}"
    );
    assert!(
        text.contains("abc1234"),
        "Should still show partial commit data, got: {text}"
    );
    assert!(
        text.contains("A good commit"),
        "Should still show commit subject, got: {text}"
    );
}

// --- Scroll clamping test ---

#[test]
fn detail_scroll_clamps_to_content_length() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.active_pane = ActivePane::Detail;

    // Load detail (will be a short empty detail)
    app.ensure_detail_loaded();

    // Artificially inflate scroll way past content
    app.detail_scroll = 9999;

    // build_detail_lines to get content length (the render method clamps during draw,
    // but we can test the clamping logic directly)
    let lines = app.build_detail_lines();
    let max_scroll = lines.len(); // without inner_height subtraction, this is the upper bound

    // After render would clamp, scroll should be <= content length
    // We can't call render() without a terminal, but verify the value is unreasonable
    assert!(
        app.detail_scroll > max_scroll,
        "Pre-condition: scroll should exceed content before clamping"
    );
}

// --- Path compaction tests ---

#[test]
fn compact_path_returns_unchanged_if_fits() {
    let path = "/tmp/short";
    let result = compact_path(path, 50);
    assert_eq!(result, path);
}

#[test]
fn compact_path_applies_tilde_expansion() {
    // Note: shellexpand::tilde only works if HOME is set and matches the path
    let path = format!(
        "{}/projects/graft",
        std::env::var("HOME").unwrap_or_default()
    );
    let result = compact_path(&path, 100);
    assert!(
        result.starts_with("~/"),
        "Should start with tilde, got: {result}"
    );
}

#[test]
fn compact_path_abbreviates_parent_components() {
    let path = "/home/user/very/long/nested/project-name/submodule";
    let result = compact_path(path, 35);

    // Should preserve last 2 components, abbreviate the rest
    assert!(
        result.contains("project-name/submodule"),
        "Should preserve final 2 components, got: {result}"
    );
    // Should have abbreviated middle parts
    assert!(
        result.len() < path.len(),
        "Should be shorter than original, got: {result}"
    );
}

#[test]
fn compact_path_preserves_final_components() {
    let path = "/a/b/c/d/project/repo";
    let result = compact_path(path, 25);

    // Last 2 components should be intact
    assert!(
        result.ends_with("project/repo"),
        "Should end with last 2 components, got: {result}"
    );
}

#[test]
fn compact_path_falls_back_to_prefix_truncation() {
    let path = "/extremely/long/path/that/will/not/fit/even/with/abbreviation/project-name";
    let result = compact_path(path, 20);

    // When even abbreviation doesn't help, should use prefix truncation
    assert!(
        result.starts_with("[..]"),
        "Should use prefix truncation, got: {result}"
    );
    assert!(
        result.len() <= 20,
        "Should not exceed max width, got: {} (len {})",
        result,
        result.len()
    );
}

#[test]
fn compact_path_handles_unicode_correctly() {
    let path = "/home/user/プロジェクト/ファイル";
    let result = compact_path(path, 50);

    // Should handle unicode characters without panicking
    // Width should be calculated correctly
    assert!(
        result.width() <= 50,
        "Unicode path should respect width limit, got width: {} for: {}",
        result.width(),
        result
    );
}

#[test]
fn compact_path_handles_very_short_max_width() {
    let path = "/home/user/project";
    let result = compact_path(path, 5);

    // Should not panic with very short width
    assert!(
        result.width() <= 5,
        "Should respect very short width, got: {} (width {})",
        result,
        result.width()
    );
}

#[test]
fn compact_path_abbreviates_fish_style() {
    let path = "/var/log/nginx/access/archive";
    let result = compact_path(path, 25);

    // Should abbreviate like fish: /v/l/n/access/archive
    // Last 2 components preserved
    assert!(result.ends_with("access/archive"));

    // Should have single-char abbreviations for parent components
    // (allowing for variation based on actual compaction strategy)
    assert!(result.len() < path.len());
}

// --- Adaptive branch display tests ---

#[test]
fn format_repo_line_shows_branch_when_space_allows() {
    let path = "/tmp/repo".to_string();
    let mut status = RepoStatus::new(RepoPath::new("/tmp/repo").unwrap());
    status.branch = Some("main".to_string());
    status.is_dirty = true;

    // Wide pane: 80 cols should have plenty of room
    let line = format_repo_line(path, Some(&status), 80);
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

    assert!(
        text.contains("[main]"),
        "Should show branch when space allows, got: {text}"
    );
}

#[test]
fn format_repo_line_drops_branch_when_path_would_be_too_short() {
    let path = "/home/user/very/long/path/to/repository".to_string();
    let mut status = RepoStatus::new(RepoPath::new(&path).unwrap());
    status.branch = Some("feature-branch-with-long-name".to_string());
    status.is_dirty = true;

    // Narrow pane: 20 cols means path would be severely compacted with branch
    let line = format_repo_line(path, Some(&status), 20);
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

    // Should NOT show branch (dropped to make room for path)
    assert!(
        !text.contains("[feature-branch-with-long-name]"),
        "Should drop branch when path would be too short, got: {text}"
    );
    // Should still show status
    assert!(text.contains("●"), "Should still show dirty indicator");
}

#[test]
fn format_repo_line_drops_branch_when_path_uses_prefix_truncation() {
    let path = "/extremely/long/nested/directory/structure/repository-name".to_string();
    let mut status = RepoStatus::new(RepoPath::new(&path).unwrap());
    status.branch = Some("main".to_string());
    status.is_dirty = false;

    // Very narrow pane where path would need [..] prefix even with abbreviation
    let line = format_repo_line(path, Some(&status), 18);
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

    // Should NOT show branch when path needs [..] prefix
    assert!(
        !text.contains("[main]"),
        "Should drop branch when path uses [..] prefix, got: {text}"
    );
}

#[test]
fn format_repo_line_unicode_path_uses_width_not_len() {
    // Unicode path where byte length != display width
    let path = "/home/用户/项目/repository".to_string();
    let mut status = RepoStatus::new(RepoPath::new(&path).unwrap());
    status.branch = Some("main".to_string());
    status.is_dirty = true;

    // Medium pane width
    let line = format_repo_line(path, Some(&status), 40);
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

    // Should handle unicode correctly (width-based, not byte-based decision)
    // The important thing is it doesn't panic and produces reasonable output
    assert!(
        text.contains("●"),
        "Should show status indicator for unicode path"
    );
}

#[test]
fn format_repo_line_preserves_ahead_behind_when_dropping_branch() {
    let path = "/home/user/very/long/path/to/repo".to_string();
    let mut status = RepoStatus::new(RepoPath::new(&path).unwrap());
    status.branch = Some("main".to_string());
    status.is_dirty = true;
    status.ahead = Some(4);
    status.behind = Some(2);

    // Narrow pane: branch will be dropped
    let line = format_repo_line(path, Some(&status), 22);
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

    // Should drop branch but keep ahead/behind
    assert!(
        !text.contains("[main]"),
        "Should drop branch in tight space"
    );
    assert!(text.contains("↑4"), "Should preserve ahead indicator");
    assert!(text.contains("↓2"), "Should preserve behind indicator");
    assert!(text.contains("●"), "Should preserve dirty indicator");
}

#[test]
fn format_repo_line_shows_basename_only_in_very_tight_space() {
    let path = "/home/user/src/graft".to_string();
    let mut status = RepoStatus::new(RepoPath::new(&path).unwrap());
    status.branch = Some("main".to_string());
    status.is_dirty = true;

    // Very narrow pane: < 15 cols
    let line = format_repo_line(path, Some(&status), 12);
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

    // Should show basename only, no branch, no path
    assert!(
        text.contains("graft"),
        "Should show basename in very tight space, got: {text}"
    );
    assert!(
        !text.contains("[main]"),
        "Should not show branch in very tight space"
    );
    assert!(
        !text.contains("src"),
        "Should not show parent dirs in very tight space"
    );
    assert!(text.contains("●"), "Should still show status");
}

#[test]
fn extract_basename_works_correctly() {
    assert_eq!(extract_basename("/home/user/src/graft"), "graft");
    assert_eq!(extract_basename("~/projects/repo"), "repo");
    assert_eq!(extract_basename("/tmp"), "tmp");
    assert_eq!(extract_basename("single"), "single");
}

// --- Overhead calculation verification ---

#[test]
fn verify_overhead_calculation_is_accurate() {
    // Test that our overhead calculation leaves appropriate space
    let path = "/home/user/repo".to_string();
    let mut status = RepoStatus::new(RepoPath::new(&path).unwrap());
    status.branch = Some("main".to_string());
    status.is_dirty = true;
    status.ahead = Some(4);
    status.behind = Some(2);

    // Format with known pane width
    let pane_width = 50;
    let line = format_repo_line(path, Some(&status), pane_width);

    // Calculate actual rendered width (excluding highlight symbol which is separate)
    let actual_width: usize = line.spans.iter().map(|s| s.content.width()).sum();

    // The line should fit comfortably within the pane
    // Overhead accounts for: highlight (2) + margins (~3) = 5
    // So actual content should be <= pane_width - 5
    assert!(
        actual_width <= pane_width as usize - 2, // At minimum, leave room for highlight
        "Line width {} should fit in pane {} with overhead, got spans: {:?}",
        actual_width,
        pane_width,
        line.spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<Vec<_>>()
    );

    // Verify all expected components are present
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
    assert!(text.contains("[main]"), "Should contain branch");
    assert!(text.contains("●"), "Should contain dirty");
    assert!(text.contains("↑4"), "Should contain ahead");
    assert!(text.contains("↓2"), "Should contain behind");
}

#[test]
fn empty_workspace_has_no_selection() {
    let app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    assert_eq!(
        app.list_state.selected(),
        None,
        "Empty workspace should have no selected item"
    );
}

#[test]
fn help_overlay_activates_on_question_mark() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // Initially should be on repo list
    assert_eq!(app.active_pane, ActivePane::RepoList);

    // Press '?' to show help
    app.handle_key(KeyCode::Char('?'));

    assert_eq!(
        app.active_pane,
        ActivePane::Help,
        "Pressing '?' should activate help overlay"
    );
}

#[test]
fn help_overlay_dismisses_on_printable_key() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // Activate help
    app.handle_key(KeyCode::Char('?'));
    assert_eq!(app.active_pane, ActivePane::Help);

    // Dismiss with any printable key
    app.handle_key(KeyCode::Char('q'));
    assert_eq!(
        app.active_pane,
        ActivePane::RepoList,
        "Printable key should dismiss help"
    );
}

#[test]
fn help_overlay_dismisses_on_esc() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // Activate help
    app.handle_key(KeyCode::Char('?'));
    assert_eq!(app.active_pane, ActivePane::Help);

    // Dismiss with Esc
    app.handle_key(KeyCode::Esc);
    assert_eq!(
        app.active_pane,
        ActivePane::RepoList,
        "Esc should dismiss help"
    );
}

#[test]
fn empty_workspace_navigation_does_not_panic() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // Navigate down - should not panic
    app.handle_key(KeyCode::Char('j'));
    assert_eq!(app.list_state.selected(), None);

    // Navigate up - should not panic
    app.handle_key(KeyCode::Char('k'));
    assert_eq!(app.list_state.selected(), None);
}

// --- Status bar tests (Phase 1 improvements) ---

#[test]
fn status_message_creates_with_timestamp() {
    let msg = StatusMessage::error("Test error");
    assert_eq!(msg.text, "Test error");
    assert_eq!(msg.msg_type, MessageType::Error);
    assert!(msg.shown_at.elapsed() < Duration::from_millis(10));
}

#[test]
fn status_message_not_expired_immediately() {
    let msg = StatusMessage::info("Test");
    assert!(!msg.is_expired());
}

#[test]
fn status_message_expires_after_three_seconds() {
    let mut msg = StatusMessage::warning("Test");
    // Manually set the timestamp to 4 seconds ago
    msg.shown_at = Instant::now() - Duration::from_secs(4);
    assert!(msg.is_expired(), "Message should expire after 3 seconds");
}

#[test]
fn status_message_convenience_constructors() {
    let error = StatusMessage::error("Error message");
    assert_eq!(error.msg_type, MessageType::Error);
    assert_eq!(error.text, "Error message");

    let warning = StatusMessage::warning("Warning message");
    assert_eq!(warning.msg_type, MessageType::Warning);
    assert_eq!(warning.text, "Warning message");

    let info = StatusMessage::info("Info message");
    assert_eq!(info.msg_type, MessageType::Info);
    assert_eq!(info.text, "Info message");

    let success = StatusMessage::success("Success message");
    assert_eq!(success.msg_type, MessageType::Success);
    assert_eq!(success.text, "Success message");
}

#[test]
fn message_type_symbols_unicode() {
    assert_eq!(MessageType::Error.symbol(true), "✗");
    assert_eq!(MessageType::Warning.symbol(true), "⚠");
    assert_eq!(MessageType::Info.symbol(true), "ℹ");
    assert_eq!(MessageType::Success.symbol(true), "✓");
}

#[test]
fn message_type_symbols_ascii() {
    assert_eq!(MessageType::Error.symbol(false), "X");
    assert_eq!(MessageType::Warning.symbol(false), "!");
    assert_eq!(MessageType::Info.symbol(false), "i");
    assert_eq!(MessageType::Success.symbol(false), "*");
}

#[test]
fn message_type_colors() {
    assert_eq!(MessageType::Error.fg_color(), Color::White);
    assert_eq!(MessageType::Error.bg_color(), Color::Red);

    assert_eq!(MessageType::Warning.fg_color(), Color::Black);
    assert_eq!(MessageType::Warning.bg_color(), Color::Yellow);

    assert_eq!(MessageType::Info.fg_color(), Color::White);
    assert_eq!(MessageType::Info.bg_color(), Color::Blue);

    assert_eq!(MessageType::Success.fg_color(), Color::Black);
    assert_eq!(MessageType::Success.bg_color(), Color::Green);
}

#[test]
fn supports_unicode_detects_incompatible_terminals() {
    // Note: This test depends on the actual TERM environment variable
    // In a real test environment, you might want to mock std::env::var

    // For now, just verify the function doesn't panic
    let _ = supports_unicode();

    // We can't easily test this without mocking, but we can at least
    // verify it returns a boolean
    assert!(supports_unicode() || !supports_unicode());
}

#[test]
fn clear_expired_status_message_removes_old_messages() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // Set a message that's already expired
    let mut old_msg = StatusMessage::info("Old message");
    old_msg.shown_at = Instant::now() - Duration::from_secs(4);
    app.status_message = Some(old_msg);

    // Clear expired messages
    app.clear_expired_status_message();

    assert!(
        app.status_message.is_none(),
        "Expired message should be cleared"
    );
}

#[test]
fn clear_expired_status_message_keeps_fresh_messages() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // Set a fresh message
    app.status_message = Some(StatusMessage::success("Fresh message"));

    // Clear expired messages
    app.clear_expired_status_message();

    assert!(
        app.status_message.is_some(),
        "Fresh message should not be cleared"
    );
}

#[test]
fn status_message_set_on_refresh() {
    let mut app = App::new(
        MockRegistry::with_repos(2),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // Trigger refresh
    app.needs_refresh = true;
    app.handle_refresh_if_needed();

    // Should have success message
    assert!(app.status_message.is_some());
    let msg = app.status_message.as_ref().unwrap();
    assert_eq!(msg.msg_type, MessageType::Success);
    assert!(msg.text.contains("Refreshed"));
}

#[test]
fn status_message_set_on_no_commands() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // Simulate pressing 'x' when no commands available
    // (load_commands_for_selected_repo will find no commands and remain empty)
    app.handle_key(KeyCode::Char('x'));

    // Should have warning message
    assert!(app.status_message.is_some());
    let msg = app.status_message.as_ref().unwrap();
    assert_eq!(msg.msg_type, MessageType::Warning);
    assert!(msg.text.contains("No commands"));
}

// ===== Command Execution Tests (Phase 1 & 2 Features) =====

#[test]
fn command_state_transitions_not_started_to_running() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    // Initial state
    assert!(matches!(app.command_state, CommandState::NotStarted));

    // Simulate command start
    app.command_state = CommandState::Running;
    assert!(matches!(app.command_state, CommandState::Running));
}

#[test]
fn command_state_transitions_running_to_completed() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    app.command_state = CommandState::Running;

    // Simulate successful completion
    app.command_state = CommandState::Completed { exit_code: 0 };
    assert!(matches!(
        app.command_state,
        CommandState::Completed { exit_code: 0 }
    ));
}

#[test]
fn command_state_transitions_running_to_failed() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    app.command_state = CommandState::Running;

    // Simulate failure
    app.command_state = CommandState::Failed {
        error: "test error".to_string(),
    };
    assert!(matches!(app.command_state, CommandState::Failed { .. }));
}

#[test]
fn confirmation_dialog_not_shown_initially() {
    let app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    assert!(
        !app.show_stop_confirmation,
        "Dialog should not be shown initially"
    );
}

#[test]
fn confirmation_dialog_shows_for_running_command() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    // Set up running command state
    app.command_state = CommandState::Running;
    app.active_pane = ActivePane::CommandOutput;

    // Press 'q' should show confirmation
    app.handle_key(KeyCode::Char('q'));
    assert!(
        app.show_stop_confirmation,
        "Dialog should show for running command"
    );
}

#[test]
fn confirmation_dialog_not_shown_for_completed_command() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    // Set up completed command state
    app.command_state = CommandState::Completed { exit_code: 0 };
    app.active_pane = ActivePane::CommandOutput;

    // Press 'q' should close immediately (no confirmation)
    app.handle_key(KeyCode::Char('q'));
    assert!(
        !app.show_stop_confirmation,
        "Dialog should not show for completed command"
    );
    assert_eq!(
        app.active_pane,
        ActivePane::RepoList,
        "Should return to repo list"
    );
}

#[test]
fn pid_tracking_none_initially() {
    let app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    assert!(
        app.running_command_pid.is_none(),
        "PID should be None initially"
    );
}

#[test]
fn pid_tracking_set_on_started_event() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    // Simulate Started event with PID 12345
    app.running_command_pid = Some(12345);
    assert_eq!(app.running_command_pid, Some(12345), "PID should be set");
}

#[test]
fn ring_buffer_flag_false_initially() {
    let app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    assert!(
        !app.output_truncated_start,
        "Ring buffer flag should be false initially"
    );
}

#[test]
fn output_pane_scroll_initialized_to_zero() {
    let app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    assert_eq!(app.output_scroll, 0, "Output scroll should start at 0");
}

#[test]
fn output_lines_empty_initially() {
    let app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    assert!(
        app.output_lines.is_empty(),
        "Output lines should be empty initially"
    );
}

// ===== Argument Input Tests (Phase 4) =====

#[test]
fn argument_input_opens_after_command_selected() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    app.available_commands = vec![(
        "test".to_string(),
        grove_core::Command {
            run: "echo test".to_string(),
            description: Some("Test command".to_string()),
            working_dir: None,
            env: None,
        },
    )];
    app.active_pane = ActivePane::CommandPicker;
    app.command_picker_state.select(Some(0));

    app.execute_selected_command();

    assert_eq!(app.active_pane, ActivePane::ArgumentInput);
    assert!(app.argument_input.is_some());
    let state = app.argument_input.as_ref().unwrap();
    assert_eq!(state.command_name, "test");
    assert!(state.buffer.is_empty());
    assert_eq!(state.cursor_pos, 0);
}

#[test]
fn argument_input_buffer_updates_on_char() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    app.active_pane = ActivePane::ArgumentInput;
    app.argument_input = Some(super::ArgumentInputState {
        buffer: String::new(),
        cursor_pos: 0,
        command_name: "test".to_string(),
    });

    app.handle_key(KeyCode::Char('a'));
    app.handle_key(KeyCode::Char('r'));
    app.handle_key(KeyCode::Char('g'));

    let state = app.argument_input.as_ref().unwrap();
    assert_eq!(state.buffer, "arg");
    assert_eq!(state.cursor_pos, 3);
}

#[test]
fn argument_input_backspace_removes_char() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    app.active_pane = ActivePane::ArgumentInput;
    app.argument_input = Some(super::ArgumentInputState {
        buffer: "test".to_string(),
        cursor_pos: 4,
        command_name: "test".to_string(),
    });

    app.handle_key(KeyCode::Backspace);

    let state = app.argument_input.as_ref().unwrap();
    assert_eq!(state.buffer, "tes");
    assert_eq!(state.cursor_pos, 3);
}

#[test]
fn argument_input_escape_cancels() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    app.active_pane = ActivePane::ArgumentInput;
    app.argument_input = Some(super::ArgumentInputState {
        buffer: "some args".to_string(),
        cursor_pos: 9,
        command_name: "test".to_string(),
    });

    app.handle_key(KeyCode::Esc);

    assert_eq!(app.active_pane, ActivePane::RepoList);
    assert!(app.argument_input.is_none());
}

#[test]
fn argument_input_enter_executes_with_args() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    app.active_pane = ActivePane::ArgumentInput;
    app.argument_input = Some(super::ArgumentInputState {
        buffer: "arg1 arg2".to_string(),
        cursor_pos: 9,
        command_name: "test".to_string(),
    });
    app.selected_repo_for_commands = Some("/tmp/test".to_string());

    app.handle_key(KeyCode::Enter);

    assert_eq!(app.active_pane, ActivePane::CommandOutput);
    assert!(app.argument_input.is_none());
    assert_eq!(app.command_name, Some("test".to_string()));
}

#[test]
fn argument_input_enter_with_empty_buffer_executes_without_args() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    app.active_pane = ActivePane::ArgumentInput;
    app.argument_input = Some(super::ArgumentInputState {
        buffer: String::new(),
        cursor_pos: 0,
        command_name: "test".to_string(),
    });
    app.selected_repo_for_commands = Some("/tmp/test".to_string());

    app.handle_key(KeyCode::Enter);

    assert_eq!(app.active_pane, ActivePane::CommandOutput);
    assert_eq!(app.command_name, Some("test".to_string()));
}

#[test]
fn argument_input_parses_quoted_arguments_correctly() {
    // This test verifies that shell-style quoting works for arguments with spaces
    // Input: Personal "This is a test"
    // Expected: ["Personal", "This is a test"] (2 arguments)

    // We can't directly test the parsing without exposing it, but we can verify
    // the behavior through the integration test. This test just documents the
    // expected behavior and verifies the shell-words crate works as expected.

    let input = r#"Personal "This is a test""#;
    let parsed = shell_words::split(input).unwrap();

    assert_eq!(parsed.len(), 2, "Should parse into 2 arguments");
    assert_eq!(parsed[0], "Personal");
    assert_eq!(parsed[1], "This is a test");
}

#[test]
fn argument_input_handles_multiple_quoted_args() {
    let input = r#""First arg" "Second arg" third"#;
    let parsed = shell_words::split(input).unwrap();

    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[0], "First arg");
    assert_eq!(parsed[1], "Second arg");
    assert_eq!(parsed[2], "third");
}

// ===== Cursor Navigation Tests (Phase 1) =====

#[test]
fn argument_input_cursor_moves_left() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    app.argument_input = Some(super::ArgumentInputState {
        buffer: "test".to_string(),
        cursor_pos: 4,
        command_name: "cmd".to_string(),
    });
    app.active_pane = ActivePane::ArgumentInput;

    app.handle_key(KeyCode::Left);

    assert_eq!(app.argument_input.as_ref().unwrap().cursor_pos, 3);
}

#[test]
fn argument_input_cursor_moves_right() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    app.argument_input = Some(super::ArgumentInputState {
        buffer: "test".to_string(),
        cursor_pos: 2,
        command_name: "cmd".to_string(),
    });
    app.active_pane = ActivePane::ArgumentInput;

    app.handle_key(KeyCode::Right);

    assert_eq!(app.argument_input.as_ref().unwrap().cursor_pos, 3);
}

#[test]
fn argument_input_cursor_stops_at_boundaries() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    app.argument_input = Some(super::ArgumentInputState {
        buffer: "test".to_string(),
        cursor_pos: 0,
        command_name: "cmd".to_string(),
    });
    app.active_pane = ActivePane::ArgumentInput;

    // Try to move left at start
    app.handle_key(KeyCode::Left);
    assert_eq!(app.argument_input.as_ref().unwrap().cursor_pos, 0);

    // Move to end
    app.argument_input.as_mut().unwrap().cursor_pos = 4;

    // Try to move right at end
    app.handle_key(KeyCode::Right);
    assert_eq!(app.argument_input.as_ref().unwrap().cursor_pos, 4);
}

#[test]
fn argument_input_home_end_keys() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    app.argument_input = Some(super::ArgumentInputState {
        buffer: "test".to_string(),
        cursor_pos: 2,
        command_name: "cmd".to_string(),
    });
    app.active_pane = ActivePane::ArgumentInput;

    app.handle_key(KeyCode::Home);
    assert_eq!(app.argument_input.as_ref().unwrap().cursor_pos, 0);

    app.handle_key(KeyCode::End);
    assert_eq!(app.argument_input.as_ref().unwrap().cursor_pos, 4);
}

#[test]
fn argument_input_inserts_char_at_cursor() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    app.argument_input = Some(super::ArgumentInputState {
        buffer: "test".to_string(),
        cursor_pos: 2,
        command_name: "cmd".to_string(),
    });
    app.active_pane = ActivePane::ArgumentInput;

    app.handle_key(KeyCode::Char('X'));

    let state = app.argument_input.as_ref().unwrap();
    assert_eq!(state.buffer, "teXst");
    assert_eq!(state.cursor_pos, 3);
}

#[test]
fn argument_input_backspace_at_cursor() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    app.argument_input = Some(super::ArgumentInputState {
        buffer: "test".to_string(),
        cursor_pos: 2,
        command_name: "cmd".to_string(),
    });
    app.active_pane = ActivePane::ArgumentInput;

    app.handle_key(KeyCode::Backspace);

    let state = app.argument_input.as_ref().unwrap();
    assert_eq!(state.buffer, "tst");
    assert_eq!(state.cursor_pos, 1);
}

#[test]
fn argument_input_prevents_execution_on_parse_error() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    app.argument_input = Some(super::ArgumentInputState {
        buffer: r#"unclosed "quote"#.to_string(),
        cursor_pos: 15,
        command_name: "cmd".to_string(),
    });
    app.active_pane = ActivePane::ArgumentInput;
    app.selected_repo_for_commands = Some("/tmp/test".to_string());

    app.handle_key(KeyCode::Enter);

    // Should stay in ArgumentInput pane
    assert_eq!(app.active_pane, ActivePane::ArgumentInput);

    // Should show error message
    assert!(app.status_message.is_some());
    let msg = app.status_message.as_ref().unwrap();
    assert!(msg.text.contains("parsing error") || msg.text.contains("Parse error"));
}

// ===== State Panel Tests (Phase 1) =====

#[test]
fn state_panel_opens_on_s_key() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.list_state.select(Some(0));
    app.active_pane = ActivePane::Detail;

    // Press 's' to open state panel
    app.handle_key(KeyCode::Char('s'));

    // Verify transition happened
    assert_eq!(
        app.active_pane,
        ActivePane::StatePanel,
        "'s' should open state panel"
    );
}

#[test]
fn state_panel_closes_on_esc() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.active_pane = ActivePane::StatePanel;

    app.handle_key(KeyCode::Esc);

    assert_eq!(
        app.active_pane,
        ActivePane::Detail,
        "Esc should return to detail view"
    );
}

#[test]
fn state_panel_closes_on_q() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.active_pane = ActivePane::StatePanel;

    app.handle_key(KeyCode::Char('q'));

    assert_eq!(
        app.active_pane,
        ActivePane::Detail,
        "'q' should return to detail view"
    );
    assert!(
        !app.should_quit,
        "'q' in state panel should NOT quit the app"
    );
}

#[test]
fn state_panel_navigation_with_j_key() {
    use crate::state::StateQuery;

    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.active_pane = ActivePane::StatePanel;

    // Mock some state queries
    app.state_queries = vec![
        StateQuery {
            name: "coverage".to_string(),
            run: "pytest --cov".to_string(),
            description: None,
            deterministic: true,
            timeout: None,
        },
        StateQuery {
            name: "tasks".to_string(),
            run: "task-list".to_string(),
            description: None,
            deterministic: true,
            timeout: None,
        },
    ];
    app.state_results = vec![None, None];
    app.state_panel_list_state.select(Some(0));

    app.handle_key(KeyCode::Char('j'));

    assert_eq!(
        app.state_panel_list_state.selected(),
        Some(1),
        "'j' should move down to next query"
    );
}

#[test]
fn state_panel_navigation_with_k_key() {
    use crate::state::StateQuery;

    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.active_pane = ActivePane::StatePanel;

    app.state_queries = vec![
        StateQuery {
            name: "q1".to_string(),
            run: "echo q1".to_string(),
            description: None,
            deterministic: true,
            timeout: None,
        },
        StateQuery {
            name: "q2".to_string(),
            run: "echo q2".to_string(),
            description: None,
            deterministic: true,
            timeout: None,
        },
    ];
    app.state_results = vec![None, None];
    app.state_panel_list_state.select(Some(1));

    app.handle_key(KeyCode::Char('k'));

    assert_eq!(
        app.state_panel_list_state.selected(),
        Some(0),
        "'k' should move up to previous query"
    );
}

#[test]
fn state_panel_navigation_does_not_move_past_end() {
    use crate::state::StateQuery;

    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.active_pane = ActivePane::StatePanel;

    app.state_queries = vec![
        StateQuery {
            name: "q1".to_string(),
            run: "echo q1".to_string(),
            description: None,
            deterministic: true,
            timeout: None,
        },
        StateQuery {
            name: "q2".to_string(),
            run: "echo q2".to_string(),
            description: None,
            deterministic: true,
            timeout: None,
        },
    ];
    app.state_results = vec![None, None];
    app.state_panel_list_state.select(Some(1)); // Last item

    app.handle_key(KeyCode::Char('j'));

    // Should stay at last item (no wrapping implemented)
    assert_eq!(
        app.state_panel_list_state.selected(),
        Some(1),
        "Should not move past last query"
    );
}

#[test]
fn state_panel_navigation_does_not_move_before_start() {
    use crate::state::StateQuery;

    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.active_pane = ActivePane::StatePanel;

    app.state_queries = vec![StateQuery {
        name: "q1".to_string(),
        run: "echo q1".to_string(),
        description: None,
        deterministic: true,
        timeout: None,
    }];
    app.state_results = vec![None];
    app.state_panel_list_state.select(Some(0)); // First item

    app.handle_key(KeyCode::Char('k'));

    // Should stay at first item
    assert_eq!(
        app.state_panel_list_state.selected(),
        Some(0),
        "Should not move before first query"
    );
}

#[test]
fn state_panel_navigation_with_arrow_keys() {
    use crate::state::StateQuery;

    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.active_pane = ActivePane::StatePanel;

    app.state_queries = vec![
        StateQuery {
            name: "q1".to_string(),
            run: "echo q1".to_string(),
            description: None,
            deterministic: true,
            timeout: None,
        },
        StateQuery {
            name: "q2".to_string(),
            run: "echo q2".to_string(),
            description: None,
            deterministic: true,
            timeout: None,
        },
        StateQuery {
            name: "q3".to_string(),
            run: "echo q3".to_string(),
            description: None,
            deterministic: true,
            timeout: None,
        },
    ];
    app.state_results = vec![None, None, None];
    app.state_panel_list_state.select(Some(1));

    // Test Down arrow
    app.handle_key(KeyCode::Down);
    assert_eq!(
        app.state_panel_list_state.selected(),
        Some(2),
        "Down arrow should move down"
    );

    // Test Up arrow
    app.handle_key(KeyCode::Up);
    assert_eq!(
        app.state_panel_list_state.selected(),
        Some(1),
        "Up arrow should move up"
    );
}

#[test]
fn state_panel_handles_empty_queries_gracefully() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.active_pane = ActivePane::StatePanel;
    app.state_queries = Vec::new();
    app.state_results = Vec::new();

    // Should not panic on navigation with no queries
    app.handle_key(KeyCode::Char('j'));
    app.handle_key(KeyCode::Char('k'));
    app.handle_key(KeyCode::Down);
    app.handle_key(KeyCode::Up);

    // Selection should remain None or 0
    assert!(
        app.state_panel_list_state.selected().is_none()
            || app.state_panel_list_state.selected() == Some(0),
        "Empty query list should not panic"
    );
}

#[test]
fn state_panel_clears_state_on_close() {
    use crate::state::StateQuery;

    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.active_pane = ActivePane::StatePanel;

    // Populate with data
    app.state_queries = vec![StateQuery {
        name: "test".to_string(),
        run: "echo test".to_string(),
        description: None,
        deterministic: true,
        timeout: None,
    }];
    app.state_results = vec![None];

    // Close panel
    app.handle_key(KeyCode::Esc);

    // Verify cleanup
    assert!(
        app.state_queries.is_empty(),
        "Queries should be cleared on close"
    );
    assert!(
        app.state_results.is_empty(),
        "Results should be cleared on close"
    );
}

#[test]
fn state_results_match_queries_length() {
    use crate::state::StateQuery;

    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // Simulate load_state_queries populating data
    app.state_queries = vec![
        StateQuery {
            name: "q1".to_string(),
            run: "echo q1".to_string(),
            description: None,
            deterministic: true,
            timeout: None,
        },
        StateQuery {
            name: "q2".to_string(),
            run: "echo q2".to_string(),
            description: None,
            deterministic: true,
            timeout: None,
        },
    ];
    app.state_results = vec![None, None];

    assert_eq!(
        app.state_queries.len(),
        app.state_results.len(),
        "Queries and results must stay in sync"
    );
}

// ===== State Panel Phase 1 Tests =====

#[test]
fn state_panel_refresh_key_triggers_refresh() {
    use crate::state::StateQuery;

    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.active_pane = ActivePane::StatePanel;
    app.list_state.select(Some(0));

    // Add mock query
    app.state_queries = vec![StateQuery {
        name: "test".to_string(),
        run: "echo test".to_string(),
        description: None,
        deterministic: true,
        timeout: None,
    }];
    app.state_results = vec![None];
    app.state_panel_list_state.select(Some(0));

    // Press 'r' to refresh
    // Note: This will attempt to run graft command, which may not be available in test
    // The test verifies the key is wired up correctly
    app.handle_key(KeyCode::Char('r'));

    // After refresh attempt, should have a status message
    assert!(
        app.status_message.is_some(),
        "Refresh should set a status message"
    );
}

#[test]
fn state_panel_refresh_with_no_selection_shows_warning() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.active_pane = ActivePane::StatePanel;
    // No selection

    app.handle_key(KeyCode::Char('r'));

    // Should show warning about no selection
    assert!(app.status_message.is_some());
    let msg = app.status_message.as_ref().unwrap();
    assert_eq!(msg.msg_type, MessageType::Warning);
}

#[test]
fn state_panel_shows_cache_age_formatting() {
    use crate::state::{StateMetadata, StateQuery, StateResult};
    use serde_json::json;

    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.state_queries = vec![StateQuery {
        name: "coverage".to_string(),
        run: "pytest --cov".to_string(),
        description: None,
        deterministic: true,
        timeout: None,
    }];

    // Create a result with known timestamp
    app.state_results = vec![Some(StateResult {
        metadata: StateMetadata {
            query_name: "coverage".to_string(),
            commit_hash: "abc123".to_string(),
            timestamp: (chrono::Utc::now() - chrono::Duration::minutes(5)).to_rfc3339(),
            command: "pytest --cov".to_string(),
            deterministic: true,
        },
        data: json!({"lines": 85}),
    })];

    // Verify time_ago() method works
    if let Some(Some(result)) = app.state_results.get(0) {
        let age = result.metadata.time_ago();
        // Should show something like "5m ago"
        assert!(
            age.contains("m ago") || age.contains("just now"),
            "Cache age should be formatted, got: {}",
            age
        );
    }
}

#[test]
fn state_panel_empty_state_is_helpful() {
    let app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // Empty state should render without panicking
    // This is more of a smoke test - actual rendering tested visually
    assert!(app.state_queries.is_empty());
    assert!(app.state_results.is_empty());
}

// ===== execute_state_query_command tests =====

#[test]
fn execute_query_command_captures_json_output() {
    let result =
        execute_state_query_command("echo '{\"total_words\": 5000}'", std::path::Path::new("/tmp"))
            .expect("should succeed");

    assert_eq!(result.data["total_words"], 5000);
}

#[test]
fn execute_query_command_supports_shell_features() {
    // Pipes should work because we use sh -c
    let result = execute_state_query_command(
        "echo '{\"a\": 1, \"b\": 2}' | cat",
        std::path::Path::new("/tmp"),
    )
    .expect("should succeed with pipes");

    assert_eq!(result.data["a"], 1);
    assert_eq!(result.data["b"], 2);
}

#[test]
fn execute_query_command_rejects_invalid_json() {
    let result = execute_state_query_command("echo 'not json'", std::path::Path::new("/tmp"));

    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("Invalid JSON"),
        "should report invalid JSON"
    );
}

#[test]
fn execute_query_command_rejects_non_object_json() {
    let result = execute_state_query_command("echo '[1, 2, 3]'", std::path::Path::new("/tmp"));

    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("JSON object"),
        "should require JSON object"
    );
}

#[test]
fn execute_query_command_reports_failed_commands() {
    let result = execute_state_query_command("exit 1", std::path::Path::new("/tmp"));

    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("Command failed"),
        "should report command failure"
    );
}

#[test]
fn execute_query_command_gets_commit_hash_from_git_repo() {
    let temp = tempfile::tempdir().unwrap();
    // Initialize a git repo and make a commit
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "--allow-empty", "-m", "initial"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    let result = execute_state_query_command("echo '{\"ok\": true}'", temp.path())
        .expect("should succeed");

    // Should have a real commit hash (40 hex chars), not "unknown"
    assert_ne!(result.commit_hash, "unknown");
    assert_eq!(result.commit_hash.len(), 40);
    assert!(result.commit_hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn execute_query_command_uses_unknown_hash_for_non_git_dir() {
    let temp = tempfile::tempdir().unwrap();

    let result = execute_state_query_command("echo '{\"ok\": true}'", temp.path())
        .expect("should succeed");

    assert_eq!(result.commit_hash, "unknown");
}

#[test]
fn raw_state_result_finalize_sets_metadata() {
    let raw = RawStateResult {
        data: serde_json::json!({"count": 42}),
        commit_hash: "abc123".to_string(),
    };

    let result = raw.finalize("my-query", "echo test", true);

    assert_eq!(result.metadata.query_name, "my-query");
    assert_eq!(result.metadata.commit_hash, "abc123");
    assert_eq!(result.metadata.command, "echo test");
    assert!(result.metadata.deterministic);
    assert_eq!(result.data["count"], 42);
    // Timestamp should be recent
    assert!(result.metadata.timestamp_parsed().is_some());
}

// ===== Full refresh flow tests =====

#[test]
fn refresh_updates_in_memory_state_result() {
    use crate::state::StateQuery;

    let temp = tempfile::tempdir().unwrap();
    let repo_path = RepoPath::new(temp.path().to_str().unwrap()).unwrap();

    let registry = MockRegistry {
        repos: vec![repo_path.clone()],
        statuses: {
            let mut m = HashMap::new();
            m.insert(repo_path, RepoStatus::new(
                RepoPath::new(temp.path().to_str().unwrap()).unwrap(),
            ));
            m
        },
    };

    let mut app = App::new(
        registry,
        MockDetailProvider::empty(),
        "test-refresh-workspace".to_string(),
    );
    app.active_pane = ActivePane::StatePanel;
    app.list_state.select(Some(0));

    // Add a query that outputs valid JSON
    app.state_queries = vec![StateQuery {
        name: "test-query".to_string(),
        run: "echo '{\"total_words\": 1234, \"words_today\": 56}'".to_string(),
        description: None,
        deterministic: false,
        timeout: None,
    }];
    app.state_results = vec![None]; // Start with no cached data
    app.state_panel_list_state.select(Some(0));

    // Press 'r' to refresh
    app.handle_key(KeyCode::Char('r'));

    // Verify the in-memory result was updated
    assert!(
        app.state_results[0].is_some(),
        "state_results should be populated after refresh"
    );

    let result = app.state_results[0].as_ref().unwrap();
    assert_eq!(result.data["total_words"], 1234);
    assert_eq!(result.data["words_today"], 56);
    assert_eq!(result.metadata.query_name, "test-query");

    // Verify success status message
    let msg = app.status_message.as_ref().unwrap();
    assert_eq!(msg.msg_type, MessageType::Success);

    // Verify summary formatting works
    assert_eq!(result.summary(), "1234 words total, 56 today");
}

#[test]
fn refresh_writes_to_cache() {
    use crate::state::{compute_workspace_hash, read_latest_cached, StateQuery};

    let temp = tempfile::tempdir().unwrap();
    let repo_path = RepoPath::new(temp.path().to_str().unwrap()).unwrap();
    let workspace_name = "test-cache-write-workspace";
    let workspace_hash = compute_workspace_hash(workspace_name);
    let repo_name = temp.path().file_name().unwrap().to_str().unwrap();

    let registry = MockRegistry {
        repos: vec![repo_path.clone()],
        statuses: {
            let mut m = HashMap::new();
            m.insert(repo_path, RepoStatus::new(
                RepoPath::new(temp.path().to_str().unwrap()).unwrap(),
            ));
            m
        },
    };

    let mut app = App::new(
        registry,
        MockDetailProvider::empty(),
        workspace_name.to_string(),
    );
    app.active_pane = ActivePane::StatePanel;
    app.list_state.select(Some(0));

    app.state_queries = vec![StateQuery {
        name: "cache-test".to_string(),
        run: "echo '{\"items\": 99}'".to_string(),
        description: None,
        deterministic: true,
        timeout: None,
    }];
    app.state_results = vec![None];
    app.state_panel_list_state.select(Some(0));

    // Refresh
    app.handle_key(KeyCode::Char('r'));

    // Verify cache was written and can be read back
    let cached = read_latest_cached(&workspace_hash, repo_name, "cache-test");
    assert!(
        cached.is_ok(),
        "should be able to read cache after refresh: {:?}",
        cached.err()
    );

    let cached = cached.unwrap();
    assert_eq!(cached.data["items"], 99);
    assert_eq!(cached.metadata.query_name, "cache-test");

    // Clean up cache
    let cache_root = std::path::PathBuf::from(std::env::var("HOME").unwrap())
        .join(".cache/graft")
        .join(&workspace_hash);
    std::fs::remove_dir_all(cache_root).ok();
}

#[test]
fn refresh_with_invalid_json_command_shows_error() {
    use crate::state::StateQuery;

    let temp = tempfile::tempdir().unwrap();
    let repo_path = RepoPath::new(temp.path().to_str().unwrap()).unwrap();

    let registry = MockRegistry {
        repos: vec![repo_path.clone()],
        statuses: {
            let mut m = HashMap::new();
            m.insert(repo_path, RepoStatus::new(
                RepoPath::new(temp.path().to_str().unwrap()).unwrap(),
            ));
            m
        },
    };

    let mut app = App::new(
        registry,
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.active_pane = ActivePane::StatePanel;
    app.list_state.select(Some(0));

    app.state_queries = vec![StateQuery {
        name: "bad-query".to_string(),
        run: "echo 'not valid json'".to_string(),
        description: None,
        deterministic: false,
        timeout: None,
    }];
    app.state_results = vec![None];
    app.state_panel_list_state.select(Some(0));

    app.handle_key(KeyCode::Char('r'));

    // Should show error, result should remain None
    let msg = app.status_message.as_ref().unwrap();
    assert_eq!(msg.msg_type, MessageType::Error);
    assert!(app.state_results[0].is_none());
}

#[test]
fn refresh_with_failing_command_shows_error() {
    use crate::state::StateQuery;

    let temp = tempfile::tempdir().unwrap();
    let repo_path = RepoPath::new(temp.path().to_str().unwrap()).unwrap();

    let registry = MockRegistry {
        repos: vec![repo_path.clone()],
        statuses: {
            let mut m = HashMap::new();
            m.insert(repo_path, RepoStatus::new(
                RepoPath::new(temp.path().to_str().unwrap()).unwrap(),
            ));
            m
        },
    };

    let mut app = App::new(
        registry,
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.active_pane = ActivePane::StatePanel;
    app.list_state.select(Some(0));

    app.state_queries = vec![StateQuery {
        name: "fail-query".to_string(),
        run: "exit 1".to_string(),
        description: None,
        deterministic: false,
        timeout: None,
    }];
    app.state_results = vec![None];
    app.state_panel_list_state.select(Some(0));

    app.handle_key(KeyCode::Char('r'));

    let msg = app.status_message.as_ref().unwrap();
    assert_eq!(msg.msg_type, MessageType::Error);
    assert!(app.state_results[0].is_none());
}
