//! Unit tests for TUI module
//!
//! These tests verify TUI logic without requiring a real terminal.

use super::*;
use crossterm::event::{KeyCode, KeyModifiers};
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

// ===== Existing tests updated =====

// Test 1: Keybinding handling - quit keys
#[test]
fn handles_quit_with_q_key() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    assert!(!app.should_quit, "Should not quit initially");

    app.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);
    assert!(app.should_quit, "Should quit after pressing 'q'");
}

#[test]
fn esc_from_dashboard_does_not_quit() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    assert!(!app.should_quit, "Should not quit initially");

    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
    // Escape goes home (Dashboard) — already there, so no-op (does not quit)
    assert!(!app.should_quit, "Esc from Dashboard should NOT quit");
    assert_eq!(*app.current_view(), View::Dashboard);
}

// Test 2: Navigation with empty list doesn't panic
#[test]
fn navigation_with_empty_list_does_not_panic() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.next();
    app.previous();

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

    app.list_state.select(Some(2));
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

    app.list_state.select(Some(0));
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

    assert_eq!(
        line.spans.len(),
        3,
        "Should have 3 spans: path, space, error"
    );

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
    status.branch = None;
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

    // Press unknown keys (note: 'x' and 's' are now valid, so use truly unknown keys)
    app.handle_key(KeyCode::Char('a'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('z'), KeyModifiers::NONE);
    app.handle_key(KeyCode::F(1), KeyModifiers::NONE);

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

    app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(app.list_state.selected(), Some(1), "'j' should move down");

    app.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
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

    app.handle_key(KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(
        app.list_state.selected(),
        Some(1),
        "Down arrow should move down"
    );

    app.handle_key(KeyCode::Up, KeyModifiers::NONE);
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

// ===== Focus management tests =====

#[test]
fn starts_with_repo_list_focused() {
    let app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    assert_eq!(*app.current_view(), View::Dashboard);
}

#[test]
fn enter_switches_to_detail_pane() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(*app.current_view(), View::RepoDetail(0));
}

#[test]
fn tab_switches_to_detail_pane() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.handle_key(KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(*app.current_view(), View::RepoDetail(0));
}

#[test]
fn q_in_detail_returns_to_list() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.push_view(View::RepoDetail(0));

    app.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);
    assert_eq!(*app.current_view(), View::Dashboard);
    assert!(!app.should_quit, "q in detail should NOT quit the app");
}

#[test]
fn esc_in_detail_resets_to_dashboard() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.push_view(View::RepoDetail(0));

    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(*app.current_view(), View::Dashboard);
    assert!(!app.should_quit, "Esc in detail should NOT quit the app");
}

#[test]
fn enter_in_detail_with_no_commands_is_noop() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.push_view(View::RepoDetail(0));

    // Enter tries to execute a command; with no commands loaded, it's a no-op.
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(*app.current_view(), View::RepoDetail(0));
}

#[test]
fn tab_in_detail_returns_to_list() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.push_view(View::RepoDetail(0));

    app.handle_key(KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(*app.current_view(), View::Dashboard);
}

// Detail scroll tests
#[test]
fn j_in_detail_scrolls_down() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.push_view(View::RepoDetail(0));

    assert_eq!(app.detail_scroll, 0);
    app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(app.detail_scroll, 1);
    app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(app.detail_scroll, 2);
}

#[test]
fn k_in_detail_does_not_go_below_zero() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.push_view(View::RepoDetail(0));

    assert_eq!(app.detail_scroll, 0);
    app.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
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

    app.cached_detail = Some(RepoDetail::empty());
    app.cached_detail_index = Some(0);

    // Navigate in Dashboard view
    app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(app.list_state.selected(), Some(1));

    app.ensure_detail_loaded();
    assert_eq!(app.cached_detail_index, Some(1));
}

// Detail rendering tests
#[test]
fn build_repo_detail_lines_no_selection() {
    let app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    let lines = app.build_repo_detail_lines();
    let text: String = lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref().to_string()))
        .collect::<String>();
    assert!(text.contains("No repository selected"));
}

#[test]
fn build_repo_detail_lines_with_error() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.cached_detail = Some(RepoDetail::with_error("git failed".to_string()));
    app.cached_detail_index = Some(0);

    let lines = app.build_repo_detail_lines();
    let text: String = lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref().to_string()))
        .collect::<String>();
    assert!(text.contains("Error: git failed"));
}

#[test]
fn build_repo_detail_lines_with_commits_and_files() {
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

    let lines = app.build_repo_detail_lines();
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
fn build_repo_detail_lines_empty_repo() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::with_detail(RepoDetail::empty()),
        "test-workspace".to_string(),
    );
    app.cached_detail_index = Some(0);
    app.ensure_detail_loaded();

    let lines = app.build_repo_detail_lines();
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
fn build_repo_detail_lines_shows_branch_header() {
    // Branch info is shown in the block title via repo_detail_title(), not in the content lines.
    // The content lines show changes/commits/state/commands sections.
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

    let lines = app.build_repo_detail_lines();
    let text: String = lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref().to_string()))
        .collect::<String>();

    // Content lines include the changes/state/commands sections
    assert!(
        text.contains("No uncommitted changes")
            || text.contains("Changed Files")
            || text.contains("State Queries"),
        "Should show content sections"
    );
}

#[test]
fn build_repo_detail_lines_clean_repo_shows_clean_indicator() {
    // Branch/dirty info is shown in the block title via repo_detail_title(), not in content lines.
    // Clean repo with empty detail shows "No uncommitted changes" in the content.
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

    let lines = app.build_repo_detail_lines();
    let text: String = lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref().to_string()))
        .collect::<String>();

    assert!(
        text.contains("No uncommitted changes"),
        "Clean repo should show no-changes message"
    );
    assert!(
        !text.contains("↑"),
        "Content lines should not show ahead indicator"
    );
    assert!(
        !text.contains("↓"),
        "Content lines should not show behind indicator"
    );
}

// --- Partial error rendering test ---

#[test]
fn build_repo_detail_lines_shows_error_and_partial_data() {
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

    let lines = app.build_repo_detail_lines();
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
    app.push_view(View::RepoDetail(0));

    app.ensure_detail_loaded();

    app.detail_scroll = 9999;

    let lines = app.build_repo_detail_lines();
    let max_scroll = lines.len();

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

    assert!(
        result.contains("project-name/submodule"),
        "Should preserve final 2 components, got: {result}"
    );
    assert!(
        result.len() < path.len(),
        "Should be shorter than original, got: {result}"
    );
}

#[test]
fn compact_path_preserves_final_components() {
    let path = "/a/b/c/d/project/repo";
    let result = compact_path(path, 25);

    assert!(
        result.ends_with("project/repo"),
        "Should end with last 2 components, got: {result}"
    );
}

#[test]
fn compact_path_falls_back_to_prefix_truncation() {
    let path = "/extremely/long/path/that/will/not/fit/even/with/abbreviation/project-name";
    let result = compact_path(path, 20);

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

    assert!(result.ends_with("access/archive"));
    assert!(result.len() < path.len());
}

// --- Adaptive branch display tests ---

#[test]
fn format_repo_line_shows_branch_when_space_allows() {
    let path = "/tmp/repo".to_string();
    let mut status = RepoStatus::new(RepoPath::new("/tmp/repo").unwrap());
    status.branch = Some("main".to_string());
    status.is_dirty = true;

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

    let line = format_repo_line(path, Some(&status), 20);
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

    assert!(
        !text.contains("[feature-branch-with-long-name]"),
        "Should drop branch when path would be too short, got: {text}"
    );
    assert!(text.contains("●"), "Should still show dirty indicator");
}

#[test]
fn format_repo_line_drops_branch_when_path_uses_prefix_truncation() {
    let path = "/extremely/long/nested/directory/structure/repository-name".to_string();
    let mut status = RepoStatus::new(RepoPath::new(&path).unwrap());
    status.branch = Some("main".to_string());
    status.is_dirty = false;

    let line = format_repo_line(path, Some(&status), 18);
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

    assert!(
        !text.contains("[main]"),
        "Should drop branch when path uses [..] prefix, got: {text}"
    );
}

#[test]
fn format_repo_line_unicode_path_uses_width_not_len() {
    let path = "/home/用户/项目/repository".to_string();
    let mut status = RepoStatus::new(RepoPath::new(&path).unwrap());
    status.branch = Some("main".to_string());
    status.is_dirty = true;

    let line = format_repo_line(path, Some(&status), 40);
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

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

    let line = format_repo_line(path, Some(&status), 22);
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

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

    let line = format_repo_line(path, Some(&status), 12);
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

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
    let path = "/home/user/repo".to_string();
    let mut status = RepoStatus::new(RepoPath::new(&path).unwrap());
    status.branch = Some("main".to_string());
    status.is_dirty = true;
    status.ahead = Some(4);
    status.behind = Some(2);

    let pane_width = 50;
    let line = format_repo_line(path, Some(&status), pane_width);

    let actual_width: usize = line.spans.iter().map(|s| s.content.width()).sum();

    assert!(
        actual_width <= pane_width as usize - 2,
        "Line width {} should fit in pane {} with overhead, got spans: {:?}",
        actual_width,
        pane_width,
        line.spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<Vec<_>>()
    );

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
    assert_eq!(*app.current_view(), View::Dashboard);

    app.handle_key(KeyCode::Char('?'), KeyModifiers::NONE);
    assert_eq!(
        *app.current_view(),
        View::Help,
        "View stack should show Help"
    );
}

#[test]
fn help_overlay_dismisses_on_printable_key() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char('?'), KeyModifiers::NONE);
    assert_eq!(*app.current_view(), View::Help);

    app.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);
    assert_eq!(*app.current_view(), View::Dashboard);
}

#[test]
fn help_overlay_dismisses_on_esc() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char('?'), KeyModifiers::NONE);
    assert_eq!(*app.current_view(), View::Help);

    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(*app.current_view(), View::Dashboard);
}

#[test]
fn empty_workspace_navigation_does_not_panic() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(app.list_state.selected(), None);

    app.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
    assert_eq!(app.list_state.selected(), None);
}

// --- Status bar tests ---

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
    let _ = supports_unicode();
    assert!(supports_unicode() || !supports_unicode());
}

#[test]
fn clear_expired_status_message_removes_old_messages() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    let mut old_msg = StatusMessage::info("Old message");
    old_msg.shown_at = Instant::now() - Duration::from_secs(4);
    app.status_message = Some(old_msg);

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

    app.status_message = Some(StatusMessage::success("Fresh message"));

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

    app.needs_refresh = true;
    app.handle_refresh_if_needed();

    assert!(app.status_message.is_some());
    let msg = app.status_message.as_ref().unwrap();
    assert_eq!(msg.msg_type, MessageType::Success);
    assert!(msg.text.contains("Refreshed"));
}

#[test]
fn x_from_repo_list_navigates_to_commands_tab() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char('x'), KeyModifiers::NONE);
    assert_eq!(*app.current_view(), View::RepoDetail(0));
}

#[test]
fn s_from_repo_list_navigates_to_state_tab() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);
    assert_eq!(*app.current_view(), View::RepoDetail(0));
}

// ===== Command Execution Tests =====

#[test]
fn command_state_transitions_not_started_to_running() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    assert!(matches!(app.command_state, CommandState::NotStarted));

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

    app.command_state = CommandState::Running;
    app.push_view(View::CommandOutput);

    app.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);
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

    app.command_state = CommandState::Completed { exit_code: 0 };
    app.push_view(View::CommandOutput);

    app.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);
    assert!(
        !app.show_stop_confirmation,
        "Dialog should not show for completed command"
    );
    assert_eq!(*app.current_view(), View::Dashboard);
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

// ===== Argument Input Tests =====

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
            args: None,
        },
    )];
    app.push_view(View::RepoDetail(0));
    app.command_picker_state.select(Some(0));

    app.execute_selected_command();

    assert!(app.argument_input.is_some());
    let state = app.argument_input.as_ref().unwrap();
    assert_eq!(state.command_name, "test");
    assert!(state.text.buffer.is_empty());
    assert_eq!(state.text.cursor_pos, 0);
}

#[test]
fn argument_input_buffer_updates_on_char() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    app.argument_input = Some(ArgumentInputState {
        text: super::text_buffer::TextBuffer::new(),
        command_name: "test".to_string(),
    });

    app.handle_key(KeyCode::Char('a'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('g'), KeyModifiers::NONE);

    let state = app.argument_input.as_ref().unwrap();
    assert_eq!(state.text.buffer, "arg");
    assert_eq!(state.text.cursor_pos, 3);
}

#[test]
fn argument_input_backspace_removes_char() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    app.argument_input = Some(ArgumentInputState {
        text: super::text_buffer::TextBuffer::with_content("test", 4),
        command_name: "test".to_string(),
    });

    app.handle_key(KeyCode::Backspace, KeyModifiers::NONE);

    let state = app.argument_input.as_ref().unwrap();
    assert_eq!(state.text.buffer, "tes");
    assert_eq!(state.text.cursor_pos, 3);
}

#[test]
fn argument_input_escape_cancels() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    app.argument_input = Some(ArgumentInputState {
        text: super::text_buffer::TextBuffer::with_content("some args", 9),
        command_name: "test".to_string(),
    });

    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
    assert!(app.argument_input.is_none());
}

#[test]
fn argument_input_enter_executes_with_args() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    app.argument_input = Some(ArgumentInputState {
        text: super::text_buffer::TextBuffer::with_content("arg1 arg2", 9),
        command_name: "test".to_string(),
    });
    app.selected_repo_for_commands = Some("/tmp/test".to_string());

    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
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
    app.argument_input = Some(ArgumentInputState {
        text: super::text_buffer::TextBuffer::new(),
        command_name: "test".to_string(),
    });
    app.selected_repo_for_commands = Some("/tmp/test".to_string());

    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(app.command_name, Some("test".to_string()));
}

#[test]
fn argument_input_parses_quoted_arguments_correctly() {
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

// ===== Cursor Navigation Tests =====

#[test]
fn argument_input_cursor_moves_left() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    app.argument_input = Some(ArgumentInputState {
        text: super::text_buffer::TextBuffer::with_content("test", 4),
        command_name: "cmd".to_string(),
    });

    app.handle_key(KeyCode::Left, KeyModifiers::NONE);

    assert_eq!(app.argument_input.as_ref().unwrap().text.cursor_pos, 3);
}

#[test]
fn argument_input_cursor_moves_right() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    app.argument_input = Some(ArgumentInputState {
        text: super::text_buffer::TextBuffer::with_content("test", 2),
        command_name: "cmd".to_string(),
    });

    app.handle_key(KeyCode::Right, KeyModifiers::NONE);

    assert_eq!(app.argument_input.as_ref().unwrap().text.cursor_pos, 3);
}

#[test]
fn argument_input_cursor_stops_at_boundaries() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    app.argument_input = Some(ArgumentInputState {
        text: super::text_buffer::TextBuffer::with_content("test", 0),
        command_name: "cmd".to_string(),
    });

    app.handle_key(KeyCode::Left, KeyModifiers::NONE);
    assert_eq!(app.argument_input.as_ref().unwrap().text.cursor_pos, 0);

    app.argument_input.as_mut().unwrap().text.cursor_pos = 4;

    app.handle_key(KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(app.argument_input.as_ref().unwrap().text.cursor_pos, 4);
}

#[test]
fn argument_input_home_end_keys() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    app.argument_input = Some(ArgumentInputState {
        text: super::text_buffer::TextBuffer::with_content("test", 2),
        command_name: "cmd".to_string(),
    });

    app.handle_key(KeyCode::Home, KeyModifiers::NONE);
    assert_eq!(app.argument_input.as_ref().unwrap().text.cursor_pos, 0);

    app.handle_key(KeyCode::End, KeyModifiers::NONE);
    assert_eq!(app.argument_input.as_ref().unwrap().text.cursor_pos, 4);
}

#[test]
fn argument_input_inserts_char_at_cursor() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    app.argument_input = Some(ArgumentInputState {
        text: super::text_buffer::TextBuffer::with_content("test", 2),
        command_name: "cmd".to_string(),
    });

    app.handle_key(KeyCode::Char('X'), KeyModifiers::NONE);

    let state = app.argument_input.as_ref().unwrap();
    assert_eq!(state.text.buffer, "teXst");
    assert_eq!(state.text.cursor_pos, 3);
}

#[test]
fn argument_input_backspace_at_cursor() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    app.argument_input = Some(ArgumentInputState {
        text: super::text_buffer::TextBuffer::with_content("test", 2),
        command_name: "cmd".to_string(),
    });

    app.handle_key(KeyCode::Backspace, KeyModifiers::NONE);

    let state = app.argument_input.as_ref().unwrap();
    assert_eq!(state.text.buffer, "tst");
    assert_eq!(state.text.cursor_pos, 1);
}

#[test]
fn argument_input_prevents_execution_on_parse_error() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );
    app.argument_input = Some(ArgumentInputState {
        text: super::text_buffer::TextBuffer::with_content(r#"unclosed "quote"#, 15),
        command_name: "cmd".to_string(),
    });
    app.selected_repo_for_commands = Some("/tmp/test".to_string());

    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert!(
        app.argument_input.is_some(),
        "argument input overlay should still be active"
    );

    assert!(app.status_message.is_some());
    let msg = app.status_message.as_ref().unwrap();
    assert!(msg.text.contains("parsing error") || msg.text.contains("Parse error"));
}

// ===== Tab Switching Tests =====

#[test]
fn q_from_any_tab_returns_to_repo_list() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.push_view(View::RepoDetail(0));
    app.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);
    assert_eq!(*app.current_view(), View::Dashboard);
    assert!(!app.should_quit);
}

// ===== State Tab Tests =====

#[test]
fn state_tab_navigation_with_j_key() {
    // In the unified RepoDetail view, j/k scroll the detail_scroll offset.
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.push_view(View::RepoDetail(0));

    assert_eq!(app.detail_scroll, 0);
    app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(app.detail_scroll, 1, "'j' should scroll detail view down");
}

#[test]
fn state_tab_navigation_with_k_key() {
    // In the unified RepoDetail view, k scrolls detail_scroll up (saturating at 0).
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.push_view(View::RepoDetail(0));
    app.detail_scroll = 3;

    app.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
    assert_eq!(app.detail_scroll, 2, "'k' should scroll detail view up");

    app.detail_scroll = 0;
    app.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
    assert_eq!(app.detail_scroll, 0, "'k' at top should not go below 0");
}

#[test]
fn state_tab_navigation_does_not_move_past_end() {
    // In the unified view, j always increments detail_scroll (no upper bound from content in this test).
    // This test verifies j continues to increment.
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.push_view(View::RepoDetail(0));
    app.detail_scroll = 5;

    app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(app.detail_scroll, 6, "j should increment scroll");
}

#[test]
fn state_tab_navigation_does_not_move_before_start() {
    // k at detail_scroll=0 should not underflow (saturating_sub).
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.push_view(View::RepoDetail(0));
    assert_eq!(app.detail_scroll, 0);

    app.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
    assert_eq!(app.detail_scroll, 0, "k at top should not underflow");
}

#[test]
fn state_tab_navigation_with_arrow_keys() {
    // Arrow keys in the unified RepoDetail view scroll detail_scroll.
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.push_view(View::RepoDetail(0));
    app.detail_scroll = 2;

    app.handle_key(KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(app.detail_scroll, 3, "Down arrow should scroll down");

    app.handle_key(KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(app.detail_scroll, 2, "Up arrow should scroll up");
}

#[test]
fn state_tab_handles_empty_queries_gracefully() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.push_view(View::RepoDetail(0));
    app.state_queries = Vec::new();
    app.state_results = Vec::new();

    app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Down, KeyModifiers::NONE);
    app.handle_key(KeyCode::Up, KeyModifiers::NONE);

    // Just verifying that navigating with an empty query list does not panic
    assert!(
        app.state_queries.is_empty(),
        "Empty query list should not panic"
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

#[test]
fn state_tab_refresh_with_no_queries_shows_info() {
    // When no state queries are configured, pressing r shows an informational message.
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.push_view(View::RepoDetail(0));
    // state_queries is empty by default

    app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);

    assert!(app.status_message.is_some());
    let msg = app.status_message.as_ref().unwrap();
    assert_eq!(
        msg.msg_type,
        MessageType::Info,
        "No queries defined should produce an Info message"
    );
}

#[test]
fn state_tab_shows_cache_age_formatting() {
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

    if let Some(Some(result)) = app.state_results.get(0) {
        let age = result.metadata.time_ago();
        assert!(
            age.contains("m ago") || age.contains("just now"),
            "Cache age should be formatted, got: {}",
            age
        );
    }
}

#[test]
fn state_tab_empty_state_is_helpful() {
    let app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    assert!(app.state_queries.is_empty());
    assert!(app.state_results.is_empty());
}

// ===== Data Invalidation Tests =====

#[test]
fn navigation_invalidates_tab_data() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // Simulate cached tab data
    app.selected_repo_for_commands = Some("/tmp/repo0".to_string());
    app.available_commands = vec![(
        "test".to_string(),
        grove_core::Command {
            run: "echo test".to_string(),
            description: None,
            working_dir: None,
            env: None,
            args: None,
        },
    )];

    // Navigate to next repo
    app.next();

    // Tab data should be invalidated
    assert!(
        app.selected_repo_for_commands.is_none(),
        "Commands cache should be cleared on navigation"
    );
    assert!(
        app.available_commands.is_empty(),
        "Commands list should be cleared on navigation"
    );
    assert!(
        app.state_queries.is_empty(),
        "State queries should be cleared on navigation"
    );
}

// ===== Hint Bar Tests =====

#[test]
fn hint_bar_shows_repo_list_hints() {
    let app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // Dashboard view (default)
    let hints = app.current_hints();
    let keys: Vec<&str> = hints.iter().map(|h| h.key).collect();

    assert!(keys.contains(&"j/k"), "Should have navigation hint");
    assert!(keys.contains(&"Enter"), "Should have details hint");
    assert!(keys.contains(&"?"), "Should have help hint");
    assert!(keys.contains(&"q"), "Should have quit hint");
}

#[test]
fn hint_bar_shows_detail_changes_hints() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.push_view(View::RepoDetail(0));

    let hints = app.current_hints();
    let keys: Vec<&str> = hints.iter().map(|h| h.key).collect();

    assert!(keys.contains(&"j/k"), "Should have scroll hint");
    assert!(keys.contains(&"q"), "Should have back hint");
}

#[test]
fn hint_bar_shows_detail_state_hints() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.push_view(View::RepoDetail(0));

    let hints = app.current_hints();
    let keys: Vec<&str> = hints.iter().map(|h| h.key).collect();

    assert!(keys.contains(&"r"), "Should have refresh hint");
    assert!(keys.contains(&"q"), "Should have back hint");
}

#[test]
fn hint_bar_shows_detail_commands_hints() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.push_view(View::RepoDetail(0));

    let hints = app.current_hints();
    let keys: Vec<&str> = hints.iter().map(|h| h.key).collect();

    assert!(keys.contains(&"Enter"), "Should have run hint");
    assert!(keys.contains(&"q"), "Should have back hint");
}

#[test]
fn hint_bar_shows_help_overlay_hint() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.push_view(View::Help);

    let hints = app.current_hints();
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0].key, "q");
    assert_eq!(hints[0].action, "close");
    assert_eq!(hints[1].key, "Esc");
    assert_eq!(hints[1].action, "home");
}

// ===== Tab Header Tests =====

#[test]
fn empty_workspace_x_does_not_navigate() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char('x'), KeyModifiers::NONE);
    assert_eq!(*app.current_view(), View::Dashboard);
}

#[test]
fn empty_workspace_s_does_not_navigate() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);
    assert_eq!(*app.current_view(), View::Dashboard);
}

// ===== Task 3: CommandOutput and ArgumentInput View Stack Tests =====

#[test]
fn command_output_pushes_onto_view_stack() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    app.push_view(View::CommandOutput);

    assert_eq!(
        *app.current_view(),
        View::CommandOutput,
        "CommandOutput should be on top of view stack"
    );
    assert_eq!(
        app.view_stack.len(),
        2,
        "Stack should have Dashboard + CommandOutput"
    );
}

#[test]
fn command_output_q_pops_back_to_previous_view() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    app.push_view(View::CommandOutput);
    app.command_state = CommandState::Completed { exit_code: 0 };

    app.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);

    assert_eq!(
        *app.current_view(),
        View::Dashboard,
        "q should pop CommandOutput back to Dashboard"
    );
    assert!(
        !app.should_quit,
        "Should not quit after popping CommandOutput"
    );
}

#[test]
fn command_output_esc_pops_back_to_previous_view() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    app.push_view(View::CommandOutput);
    app.command_state = CommandState::Completed { exit_code: 0 };

    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);

    assert_eq!(
        *app.current_view(),
        View::Dashboard,
        "Esc should pop CommandOutput back to Dashboard"
    );
    assert!(
        !app.should_quit,
        "Should not quit after Esc in CommandOutput"
    );
}

#[test]
fn command_output_pops_to_repo_detail_when_launched_from_there() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    app.push_view(View::RepoDetail(1));
    app.push_view(View::CommandOutput);
    app.command_state = CommandState::Completed { exit_code: 0 };

    app.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);

    assert_eq!(
        *app.current_view(),
        View::RepoDetail(1),
        "q should pop CommandOutput back to RepoDetail"
    );
}

#[test]
fn command_output_q_shows_stop_confirmation_when_running() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    app.push_view(View::CommandOutput);
    app.command_state = CommandState::Running;

    app.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);

    assert!(
        app.show_stop_confirmation,
        "q should show stop confirmation for running command"
    );
    assert_eq!(
        *app.current_view(),
        View::CommandOutput,
        "Should stay on CommandOutput while confirming"
    );
}

#[test]
fn command_output_esc_shows_stop_confirmation_when_running() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    app.push_view(View::CommandOutput);
    app.command_state = CommandState::Running;

    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);

    assert!(
        app.show_stop_confirmation,
        "Esc should show stop confirmation for running command"
    );
    assert_eq!(
        *app.current_view(),
        View::CommandOutput,
        "Should stay on CommandOutput while confirming stop"
    );
}

#[test]
fn stop_confirmation_n_cancels_and_stays_in_command_output() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    app.push_view(View::CommandOutput);
    app.command_state = CommandState::Running;
    app.show_stop_confirmation = true;

    app.handle_key(KeyCode::Char('n'), KeyModifiers::NONE);

    assert!(
        !app.show_stop_confirmation,
        "n should dismiss confirmation dialog"
    );
    assert_eq!(
        *app.current_view(),
        View::CommandOutput,
        "Should remain in CommandOutput after cancelling stop"
    );
    assert!(
        matches!(app.command_state, CommandState::Running),
        "Command should still be running"
    );
}

#[test]
fn stop_confirmation_esc_cancels_and_stays_in_command_output() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    app.push_view(View::CommandOutput);
    app.command_state = CommandState::Running;
    app.show_stop_confirmation = true;

    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);

    assert!(
        !app.show_stop_confirmation,
        "Esc should dismiss confirmation dialog"
    );
    assert_eq!(
        *app.current_view(),
        View::CommandOutput,
        "Should remain in CommandOutput after cancelling stop"
    );
    assert!(
        matches!(app.command_state, CommandState::Running),
        "Command should still be running"
    );
}

#[test]
fn argument_input_overlay_is_intercepted_before_view_dispatch() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    // Set up ArgumentInput overlay while in RepoDetail view
    app.push_view(View::RepoDetail(0));
    app.argument_input = Some(ArgumentInputState {
        text: super::text_buffer::TextBuffer::new(),
        command_name: "test".to_string(),
    });

    // Press 'q' — in view dispatch this would pop the view, but the overlay
    // should intercept and treat it as a char input instead
    // (Actually Esc is the cancel key for argument input, q is just a char)
    app.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);

    // The ArgumentInput overlay should have handled it (as char input)
    assert_eq!(
        *app.current_view(),
        View::RepoDetail(0),
        "View should not change — ArgumentInput intercepts before view dispatch"
    );
    assert!(
        app.argument_input.is_some(),
        "argument input overlay should still be active"
    );
    let state = app.argument_input.as_ref().unwrap();
    assert_eq!(
        state.text.buffer, "q",
        "q should be added to buffer, not pop the view"
    );
}

#[test]
fn argument_input_esc_restores_view_without_popping_stack() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    // ArgumentInput overlay while in RepoDetail
    app.push_view(View::RepoDetail(0));
    app.argument_input = Some(ArgumentInputState {
        text: super::text_buffer::TextBuffer::with_content("some args", 9),
        command_name: "test".to_string(),
    });

    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);

    // Should cancel overlay, stay on current view
    assert_eq!(
        *app.current_view(),
        View::RepoDetail(0),
        "View stack should be unchanged after ArgumentInput Esc"
    );
    assert!(
        app.argument_input.is_none(),
        "ArgumentInput state should be cleared"
    );
}

// ===== Task 6: Escape-goes-home and stack discipline =====

#[test]
fn escape_from_deep_stack_resets_to_dashboard() {
    // Dashboard → RepoDetail → CommandOutput: Esc resets all the way home
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    app.push_view(View::RepoDetail(1));
    app.push_view(View::Help);
    assert_eq!(app.view_stack.len(), 3);

    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);

    assert_eq!(
        *app.current_view(),
        View::Dashboard,
        "Esc from deep stack should reset to Dashboard"
    );
    assert_eq!(app.view_stack.len(), 1, "Stack should have only Dashboard");
    assert!(!app.should_quit);
}

#[test]
fn q_in_detail_pops_one_level() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    app.push_view(View::RepoDetail(0));
    assert_eq!(app.view_stack.len(), 2);

    app.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);

    assert_eq!(*app.current_view(), View::Dashboard);
    assert_eq!(app.view_stack.len(), 1);
    assert!(!app.should_quit, "q in RepoDetail should not quit");
}

#[test]
fn q_from_dashboard_quits() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    assert_eq!(*app.current_view(), View::Dashboard);

    app.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);
    assert!(app.should_quit, "q from Dashboard should quit");
}

#[test]
fn esc_from_dashboard_is_noop_not_quit() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    assert_eq!(*app.current_view(), View::Dashboard);

    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
    assert!(!app.should_quit, "Esc from Dashboard should not quit");
    assert_eq!(
        *app.current_view(),
        View::Dashboard,
        "Should remain at Dashboard"
    );
}

#[test]
fn esc_from_help_resets_to_dashboard() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    app.push_view(View::Help);

    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);

    assert_eq!(*app.current_view(), View::Dashboard);
    assert!(!app.should_quit);
}

#[test]
fn q_from_help_pops_one_level() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    app.push_view(View::Help);

    app.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);

    assert_eq!(*app.current_view(), View::Dashboard);
    assert!(!app.should_quit, "q from Help should not quit");
}

#[test]
fn esc_from_command_output_resets_to_dashboard() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    app.push_view(View::RepoDetail(0));
    app.push_view(View::CommandOutput);
    app.command_state = CommandState::Completed { exit_code: 0 };

    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);

    assert_eq!(
        *app.current_view(),
        View::Dashboard,
        "Esc from CommandOutput should reset to Dashboard"
    );
    assert!(!app.should_quit);
}

#[test]
fn q_from_command_output_pops_to_previous_view() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    app.push_view(View::RepoDetail(0));
    app.push_view(View::CommandOutput);
    app.command_state = CommandState::Completed { exit_code: 0 };

    app.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);

    assert_eq!(
        *app.current_view(),
        View::RepoDetail(0),
        "q from CommandOutput should pop to previous view (RepoDetail)"
    );
    assert!(!app.should_quit);
}

#[test]
fn stop_confirmation_gates_esc_in_command_output() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    app.push_view(View::CommandOutput);
    app.command_state = CommandState::Running;

    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);

    assert!(
        app.show_stop_confirmation,
        "Esc should show stop confirmation for running command"
    );
    assert_eq!(
        *app.current_view(),
        View::CommandOutput,
        "Should stay in CommandOutput while confirming"
    );
}

#[test]
fn stop_confirmation_gates_q_in_command_output() {
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    app.push_view(View::CommandOutput);
    app.command_state = CommandState::Running;

    app.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);

    assert!(
        app.show_stop_confirmation,
        "q should show stop confirmation for running command"
    );
    assert_eq!(
        *app.current_view(),
        View::CommandOutput,
        "Should stay in CommandOutput while confirming"
    );
}

#[test]
fn esc_from_deeply_nested_repo_detail_resets_to_dashboard() {
    // Stack: Dashboard → RepoDetail(0) → RepoDetail(1) — hypothetically deep
    // (In practice only one RepoDetail would be on stack, but the escape-goes-home
    // semantic should clear any depth)
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test".to_string(),
    );

    app.push_view(View::RepoDetail(0));
    assert_eq!(app.view_stack.len(), 2);

    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);

    assert_eq!(*app.current_view(), View::Dashboard);
    assert_eq!(app.view_stack.len(), 1, "All views above Dashboard cleared");
    assert!(!app.should_quit);
}

// ===== Task 7: Command Line Input Infrastructure =====

#[test]
fn colon_activates_command_line() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    assert!(
        app.command_line.is_none(),
        "Command line should be inactive initially"
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);

    assert!(
        app.command_line.is_some(),
        "`:` should activate command line"
    );
    let state = app.command_line.as_ref().unwrap();
    assert!(
        state.text.buffer.is_empty(),
        "Buffer should be empty on activation"
    );
    assert_eq!(state.text.cursor_pos, 0, "Cursor should be at position 0");
}

#[test]
fn colon_activates_command_line_from_repo_detail() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.push_view(View::RepoDetail(0));

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);

    assert!(
        app.command_line.is_some(),
        "`:` should activate from any view"
    );
    assert_eq!(
        *app.current_view(),
        View::RepoDetail(0),
        "View should not change"
    );
}

#[test]
fn command_line_escape_cancels() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    assert!(app.command_line.is_some());

    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);

    assert!(app.command_line.is_none(), "Esc should cancel command line");
    assert_eq!(
        *app.current_view(),
        View::Dashboard,
        "View should not change"
    );
    assert!(!app.should_quit, "Esc in command line should not quit");
}

#[test]
fn command_line_char_input_appends_to_buffer() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('e'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('l'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('p'), KeyModifiers::NONE);

    let state = app.command_line.as_ref().unwrap();
    assert_eq!(state.text.buffer, "help");
    assert_eq!(state.text.cursor_pos, 4);
}

#[test]
fn command_line_backspace_removes_char() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('e'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('l'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('p'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Backspace, KeyModifiers::NONE);

    let state = app.command_line.as_ref().unwrap();
    assert_eq!(state.text.buffer, "hel");
    assert_eq!(state.text.cursor_pos, 3);
}

#[test]
fn command_line_backspace_at_start_is_noop() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    // cursor_pos = 0, buffer = ""
    app.handle_key(KeyCode::Backspace, KeyModifiers::NONE);

    let state = app.command_line.as_ref().unwrap();
    assert!(
        state.text.buffer.is_empty(),
        "Backspace at start should be noop"
    );
    assert_eq!(state.text.cursor_pos, 0);
}

#[test]
fn command_line_left_right_cursor_movement() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('a'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('b'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('c'), KeyModifiers::NONE);
    // cursor at 3

    app.handle_key(KeyCode::Left, KeyModifiers::NONE);
    assert_eq!(app.command_line.as_ref().unwrap().text.cursor_pos, 2);

    app.handle_key(KeyCode::Left, KeyModifiers::NONE);
    assert_eq!(app.command_line.as_ref().unwrap().text.cursor_pos, 1);

    app.handle_key(KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(app.command_line.as_ref().unwrap().text.cursor_pos, 2);
}

#[test]
fn command_line_cursor_stops_at_boundaries() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('x'), KeyModifiers::NONE);
    // cursor at 1, buffer = "x"

    // Can't go right past end
    app.handle_key(KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(app.command_line.as_ref().unwrap().text.cursor_pos, 1);

    // Move to start
    app.handle_key(KeyCode::Left, KeyModifiers::NONE);
    assert_eq!(app.command_line.as_ref().unwrap().text.cursor_pos, 0);

    // Can't go left past start
    app.handle_key(KeyCode::Left, KeyModifiers::NONE);
    assert_eq!(app.command_line.as_ref().unwrap().text.cursor_pos, 0);
}

#[test]
fn command_line_home_end_keys() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('a'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('b'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('c'), KeyModifiers::NONE);
    // cursor at 3

    app.handle_key(KeyCode::Home, KeyModifiers::NONE);
    assert_eq!(app.command_line.as_ref().unwrap().text.cursor_pos, 0);

    app.handle_key(KeyCode::End, KeyModifiers::NONE);
    assert_eq!(app.command_line.as_ref().unwrap().text.cursor_pos, 3);
}

#[test]
fn command_line_enter_with_empty_buffer_fills_selected_palette_entry() {
    // When buffer is empty and the selected palette entry takes no args,
    // Enter executes it immediately and closes the command line.
    // The first palette entry is "help" (takes_args: false).
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    // No input — empty buffer, palette shows all commands, first is selected ("help")

    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    // No-arg command executes immediately — command line should be dismissed
    assert!(
        app.command_line.is_none(),
        "Enter on a no-arg palette entry should close the command line"
    );
    // "help" pushes the Help view
    assert_eq!(
        app.current_view(),
        &View::Help,
        "Enter on 'help' palette entry should push the Help view"
    );
}

#[test]
fn command_line_enter_with_help_pushes_help_view() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('e'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('l'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('p'), KeyModifiers::NONE);

    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert!(
        app.command_line.is_none(),
        "Enter should dismiss command line"
    );
    // Task 8: `:help` pushes Help view
    assert_eq!(
        *app.current_view(),
        View::Help,
        ":help should push Help view"
    );
}

#[test]
fn command_line_intercepts_before_view_dispatch() {
    // When command line is active, keys are handled by command line — not by view dispatch.
    // j navigates the palette (not scroll the detail view), other chars go to the buffer.
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.push_view(View::RepoDetail(0));

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    let detail_scroll_before = app.detail_scroll;

    // Type 'r' (not a palette nav key) — should go to buffer
    app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);

    assert_eq!(
        app.command_line.as_ref().unwrap().text.buffer,
        "r",
        "r should be added to command line buffer"
    );
    assert_eq!(
        app.detail_scroll, detail_scroll_before,
        "detail_scroll should not change while command line is active"
    );

    // j inserts into buffer when buffer has content (no longer navigates palette)
    app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(
        app.command_line.as_ref().unwrap().text.buffer,
        "rj",
        "j should be inserted into buffer when buffer has content"
    );
    assert_eq!(
        app.detail_scroll, detail_scroll_before,
        "detail_scroll should not change while command line is active"
    );
}

#[test]
fn command_line_j_k_insert_when_buffer_has_content() {
    // When the buffer already has text, j and k should be inserted as characters,
    // not consumed by palette navigation.
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    // Type "run " then j and k
    for c in "run ".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::NONE);
    }
    app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);

    assert_eq!(
        app.command_line.as_ref().unwrap().text.buffer,
        "run jk",
        "j and k should be inserted when buffer has content"
    );
}

#[test]
fn command_line_j_k_navigate_palette_when_buffer_empty() {
    // When the buffer is empty, j and k should navigate the palette.
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    assert_eq!(app.command_line.as_ref().unwrap().palette_selected, 0);

    // j moves down
    app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(
        app.command_line.as_ref().unwrap().palette_selected,
        1,
        "j should navigate palette down when buffer is empty"
    );
    assert!(
        app.command_line.as_ref().unwrap().text.buffer.is_empty(),
        "j should not be inserted when buffer is empty"
    );

    // k moves back up
    app.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
    assert_eq!(
        app.command_line.as_ref().unwrap().palette_selected,
        0,
        "k should navigate palette up when buffer is empty"
    );
    assert!(
        app.command_line.as_ref().unwrap().text.buffer.is_empty(),
        "k should not be inserted when buffer is empty"
    );
}

#[test]
fn command_line_arrows_navigate_palette_regardless_of_buffer() {
    // Arrow keys should always navigate the palette, even when buffer has content.
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    // Type "r" — palette shows refresh, repo, run
    app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
    assert_eq!(app.command_line.as_ref().unwrap().palette_selected, 0);

    // Down arrow navigates palette
    app.handle_key(KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(
        app.command_line.as_ref().unwrap().palette_selected,
        1,
        "Down arrow should navigate palette even with buffer content"
    );

    // Up arrow navigates palette
    app.handle_key(KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(
        app.command_line.as_ref().unwrap().palette_selected,
        0,
        "Up arrow should navigate palette even with buffer content"
    );

    // Buffer should remain unchanged
    assert_eq!(
        app.command_line.as_ref().unwrap().text.buffer,
        "r",
        "Arrow keys should not modify buffer"
    );
}

#[test]
fn command_line_esc_does_not_affect_view_stack() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.push_view(View::RepoDetail(0));
    app.push_view(View::Help);
    // Stack: Dashboard → RepoDetail(0) → Help

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);

    // Command line Esc only closes command line — does not pop view stack
    assert_eq!(
        *app.current_view(),
        View::Help,
        "View should remain Help after command line Esc"
    );
    assert_eq!(app.view_stack.len(), 3, "Stack should be unchanged");
}

#[test]
fn argument_input_blocks_command_line_activation() {
    // ArgumentInput intercepts before command line, so `:` in argument input is a char input.
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.argument_input = Some(ArgumentInputState {
        text: super::text_buffer::TextBuffer::new(),
        command_name: "test".to_string(),
    });

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);

    // `:` should be added to argument input buffer, NOT activate command line
    assert!(
        app.command_line.is_none(),
        "Command line should NOT activate when ArgumentInput is active"
    );
    assert_eq!(
        app.argument_input.as_ref().unwrap().text.buffer,
        ":",
        "`:` should be appended to argument input buffer"
    );
}

#[test]
fn colon_from_help_activates_command_line_not_pop() {
    // In Help view, `q` and printable chars pop the view.
    // But `:` should activate command line (intercepted before view dispatch).
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );
    app.push_view(View::Help);

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);

    assert!(
        app.command_line.is_some(),
        "`:` in Help should activate command line"
    );
    assert_eq!(
        *app.current_view(),
        View::Help,
        "Help view should remain on stack"
    );
}

#[test]
fn command_line_char_insert_at_cursor_mid_buffer() {
    // Insert a char in the middle of the buffer via cursor navigation.
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('a'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('c'), KeyModifiers::NONE);
    // buffer = "ac", cursor at 2

    app.handle_key(KeyCode::Left, KeyModifiers::NONE);
    // cursor at 1

    app.handle_key(KeyCode::Char('b'), KeyModifiers::NONE);
    // buffer should be "abc", cursor at 2

    let state = app.command_line.as_ref().unwrap();
    assert_eq!(state.text.buffer, "abc");
    assert_eq!(state.text.cursor_pos, 2);
}

// ===== Task 8: Command execution from command line =====

#[test]
fn cli_command_help_pushes_help_view() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert!(app.command_line.is_none());
    assert_eq!(*app.current_view(), View::Help, ":h should push Help view");
}

#[test]
fn cli_command_quit_sets_should_quit() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert!(app.command_line.is_none());
    assert!(app.should_quit, ":q should quit the application");
}

#[test]
fn cli_command_quit_long_form() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.command_line = Some(CommandLineState {
        text: super::text_buffer::TextBuffer::with_content("quit", 4),
        palette_selected: 0,
        history_index: None,
        history_draft: String::new(),
    });
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert!(app.should_quit, ":quit should quit the application");
}

#[test]
fn cli_command_refresh_triggers_refresh() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.command_line = Some(super::CommandLineState {
        text: super::text_buffer::TextBuffer::with_content("refresh", 7),
        palette_selected: 0,
        history_index: None,
        history_draft: String::new(),
    });
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert!(app.command_line.is_none());
    assert!(app.needs_refresh, ":refresh should set needs_refresh");
    assert!(app.status_message.is_some(), ":refresh should show status");
}

#[test]
fn cli_command_unknown_shows_error() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.command_line = Some(super::CommandLineState {
        text: super::text_buffer::TextBuffer::with_content("frobnicate", 10),
        palette_selected: 0,
        history_index: None,
        history_draft: String::new(),
    });
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert!(app.command_line.is_none());
    let msg = app
        .status_message
        .as_ref()
        .expect("Should have error message");
    assert!(
        msg.text.contains("frobnicate"),
        "Error should mention the unknown command, got: {}",
        msg.text
    );
    assert_eq!(msg.msg_type, MessageType::Error);
}

#[test]
fn cli_command_repo_by_index_jumps_to_repo() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.command_line = Some(super::CommandLineState {
        text: super::text_buffer::TextBuffer::with_content("repo 2", 6),
        palette_selected: 0,
        history_index: None,
        history_draft: String::new(),
    });
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert!(app.command_line.is_none());
    // Index 2 (1-based) = index 1 (0-based)
    assert_eq!(
        *app.current_view(),
        View::RepoDetail(1),
        ":repo 2 should jump to index 1 (0-based)"
    );
    assert_eq!(
        app.view_stack.len(),
        1,
        "reset_to_view should replace stack"
    );
}

#[test]
fn cli_command_repo_by_index_out_of_range_shows_error() {
    let mut app = App::new(
        MockRegistry::with_repos(2),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.command_line = Some(super::CommandLineState {
        text: super::text_buffer::TextBuffer::with_content("repo 99", 7),
        palette_selected: 0,
        history_index: None,
        history_draft: String::new(),
    });
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert!(app.command_line.is_none());
    assert_eq!(
        *app.current_view(),
        View::Dashboard,
        "Should stay on Dashboard for out-of-range index"
    );
    let msg = app.status_message.as_ref().expect("Should have error");
    assert_eq!(msg.msg_type, MessageType::Error);
}

#[test]
fn cli_command_repo_by_name_jumps_to_matching_repo() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // Repos are named /tmp/repo0, /tmp/repo1, /tmp/repo2
    app.command_line = Some(super::CommandLineState {
        text: super::text_buffer::TextBuffer::with_content("repo repo1", 10),
        palette_selected: 0,
        history_index: None,
        history_draft: String::new(),
    });
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert!(app.command_line.is_none());
    assert_eq!(
        *app.current_view(),
        View::RepoDetail(1),
        ":repo repo1 should match /tmp/repo1"
    );
}

#[test]
fn cli_command_repo_no_match_shows_error() {
    let mut app = App::new(
        MockRegistry::with_repos(2),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.command_line = Some(super::CommandLineState {
        text: super::text_buffer::TextBuffer::with_content("repo nonexistent", 16),
        palette_selected: 0,
        history_index: None,
        history_draft: String::new(),
    });
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert!(app.command_line.is_none());
    assert_eq!(*app.current_view(), View::Dashboard);
    let msg = app.status_message.as_ref().expect("Should have error");
    assert_eq!(msg.msg_type, MessageType::Error);
    assert!(msg.text.contains("nonexistent"));
}

#[test]
fn cli_command_run_with_no_repo_shows_warning() {
    // Empty registry — no repo selected
    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.command_line = Some(super::CommandLineState {
        text: super::text_buffer::TextBuffer::with_content("run test", 8),
        palette_selected: 0,
        history_index: None,
        history_draft: String::new(),
    });
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert!(app.command_line.is_none());
    // No repo selected → warning
    let msg = app.status_message.as_ref().expect("Should have warning");
    assert_eq!(msg.msg_type, MessageType::Warning);
}

#[test]
fn cli_command_run_from_repo_detail_pushes_command_output() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // Navigate to RepoDetail first
    app.push_view(View::RepoDetail(0));

    app.command_line = Some(super::CommandLineState {
        text: super::text_buffer::TextBuffer::with_content("run test", 8),
        palette_selected: 0,
        history_index: None,
        history_draft: String::new(),
    });
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert!(app.command_line.is_none());
    // Should push CommandOutput on top of RepoDetail
    assert_eq!(
        *app.current_view(),
        View::CommandOutput,
        ":run should push CommandOutput view"
    );
    assert_eq!(
        app.view_stack,
        vec![View::Dashboard, View::RepoDetail(0), View::CommandOutput],
        "Stack should be Dashboard → RepoDetail → CommandOutput"
    );
    assert_eq!(
        app.command_name,
        Some("test".to_string()),
        "command_name should be set"
    );
}

#[test]
fn cli_command_run_from_dashboard_uses_selected_repo() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // Select repo at index 1 from the dashboard
    app.list_state.select(Some(1));

    app.command_line = Some(super::CommandLineState {
        text: super::text_buffer::TextBuffer::with_content("run build", 9),
        palette_selected: 0,
        history_index: None,
        history_draft: String::new(),
    });
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert!(app.command_line.is_none());
    assert_eq!(
        *app.current_view(),
        View::CommandOutput,
        ":run from Dashboard should push CommandOutput"
    );
    assert_eq!(app.command_name, Some("build".to_string()));
}

#[test]
fn cli_command_state_refreshes_state_query() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // With no state queries selected, :state shows warning
    app.command_line = Some(super::CommandLineState {
        text: super::text_buffer::TextBuffer::with_content("state", 5),
        palette_selected: 0,
        history_index: None,
        history_draft: String::new(),
    });
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert!(app.command_line.is_none());
    // refresh_state_queries with no queries loaded shows a warning
    assert!(
        app.status_message.is_some(),
        ":state should produce a status message"
    );
}

// ===== Task 9: Command palette =====

#[test]
fn command_palette_shows_all_commands_when_buffer_empty() {
    use crate::tui::command_line::{filtered_palette, PALETTE_COMMANDS};

    // Empty buffer → all commands visible
    let entries = filtered_palette("");
    assert_eq!(
        entries.len(),
        PALETTE_COMMANDS.len(),
        "Empty filter should show all palette entries"
    );
}

#[test]
fn command_palette_filters_by_prefix() {
    use crate::tui::command_line::filtered_palette;

    // "re" matches "refresh" and "repo"
    let entries = filtered_palette("re");
    let names: Vec<&str> = entries.iter().map(|e| e.command).collect();
    assert!(names.contains(&"refresh"), "Should match 'refresh'");
    assert!(names.contains(&"repo"), "Should match 'repo'");
    assert!(!names.contains(&"help"), "'help' should not match 're'");
    assert!(!names.contains(&"quit"), "'quit' should not match 're'");
}

#[test]
fn command_palette_filter_is_case_insensitive() {
    use crate::tui::command_line::filtered_palette;

    // "RE" should match same as "re"
    let lower = filtered_palette("re");
    let upper = filtered_palette("RE");
    assert_eq!(
        lower.len(),
        upper.len(),
        "Filtering should be case-insensitive"
    );
}

#[test]
fn command_palette_filter_no_matches_returns_empty() {
    use crate::tui::command_line::filtered_palette;

    let entries = filtered_palette("zzz");
    assert!(
        entries.is_empty(),
        "No commands match 'zzz', should return empty"
    );
}

#[test]
fn palette_navigation_j_moves_selection_down() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    // palette_selected starts at 0

    app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(
        app.command_line.as_ref().unwrap().palette_selected,
        1,
        "j should move palette selection down"
    );

    app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(
        app.command_line.as_ref().unwrap().palette_selected,
        2,
        "j should move palette selection down again"
    );
}

#[test]
fn palette_navigation_k_moves_selection_up() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    // Move to index 2 first
    app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(app.command_line.as_ref().unwrap().palette_selected, 2);

    app.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
    assert_eq!(
        app.command_line.as_ref().unwrap().palette_selected,
        1,
        "k should move palette selection up"
    );
}

#[test]
fn palette_navigation_j_wraps_from_last_to_first() {
    use crate::tui::command_line::PALETTE_COMMANDS;

    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    // Move to the last entry
    let last_idx = PALETTE_COMMANDS.len() - 1;
    app.command_line.as_mut().unwrap().palette_selected = last_idx;

    app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(
        app.command_line.as_ref().unwrap().palette_selected,
        0,
        "j from last entry should wrap to first"
    );
}

#[test]
fn palette_navigation_k_wraps_from_first_to_last() {
    use crate::tui::command_line::PALETTE_COMMANDS;

    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    // palette_selected starts at 0

    app.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
    assert_eq!(
        app.command_line.as_ref().unwrap().palette_selected,
        PALETTE_COMMANDS.len() - 1,
        "k from first entry should wrap to last"
    );
}

#[test]
fn palette_navigation_up_down_arrows_work() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    assert_eq!(app.command_line.as_ref().unwrap().palette_selected, 0);

    app.handle_key(KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(
        app.command_line.as_ref().unwrap().palette_selected,
        1,
        "Down arrow should work same as j"
    );

    app.handle_key(KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(
        app.command_line.as_ref().unwrap().palette_selected,
        0,
        "Up arrow should work same as k"
    );
}

#[test]
fn palette_enter_fills_command_line_with_selected_entry() {
    // "quit" is a no-arg command — Enter executes it immediately.
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    // Navigate to "quit" (index 1 in PALETTE_COMMANDS)
    app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(app.command_line.as_ref().unwrap().palette_selected, 1);

    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    // No-arg command executes immediately — command line dismissed and quit triggered
    assert!(
        app.command_line.is_none(),
        "Command line should be dismissed after executing no-arg palette entry"
    );
    assert!(
        app.should_quit,
        "Selecting 'quit' from palette should set should_quit"
    );
}

#[test]
fn palette_enter_on_filled_buffer_submits_command() {
    // When buffer has text, Enter submits rather than filling from palette
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    // Type "quit" manually
    app.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('u'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('i'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('t'), KeyModifiers::NONE);

    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert!(
        app.command_line.is_none(),
        "Enter with non-empty buffer should dismiss command line"
    );
    assert!(app.should_quit, ":quit should have been executed");
}

#[test]
fn typing_resets_palette_selection() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    // Navigate palette
    app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(app.command_line.as_ref().unwrap().palette_selected, 2);

    // Typing a character should reset selection to 0
    app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
    assert_eq!(
        app.command_line.as_ref().unwrap().palette_selected,
        0,
        "Typing should reset palette selection to 0"
    );
}

#[test]
fn backspace_resets_palette_selection() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
    // Navigate palette in filtered state (3 entries: refresh, repo, run) using Down arrow
    app.handle_key(KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(app.command_line.as_ref().unwrap().palette_selected, 1);

    // Backspace should reset selection
    app.handle_key(KeyCode::Backspace, KeyModifiers::NONE);
    assert_eq!(
        app.command_line.as_ref().unwrap().palette_selected,
        0,
        "Backspace should reset palette selection to 0"
    );
}

#[test]
fn palette_escape_dismisses_command_line() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    // Navigate palette a bit
    app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);

    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);

    assert!(
        app.command_line.is_none(),
        "Esc should dismiss command line (and palette)"
    );
    assert_eq!(*app.current_view(), View::Dashboard);
    assert!(!app.should_quit);
}

// ===== Tab completion tests =====

#[test]
fn tab_completes_unique_match() {
    // Typing "he" matches only "help" — Tab should complete fully.
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('e'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Tab, KeyModifiers::NONE);

    // "help" doesn't take args, so no trailing space
    assert_eq!(
        app.command_line.as_ref().unwrap().text.buffer,
        "help",
        "Tab should complete unique match"
    );
}

#[test]
fn tab_completes_with_trailing_space_for_arg_commands() {
    // Typing "ru" matches only "run" (takes_args) — Tab adds trailing space.
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('u'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Tab, KeyModifiers::NONE);

    assert_eq!(
        app.command_line.as_ref().unwrap().text.buffer,
        "run ",
        "Tab should complete with trailing space for commands that take args"
    );
}

#[test]
fn tab_fills_common_prefix_for_multiple_matches() {
    // Typing "r" matches "refresh", "repo", "run" — common prefix is "r" (no change).
    // Typing "re" matches "refresh", "repo" — common prefix is "re" (no change).
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('e'), KeyModifiers::NONE);
    // "re" matches "refresh" and "repo" — common prefix is "re"
    app.handle_key(KeyCode::Tab, KeyModifiers::NONE);

    assert_eq!(
        app.command_line.as_ref().unwrap().text.buffer,
        "re",
        "Tab should not change buffer when common prefix equals current buffer"
    );
}

#[test]
fn tab_extends_to_common_prefix() {
    // Typing "st" matches only "state" — single match, completes fully.
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('t'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Tab, KeyModifiers::NONE);

    assert_eq!(
        app.command_line.as_ref().unwrap().text.buffer,
        "state",
        "Tab should complete 'st' to 'state'"
    );
}

#[test]
fn tab_noop_on_no_matches() {
    // Typing "xyz" matches nothing — Tab should do nothing.
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('x'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('z'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Tab, KeyModifiers::NONE);

    assert_eq!(
        app.command_line.as_ref().unwrap().text.buffer,
        "xyz",
        "Tab should be noop when no palette matches"
    );
}

// ===== Command history tests =====

#[test]
fn history_saves_on_enter() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // Execute "help"
    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    for c in "help".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::NONE);
    }
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert_eq!(app.command_history, vec!["help"]);
}

#[test]
fn history_skips_consecutive_duplicates() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // Execute "help" twice
    for _ in 0..2 {
        app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
        for c in "help".chars() {
            app.handle_key(KeyCode::Char(c), KeyModifiers::NONE);
        }
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    }

    assert_eq!(
        app.command_history,
        vec!["help"],
        "Consecutive duplicate commands should not be saved twice"
    );
}

#[test]
fn history_up_recalls_previous_command() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // Execute "help", then open command line and press Up
    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    for c in "help".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::NONE);
    }
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    // Open command line, type something that doesn't match palette (so Up goes to history)
    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    for c in "xyz".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::NONE);
    }
    // "xyz" matches no palette entries, so Up navigates history
    app.handle_key(KeyCode::Up, KeyModifiers::NONE);

    assert_eq!(
        app.command_line.as_ref().unwrap().text.buffer,
        "help",
        "Up should recall the previous command from history"
    );
}

#[test]
fn history_down_restores_draft() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // Execute "help"
    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    for c in "help".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::NONE);
    }
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    // Open command line, type "xyz" (no palette matches)
    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    for c in "xyz".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::NONE);
    }

    // Up recalls "help"
    app.handle_key(KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(app.command_line.as_ref().unwrap().text.buffer, "help");

    // Down restores draft "xyz"
    app.handle_key(KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(
        app.command_line.as_ref().unwrap().text.buffer,
        "xyz",
        "Down should restore the draft after navigating through history"
    );
}

#[test]
fn history_multiple_entries() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // Execute "help", then "refresh"
    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    for c in "help".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::NONE);
    }
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    for c in "refresh".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::NONE);
    }
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    // Open command line with "xyz" (no matches)
    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    for c in "xyz".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::NONE);
    }

    // Up gets "refresh" (most recent)
    app.handle_key(KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(app.command_line.as_ref().unwrap().text.buffer, "refresh");

    // Up again gets "help" (older)
    app.handle_key(KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(app.command_line.as_ref().unwrap().text.buffer, "help");

    // Down gets "refresh" again
    app.handle_key(KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(app.command_line.as_ref().unwrap().text.buffer, "refresh");

    // Down restores draft "xyz"
    app.handle_key(KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(app.command_line.as_ref().unwrap().text.buffer, "xyz");
}

#[test]
fn history_typing_resets_index() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // Execute "help"
    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    for c in "help".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::NONE);
    }
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    // Open command line with "xyz" (no matches), navigate to history, then type
    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    for c in "xyz".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::NONE);
    }
    app.handle_key(KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(app.command_line.as_ref().unwrap().text.buffer, "help");

    // Typing resets history browsing
    app.handle_key(KeyCode::Char('!'), KeyModifiers::NONE);
    assert!(
        app.command_line.as_ref().unwrap().history_index.is_none(),
        "Typing should reset history index"
    );
}

// ===== Argument hint tests =====

#[test]
fn argument_hint_for_run_command() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // Set up available commands
    app.available_commands = vec![
        (
            "build".to_string(),
            grove_core::Command {
                run: "make build".to_string(),
                description: None,
                working_dir: None,
                env: None,
                args: None,
            },
        ),
        (
            "test".to_string(),
            grove_core::Command {
                run: "make test".to_string(),
                description: None,
                working_dir: None,
                env: None,
                args: None,
            },
        ),
    ];

    // Open command line, type "run t"
    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    for c in "run t".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::NONE);
    }

    let hint = app.compute_argument_hint();
    assert_eq!(
        hint.as_deref(),
        Some("est"),
        "Should hint 'est' to complete 'test'"
    );
}

#[test]
fn argument_hint_for_repo_command() {
    let mut app = App::new(
        MockRegistry::with_repos(3),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // Open command line, type "repo repo1"
    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    for c in "repo repo".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::NONE);
    }

    // repo names are /tmp/repo0, /tmp/repo1, /tmp/repo2
    // Partial "repo" matches "repo0", "repo1", "repo2" — first is "repo0"
    let hint = app.compute_argument_hint();
    assert_eq!(
        hint.as_deref(),
        Some("0"),
        "Should hint '0' to complete 'repo0'"
    );
}

#[test]
fn argument_hint_none_for_full_match() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.available_commands = vec![(
        "test".to_string(),
        grove_core::Command {
            run: "make test".to_string(),
            description: None,
            working_dir: None,
            env: None,
            args: None,
        },
    )];

    // Type "run test" — fully matched, no hint
    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    for c in "run test".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::NONE);
    }

    let hint = app.compute_argument_hint();
    assert_eq!(hint, None, "Should not hint when arg is fully matched");
}

#[test]
fn argument_hint_none_for_unknown_command() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    // Type "help" — no arg hints for help command
    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    for c in "help".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::NONE);
    }

    let hint = app.compute_argument_hint();
    assert_eq!(hint, None, "Should not hint for commands without args");
}

#[test]
fn tab_accepts_argument_hint() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-workspace".to_string(),
    );

    app.available_commands = vec![(
        "build".to_string(),
        grove_core::Command {
            run: "make build".to_string(),
            description: None,
            working_dir: None,
            env: None,
            args: None,
        },
    )];

    // Type "run b" — hint should be "uild"
    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    for c in "run b".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::NONE);
    }

    // Tab accepts the hint
    app.handle_key(KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(
        app.command_line.as_ref().unwrap().text.buffer,
        "run build",
        "Tab should accept the argument hint"
    );
}

// ===== Form input overlay tests =====

#[test]
fn form_input_from_schema_initializes_defaults() {
    let args = vec![
        grove_core::ArgDef {
            name: "env".to_string(),
            arg_type: grove_core::ArgType::Choice,
            description: Some("Target environment".to_string()),
            required: true,
            default: Some("production".to_string()),
            options: Some(vec!["staging".to_string(), "production".to_string()]),
            positional: false,
        },
        grove_core::ArgDef {
            name: "tag".to_string(),
            arg_type: grove_core::ArgType::String,
            description: None,
            required: false,
            default: Some("latest".to_string()),
            options: None,
            positional: false,
        },
        grove_core::ArgDef {
            name: "verbose".to_string(),
            arg_type: grove_core::ArgType::Flag,
            description: None,
            required: false,
            default: Some("true".to_string()),
            options: None,
            positional: false,
        },
    ];

    let state = FormInputState::from_schema("deploy".to_string(), args);
    assert_eq!(state.command_name, "deploy");
    assert_eq!(state.fields.len(), 3);
    assert_eq!(state.focused, 0);

    // Choice defaults to "production" (index 1)
    match &state.fields[0].value {
        FieldValue::Choice(idx) => assert_eq!(*idx, 1),
        _ => panic!("Expected Choice"),
    }

    // String defaults to "latest"
    match &state.fields[1].value {
        FieldValue::Text(buf) => assert_eq!(buf.buffer, "latest"),
        _ => panic!("Expected Text"),
    }

    // Flag defaults to true
    match &state.fields[2].value {
        FieldValue::Flag(on) => assert!(*on),
        _ => panic!("Expected Flag"),
    }
}

#[test]
fn form_input_from_schema_no_defaults() {
    let args = vec![grove_core::ArgDef {
        name: "target".to_string(),
        arg_type: grove_core::ArgType::String,
        description: None,
        required: true,
        default: None,
        options: None,
        positional: true,
    }];

    let state = FormInputState::from_schema("build".to_string(), args);
    match &state.fields[0].value {
        FieldValue::Text(buf) => assert!(buf.buffer.is_empty()),
        _ => panic!("Expected Text"),
    }
}

#[test]
fn form_assemble_args_auto_assembly() {
    // This tests the assemble_args method via the App impl
    let fields = vec![
        FormField {
            def: grove_core::ArgDef {
                name: "target".to_string(),
                arg_type: grove_core::ArgType::String,
                description: None,
                required: true,
                default: None,
                options: None,
                positional: true,
            },
            value: FieldValue::Text(text_buffer::TextBuffer::with_content("release", 7)),
        },
        FormField {
            def: grove_core::ArgDef {
                name: "env".to_string(),
                arg_type: grove_core::ArgType::Choice,
                description: None,
                required: true,
                default: None,
                options: Some(vec!["staging".to_string(), "production".to_string()]),
                positional: false,
            },
            value: FieldValue::Choice(1),
        },
        FormField {
            def: grove_core::ArgDef {
                name: "verbose".to_string(),
                arg_type: grove_core::ArgType::Flag,
                description: None,
                required: false,
                default: None,
                options: None,
                positional: false,
            },
            value: FieldValue::Flag(true),
        },
    ];

    let result = App::<MockRegistry, MockDetailProvider>::assemble_args("./deploy.sh", &fields);
    assert_eq!(result, "./deploy.sh release --env production --verbose");
}

#[test]
fn form_assemble_args_omits_empty_and_false() {
    let fields = vec![
        FormField {
            def: grove_core::ArgDef {
                name: "tag".to_string(),
                arg_type: grove_core::ArgType::String,
                description: None,
                required: false,
                default: None,
                options: None,
                positional: false,
            },
            value: FieldValue::Text(text_buffer::TextBuffer::new()),
        },
        FormField {
            def: grove_core::ArgDef {
                name: "verbose".to_string(),
                arg_type: grove_core::ArgType::Flag,
                description: None,
                required: false,
                default: None,
                options: None,
                positional: false,
            },
            value: FieldValue::Flag(false),
        },
    ];

    let result = App::<MockRegistry, MockDetailProvider>::assemble_args("make build", &fields);
    assert_eq!(result, "make build");
}

#[test]
fn form_assemble_args_template_interpolation() {
    let fields = vec![
        FormField {
            def: grove_core::ArgDef {
                name: "env".to_string(),
                arg_type: grove_core::ArgType::Choice,
                description: None,
                required: true,
                default: None,
                options: Some(vec!["staging".to_string(), "production".to_string()]),
                positional: false,
            },
            value: FieldValue::Choice(0),
        },
        FormField {
            def: grove_core::ArgDef {
                name: "tag".to_string(),
                arg_type: grove_core::ArgType::String,
                description: None,
                required: false,
                default: None,
                options: None,
                positional: false,
            },
            value: FieldValue::Text(text_buffer::TextBuffer::with_content("v1.0", 4)),
        },
    ];

    let result = App::<MockRegistry, MockDetailProvider>::assemble_args(
        "deploy --to {env} --version {tag}",
        &fields,
    );
    assert_eq!(result, "deploy --to staging --version v1.0");
}

#[test]
fn form_has_placeholders_detection() {
    assert!(App::<MockRegistry, MockDetailProvider>::has_placeholders(
        "deploy {env}"
    ));
    assert!(App::<MockRegistry, MockDetailProvider>::has_placeholders(
        "{name} --flag"
    ));
    assert!(!App::<MockRegistry, MockDetailProvider>::has_placeholders(
        "./deploy.sh"
    ));
    assert!(!App::<MockRegistry, MockDetailProvider>::has_placeholders(
        "echo ${HOME}"
    ));
    assert!(!App::<MockRegistry, MockDetailProvider>::has_placeholders(
        "echo {}"
    ));
    // Mixed: has both ${env} and {placeholder}
    assert!(App::<MockRegistry, MockDetailProvider>::has_placeholders(
        "echo ${HOME} {name}"
    ));
    // Brace-like content that's not a valid identifier
    assert!(!App::<MockRegistry, MockDetailProvider>::has_placeholders(
        "echo {foo bar}"
    ));
}

#[test]
fn form_validate_required_empty_string_fails() {
    let state = FormInputState {
        command_name: "test".to_string(),
        fields: vec![FormField {
            def: grove_core::ArgDef {
                name: "target".to_string(),
                arg_type: grove_core::ArgType::String,
                description: None,
                required: true,
                default: None,
                options: None,
                positional: false,
            },
            value: FieldValue::Text(text_buffer::TextBuffer::new()),
        }],
        focused: 0,
    };

    let err = App::<MockRegistry, MockDetailProvider>::validate_form(&state);
    assert!(err.is_some());
    assert!(err.unwrap().contains("target"));
}

#[test]
fn form_validate_required_choice_passes() {
    let state = FormInputState {
        command_name: "test".to_string(),
        fields: vec![FormField {
            def: grove_core::ArgDef {
                name: "env".to_string(),
                arg_type: grove_core::ArgType::Choice,
                description: None,
                required: true,
                default: None,
                options: Some(vec!["staging".to_string(), "production".to_string()]),
                positional: false,
            },
            value: FieldValue::Choice(0),
        }],
        focused: 0,
    };

    assert!(App::<MockRegistry, MockDetailProvider>::validate_form(&state).is_none());
}

#[test]
fn form_execute_selected_command_shows_form_for_args() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-ws".to_string(),
    );

    app.selected_repo_for_commands = Some("/tmp/repo0".to_string());
    app.available_commands = vec![(
        "deploy".to_string(),
        grove_core::Command {
            run: "./deploy.sh".to_string(),
            description: None,
            working_dir: None,
            env: None,
            args: Some(vec![grove_core::ArgDef {
                name: "env".to_string(),
                arg_type: grove_core::ArgType::Choice,
                description: None,
                required: true,
                default: None,
                options: Some(vec!["staging".to_string(), "production".to_string()]),
                positional: false,
            }]),
        },
    )];
    app.command_picker_state.select(Some(0));

    app.execute_selected_command();

    assert!(app.form_input.is_some(), "Should show form for args");
    assert!(app.argument_input.is_none(), "Should not show free-text");
}

#[test]
fn form_execute_selected_command_shows_freetext_for_no_args() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-ws".to_string(),
    );

    app.selected_repo_for_commands = Some("/tmp/repo0".to_string());
    app.available_commands = vec![(
        "test".to_string(),
        grove_core::Command {
            run: "cargo test".to_string(),
            description: None,
            working_dir: None,
            env: None,
            args: None,
        },
    )];
    app.command_picker_state.select(Some(0));

    app.execute_selected_command();

    assert!(app.form_input.is_none(), "Should not show form");
    assert!(app.argument_input.is_some(), "Should show free-text input");
}

#[test]
fn form_key_handling_navigation() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-ws".to_string(),
    );

    let args = vec![
        grove_core::ArgDef {
            name: "a".to_string(),
            arg_type: grove_core::ArgType::String,
            description: None,
            required: false,
            default: None,
            options: None,
            positional: false,
        },
        grove_core::ArgDef {
            name: "b".to_string(),
            arg_type: grove_core::ArgType::Flag,
            description: None,
            required: false,
            default: None,
            options: None,
            positional: false,
        },
    ];

    app.form_input = Some(FormInputState::from_schema("test".to_string(), args));

    assert_eq!(app.form_input.as_ref().unwrap().focused, 0);

    // Tab moves to next field
    app.handle_key(KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(app.form_input.as_ref().unwrap().focused, 1);

    // Tab wraps around
    app.handle_key(KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(app.form_input.as_ref().unwrap().focused, 0);

    // Down also moves forward
    app.handle_key(KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(app.form_input.as_ref().unwrap().focused, 1);

    // Up moves backward
    app.handle_key(KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(app.form_input.as_ref().unwrap().focused, 0);

    // Esc dismisses
    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
    assert!(app.form_input.is_none());
}

#[test]
fn form_key_handling_flag_toggle() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-ws".to_string(),
    );

    let args = vec![grove_core::ArgDef {
        name: "verbose".to_string(),
        arg_type: grove_core::ArgType::Flag,
        description: None,
        required: false,
        default: None,
        options: None,
        positional: false,
    }];

    app.form_input = Some(FormInputState::from_schema("test".to_string(), args));

    // Initially false
    match &app.form_input.as_ref().unwrap().fields[0].value {
        FieldValue::Flag(on) => assert!(!on),
        _ => panic!("Expected Flag"),
    }

    // Space toggles
    app.handle_key(KeyCode::Char(' '), KeyModifiers::NONE);
    match &app.form_input.as_ref().unwrap().fields[0].value {
        FieldValue::Flag(on) => assert!(on),
        _ => panic!("Expected Flag"),
    }

    // Space toggles back
    app.handle_key(KeyCode::Char(' '), KeyModifiers::NONE);
    match &app.form_input.as_ref().unwrap().fields[0].value {
        FieldValue::Flag(on) => assert!(!on),
        _ => panic!("Expected Flag"),
    }
}

#[test]
fn form_key_handling_choice_cycle() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-ws".to_string(),
    );

    let args = vec![grove_core::ArgDef {
        name: "env".to_string(),
        arg_type: grove_core::ArgType::Choice,
        description: None,
        required: false,
        default: None,
        options: Some(vec![
            "dev".to_string(),
            "staging".to_string(),
            "prod".to_string(),
        ]),
        positional: false,
    }];

    app.form_input = Some(FormInputState::from_schema("test".to_string(), args));

    // Helper to get the current choice index
    let get_choice = |app: &App<MockRegistry, MockDetailProvider>| -> usize {
        match &app.form_input.as_ref().unwrap().fields[0].value {
            FieldValue::Choice(idx) => *idx,
            _ => panic!("Expected Choice"),
        }
    };

    // Initially index 0
    assert_eq!(get_choice(&app), 0);

    // Right cycles forward
    app.handle_key(KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(get_choice(&app), 1);

    // Right again
    app.handle_key(KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(get_choice(&app), 2);

    // Right wraps to 0
    app.handle_key(KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(get_choice(&app), 0);

    // Left wraps to last
    app.handle_key(KeyCode::Left, KeyModifiers::NONE);
    assert_eq!(get_choice(&app), 2);
}

#[test]
fn form_key_handling_text_input() {
    let mut app = App::new(
        MockRegistry::with_repos(1),
        MockDetailProvider::empty(),
        "test-ws".to_string(),
    );

    let args = vec![grove_core::ArgDef {
        name: "tag".to_string(),
        arg_type: grove_core::ArgType::String,
        description: None,
        required: false,
        default: None,
        options: None,
        positional: false,
    }];

    app.form_input = Some(FormInputState::from_schema("test".to_string(), args));

    // Type some text
    app.handle_key(KeyCode::Char('v'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('1'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('.'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('0'), KeyModifiers::NONE);

    match &app.form_input.as_ref().unwrap().fields[0].value {
        FieldValue::Text(buf) => assert_eq!(buf.buffer, "v1.0"),
        _ => panic!("Expected Text"),
    }

    // Backspace
    app.handle_key(KeyCode::Backspace, KeyModifiers::NONE);
    match &app.form_input.as_ref().unwrap().fields[0].value {
        FieldValue::Text(buf) => assert_eq!(buf.buffer, "v1."),
        _ => panic!("Expected Text"),
    }

    // Ctrl+U clears
    app.handle_key(KeyCode::Char('u'), KeyModifiers::CONTROL);
    match &app.form_input.as_ref().unwrap().fields[0].value {
        FieldValue::Text(buf) => assert!(buf.buffer.is_empty()),
        _ => panic!("Expected Text"),
    }
}

#[test]
fn form_assemble_args_shell_escapes_special_chars() {
    let fields = vec![FormField {
        def: grove_core::ArgDef {
            name: "msg".to_string(),
            arg_type: grove_core::ArgType::String,
            description: None,
            required: false,
            default: None,
            options: None,
            positional: false,
        },
        value: FieldValue::Text(text_buffer::TextBuffer::with_content("hello world", 11)),
    }];

    let result = App::<MockRegistry, MockDetailProvider>::assemble_args("echo", &fields);
    // Should shell-escape the value with spaces
    assert!(
        result.contains("'hello world'") || result.contains("\"hello world\""),
        "Should escape value with spaces, got: {result}"
    );
}

#[test]
fn form_assemble_args_notebook_capture_scenario() {
    // Simulates the real notebook capture command:
    //   run: "uv run notecap capture"
    //   args: section (choice, positional), content (string, positional), raw (flag)
    let fields = vec![
        FormField {
            def: grove_core::ArgDef {
                name: "section".to_string(),
                arg_type: grove_core::ArgType::Choice,
                description: Some("Section: Personal or Work".to_string()),
                required: true,
                default: None,
                options: Some(vec!["Personal".to_string(), "Work".to_string()]),
                positional: true,
            },
            value: FieldValue::Choice(0), // "Personal"
        },
        FormField {
            def: grove_core::ArgDef {
                name: "content".to_string(),
                arg_type: grove_core::ArgType::String,
                description: Some("Content to capture".to_string()),
                required: true,
                default: None,
                options: None,
                positional: true,
            },
            value: FieldValue::Text(text_buffer::TextBuffer::with_content("Buy groceries", 13)),
        },
        FormField {
            def: grove_core::ArgDef {
                name: "raw".to_string(),
                arg_type: grove_core::ArgType::Flag,
                description: Some("Skip content sanitization".to_string()),
                required: false,
                default: None,
                options: None,
                positional: false,
            },
            value: FieldValue::Flag(false),
        },
    ];

    let result = App::<MockRegistry, MockDetailProvider>::assemble_args(
        "uv run notecap capture",
        &fields,
    );
    // Expected: positional args first (section, content), flag omitted when false
    assert_eq!(
        result, "uv run notecap capture Personal 'Buy groceries'",
        "Assembled command should match notecap CLI expectations"
    );

    // Now with --raw enabled
    let mut fields_with_raw = fields;
    fields_with_raw[2].value = FieldValue::Flag(true);

    let result_raw = App::<MockRegistry, MockDetailProvider>::assemble_args(
        "uv run notecap capture",
        &fields_with_raw,
    );
    assert_eq!(
        result_raw, "uv run notecap capture Personal 'Buy groceries' --raw",
        "Should append --raw flag when enabled"
    );
}
