//! Unit tests for TUI module (transcript paradigm)

use crossterm::event::{KeyCode, KeyModifiers};
use grove_core::{
    FileChangeStatus, RepoDetail, RepoDetailProvider, RepoPath, RepoRegistry, RepoStatus, Result,
};
use std::collections::HashMap;

use super::command_line::{filtered_palette, parse_command, CliCommand, PALETTE_COMMANDS};
use super::formatting::{compact_path, extract_basename, format_file_change_indicator};
use super::prompt::{
    compute_run_completions, extract_command_prefix, ghost_hint_suffix, ArgCompletion,
    CompletionState, PickerItem, PickerOutcome, PickerState, PromptState,
};
use super::scroll_buffer::{BlockId, ContentBlock, ScrollBuffer};
use super::transcript::{extract_options_from_state, TranscriptApp};

// ===== Mock infrastructure =====

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

    #[allow(dead_code)]
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

    fn get_status(&self, repo_path: &RepoPath) -> Option<&RepoStatus> {
        self.statuses.get(repo_path)
    }

    fn refresh_all(&mut self) -> Result<grove_core::RefreshStats> {
        Ok(grove_core::RefreshStats {
            successful: self.repos.len(),
            failed: 0,
        })
    }
}

struct MockDetailProvider;

impl RepoDetailProvider for MockDetailProvider {
    fn get_detail(&self, _path: &RepoPath, _max_commits: usize) -> Result<RepoDetail> {
        Ok(RepoDetail::empty())
    }
}

fn create_app(registry: MockRegistry) -> TranscriptApp<MockRegistry, MockDetailProvider> {
    TranscriptApp::new(registry, MockDetailProvider, "test-workspace".to_string())
}

// ===== TranscriptApp tests =====

#[test]
fn new_app_selects_first_repo() {
    let app = create_app(MockRegistry::with_repos(3));
    assert_eq!(app.context.selected_index, Some(0));
    assert!(!app.should_quit);
}

#[test]
fn new_app_empty_registry() {
    let app = create_app(MockRegistry::empty());
    assert_eq!(app.context.selected_index, None);
}

#[test]
fn quit_key_sets_should_quit() {
    let mut app = create_app(MockRegistry::with_repos(1));
    app.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);
    assert!(app.should_quit);
}

#[test]
fn colon_opens_command_line() {
    let mut app = create_app(MockRegistry::with_repos(1));
    assert!(!app.prompt.is_active());
    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    assert!(app.prompt.is_active());
}

#[test]
fn esc_closes_command_line() {
    let mut app = create_app(MockRegistry::with_repos(1));
    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    assert!(app.prompt.is_active());

    // Send Esc to close command line
    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
    assert!(!app.prompt.is_active());
}

#[test]
fn help_command_pushes_help_block() {
    let mut app = create_app(MockRegistry::with_repos(1));
    let initial_blocks = app.scroll.blocks.len();

    // Open command line and type "help"
    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('e'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('l'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('p'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert!(app.scroll.blocks.len() > initial_blocks);
    assert!(!app.prompt.is_active());
}

#[test]
fn refresh_key_triggers_refresh() {
    let mut app = create_app(MockRegistry::with_repos(1));
    app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
    assert!(app.needs_refresh);
}

#[test]
fn scroll_keys_work() {
    let mut app = create_app(MockRegistry::with_repos(1));
    // Push some blocks to have content
    for _ in 0..5 {
        app.scroll.push(ContentBlock::Text {
            id: BlockId::new(),
            lines: vec![ratatui::text::Line::from("test line")],
            collapsed: false,
        });
    }

    // j scrolls down
    app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    // k scrolls up
    app.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
}

#[test]
fn repo_command_switches_context() {
    let mut app = create_app(MockRegistry::with_repos(3));
    assert_eq!(app.context.selected_index, Some(0));

    // :repo 2
    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    for c in "repo 2".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::NONE);
    }
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert_eq!(app.context.selected_index, Some(1)); // 2 is 1-based
}

#[test]
fn repos_command_adds_table_block() {
    let mut app = create_app(MockRegistry::with_repos(3));
    let initial_blocks = app.scroll.blocks.len();

    // :repos
    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    for c in "repos".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::NONE);
    }
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert!(app.scroll.blocks.len() > initial_blocks);
    // The last block should be a table
    match app.scroll.blocks.last() {
        Some(ContentBlock::Table { title, rows, .. }) => {
            assert_eq!(title, "Repositories");
            assert_eq!(rows.len(), 3);
        }
        _ => panic!("Expected a Table block"),
    }
}

// ===== ScrollBuffer tests =====

#[test]
fn scroll_buffer_push_and_count() {
    let mut buf = ScrollBuffer::new();
    assert!(buf.blocks.is_empty());

    buf.push(ContentBlock::Text {
        id: BlockId::new(),
        lines: vec![ratatui::text::Line::from("hello")],
        collapsed: false,
    });
    assert_eq!(buf.blocks.len(), 1);
}

#[test]
fn scroll_buffer_clear() {
    let mut buf = ScrollBuffer::new();
    buf.push(ContentBlock::Text {
        id: BlockId::new(),
        lines: vec![ratatui::text::Line::from("hello")],
        collapsed: false,
    });
    buf.clear();
    assert!(buf.blocks.is_empty());
}

#[test]
fn block_id_unique() {
    let id1 = BlockId::new();
    let id2 = BlockId::new();
    assert_ne!(id1, id2);
}

#[test]
fn content_block_collapse_toggle() {
    let mut block = ContentBlock::Text {
        id: BlockId::new(),
        lines: vec![ratatui::text::Line::from("test")],
        collapsed: false,
    };
    assert!(!block.is_collapsed());
    block.toggle_collapse();
    assert!(block.is_collapsed());
    block.toggle_collapse();
    assert!(!block.is_collapsed());
}

#[test]
fn divider_not_collapsible() {
    let mut block = ContentBlock::Divider { id: BlockId::new() };
    assert!(!block.is_collapsed());
    block.toggle_collapse();
    assert!(!block.is_collapsed());
}

#[test]
fn focus_next_prev() {
    let mut buf = ScrollBuffer::new();
    for _ in 0..3 {
        buf.push(ContentBlock::Text {
            id: BlockId::new(),
            lines: vec![ratatui::text::Line::from("line")],
            collapsed: false,
        });
    }

    assert_eq!(buf.focused_block, None);
    buf.focus_next();
    assert_eq!(buf.focused_block, Some(0));
    buf.focus_next();
    assert_eq!(buf.focused_block, Some(1));
    buf.focus_next();
    assert_eq!(buf.focused_block, Some(2));
    buf.focus_next(); // should stay at 2
    assert_eq!(buf.focused_block, Some(2));

    buf.focus_prev();
    assert_eq!(buf.focused_block, Some(1));
    buf.focus_prev();
    assert_eq!(buf.focused_block, Some(0));
    buf.focus_prev(); // should stay at 0
    assert_eq!(buf.focused_block, Some(0));
}

// ===== Formatting tests =====

#[test]
fn basename_simple() {
    assert_eq!(extract_basename("/home/user/src/graft"), "graft");
    assert_eq!(extract_basename("graft"), "graft");
    assert_eq!(extract_basename("/tmp"), "tmp");
}

#[test]
fn compact_path_no_truncation() {
    let path = "/short";
    assert_eq!(compact_path(path, 100), path);
}

#[test]
fn file_change_indicator_colors() {
    let (indicator, color) = format_file_change_indicator(&FileChangeStatus::Modified);
    assert_eq!(indicator, "M");
    assert_eq!(color, ratatui::style::Color::Yellow);

    let (indicator, color) = format_file_change_indicator(&FileChangeStatus::Added);
    assert_eq!(indicator, "A");
    assert_eq!(color, ratatui::style::Color::Green);

    let (indicator, color) = format_file_change_indicator(&FileChangeStatus::Deleted);
    assert_eq!(indicator, "D");
    assert_eq!(color, ratatui::style::Color::Red);
}

// ===== Command parsing tests =====

#[test]
fn parse_command_help() {
    assert_eq!(parse_command("help"), CliCommand::Help);
    assert_eq!(parse_command("h"), CliCommand::Help);
}

#[test]
fn parse_command_quit() {
    assert_eq!(parse_command("quit"), CliCommand::Quit);
    assert_eq!(parse_command("q"), CliCommand::Quit);
}

#[test]
fn parse_command_refresh() {
    assert_eq!(parse_command("refresh"), CliCommand::Refresh);
    assert_eq!(parse_command("r"), CliCommand::Refresh);
}

#[test]
fn parse_command_repos() {
    assert_eq!(parse_command("repos"), CliCommand::Repos);
}

#[test]
fn parse_command_repo() {
    assert_eq!(
        parse_command("repo graft"),
        CliCommand::Repo("graft".to_string())
    );
    assert_eq!(parse_command("repo 1"), CliCommand::Repo("1".to_string()));
}

#[test]
fn parse_command_run() {
    assert_eq!(
        parse_command("run test"),
        CliCommand::Run("test".to_string(), vec![])
    );
    assert_eq!(
        parse_command("run deploy --env staging"),
        CliCommand::Run(
            "deploy".to_string(),
            vec!["--env".to_string(), "staging".to_string()]
        )
    );
}

#[test]
fn parse_command_unknown() {
    assert_eq!(
        parse_command("frobnicate"),
        CliCommand::Unknown("frobnicate".to_string())
    );
    assert_eq!(parse_command(""), CliCommand::Unknown(String::new()));
}

#[test]
fn palette_commands_exist() {
    assert!(PALETTE_COMMANDS.len() >= 10);
    assert!(PALETTE_COMMANDS.iter().any(|e| e.command == "help"));
    assert!(PALETTE_COMMANDS.iter().any(|e| e.command == "quit"));
    assert!(PALETTE_COMMANDS.iter().any(|e| e.command == "repos"));
    assert!(PALETTE_COMMANDS.iter().any(|e| e.command == "status"));
    assert!(PALETTE_COMMANDS.iter().any(|e| e.command == "catalog"));
    assert!(PALETTE_COMMANDS.iter().any(|e| e.command == "state"));
    assert!(PALETTE_COMMANDS.iter().any(|e| e.command == "invalidate"));
}

#[test]
fn filtered_palette_all() {
    let all = filtered_palette("");
    assert_eq!(all.len(), PALETTE_COMMANDS.len());
}

#[test]
fn filtered_palette_filter() {
    let matches = filtered_palette("re");
    assert!(matches.iter().any(|e| e.command == "refresh"));
    assert!(matches.iter().any(|e| e.command == "repo"));
    assert!(matches.iter().any(|e| e.command == "repos"));
}

#[test]
fn filtered_palette_no_match() {
    let matches = filtered_palette("zzz");
    assert!(matches.is_empty());
}

// ===== TextBuffer tests =====

#[test]
fn text_buffer_insert_and_backspace() {
    let mut buf = super::text_buffer::TextBuffer::new();
    buf.insert_char('h');
    buf.insert_char('i');
    assert_eq!(buf.buffer, "hi");
    assert_eq!(buf.cursor_pos, 2);

    buf.backspace();
    assert_eq!(buf.buffer, "h");
    assert_eq!(buf.cursor_pos, 1);
}

#[test]
fn text_buffer_clear() {
    let mut buf = super::text_buffer::TextBuffer::new();
    buf.set("hello world");
    buf.clear();
    assert_eq!(buf.buffer, "");
    assert_eq!(buf.cursor_pos, 0);
}

#[test]
fn text_buffer_delete_word_backward() {
    let mut buf = super::text_buffer::TextBuffer::new();
    buf.set("hello world");
    buf.delete_word_backward();
    assert_eq!(buf.buffer, "hello ");
    assert_eq!(buf.cursor_pos, 6);
}

#[test]
fn text_buffer_cursor_movement() {
    let mut buf = super::text_buffer::TextBuffer::new();
    buf.set("abc");
    assert_eq!(buf.cursor_pos, 3);
    buf.move_left();
    assert_eq!(buf.cursor_pos, 2);
    buf.move_home();
    assert_eq!(buf.cursor_pos, 0);
    buf.move_end();
    assert_eq!(buf.cursor_pos, 3);
}

// ===== PromptState tests =====

#[test]
fn prompt_open_close() {
    let mut prompt = super::prompt::PromptState::new();
    assert!(!prompt.is_active());

    prompt.open();
    assert!(prompt.is_active());

    prompt.close();
    assert!(!prompt.is_active());
}

#[test]
fn prompt_esc_closes() {
    let mut prompt = super::prompt::PromptState::new();
    prompt.open();
    let result = prompt.handle_key(
        KeyCode::Esc,
        KeyModifiers::NONE,
        &CompletionState::default(),
    );
    assert!(result.is_none());
    assert!(!prompt.is_active());
}

#[test]
fn prompt_enter_submits() {
    let mut prompt = super::prompt::PromptState::new();
    prompt.open();

    // Type "help"
    prompt.handle_key(
        KeyCode::Char('h'),
        KeyModifiers::NONE,
        &CompletionState::default(),
    );
    prompt.handle_key(
        KeyCode::Char('e'),
        KeyModifiers::NONE,
        &CompletionState::default(),
    );
    prompt.handle_key(
        KeyCode::Char('l'),
        KeyModifiers::NONE,
        &CompletionState::default(),
    );
    prompt.handle_key(
        KeyCode::Char('p'),
        KeyModifiers::NONE,
        &CompletionState::default(),
    );

    let result = prompt.handle_key(
        KeyCode::Enter,
        KeyModifiers::NONE,
        &CompletionState::default(),
    );
    assert_eq!(result, Some(CliCommand::Help));
    assert!(!prompt.is_active());
}

#[test]
fn prompt_history() {
    let mut prompt = super::prompt::PromptState::new();

    // Submit "help" to history
    prompt.open();
    for c in "help".chars() {
        prompt.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    prompt.handle_key(
        KeyCode::Enter,
        KeyModifiers::NONE,
        &CompletionState::default(),
    );
    assert_eq!(prompt.history.len(), 1);

    // Submit "repos" to history
    prompt.open();
    for c in "repos".chars() {
        prompt.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    prompt.handle_key(
        KeyCode::Enter,
        KeyModifiers::NONE,
        &CompletionState::default(),
    );
    assert_eq!(prompt.history.len(), 2);
}

#[test]
fn prompt_palette_enter_selects_command() {
    // Find the first palette entry that doesn't take args and navigate to it
    let no_args_index = PALETTE_COMMANDS.iter().position(|e| !e.takes_args).unwrap();
    let expected = parse_command(PALETTE_COMMANDS[no_args_index].command);

    let mut prompt = super::prompt::PromptState::new();
    prompt.open();

    // Navigate down to the first no-args entry
    for _ in 0..no_args_index {
        prompt.handle_key(
            KeyCode::Down,
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }

    let result = prompt.handle_key(
        KeyCode::Enter,
        KeyModifiers::NONE,
        &CompletionState::default(),
    );
    assert_eq!(result, Some(expected));
    assert!(!prompt.is_active());
}

#[test]
fn prompt_palette_enter_fills_args_command() {
    // Type a prefix that uniquely matches a takes_args command
    let mut prompt = super::prompt::PromptState::new();
    prompt.open();

    // Type "repo" to filter to just "repo" and "repos"
    for c in "repo".chars() {
        prompt.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    // First match should be "repo" (takes_args: true)
    let result = prompt.handle_key(
        KeyCode::Enter,
        KeyModifiers::NONE,
        &CompletionState::default(),
    );
    // Should fill buffer with "repo " and stay active (needs args)
    assert!(result.is_none());
    assert!(prompt.is_active());
    assert_eq!(prompt.command_line.as_ref().unwrap().text.buffer, "repo ");
}

// ===== New command integration tests =====

/// Helper to type a command string and press Enter.
fn type_command(app: &mut TranscriptApp<MockRegistry, MockDetailProvider>, cmd: &str) {
    app.handle_key(KeyCode::Char(':'), KeyModifiers::NONE);
    for c in cmd.chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::NONE);
    }
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
}

#[test]
fn status_command_pushes_text_block() {
    let mut app = create_app(MockRegistry::with_repos(1));
    let initial_blocks = app.scroll.blocks.len();

    type_command(&mut app, "status");

    assert!(app.scroll.blocks.len() > initial_blocks);
    match app.scroll.blocks.last() {
        Some(ContentBlock::Text { lines, .. }) => {
            let text: String = lines
                .iter()
                .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
                .collect::<Vec<_>>()
                .join(" ");
            assert!(
                text.contains("Changed Files"),
                "Expected 'Changed Files' in output"
            );
            assert!(
                text.contains("Recent Commits"),
                "Expected 'Recent Commits' in output"
            );
        }
        _ => panic!("Expected a Text block from :status"),
    }
}

#[test]
fn status_command_no_repo_shows_warning() {
    let mut app = create_app(MockRegistry::empty());
    type_command(&mut app, "status");
    assert!(app.status.is_some());
}

#[test]
fn catalog_command_no_repo_shows_warning() {
    let mut app = create_app(MockRegistry::empty());
    type_command(&mut app, "catalog");
    assert!(app.status.is_some());
}

#[test]
fn state_command_no_repo_shows_warning() {
    let mut app = create_app(MockRegistry::empty());
    type_command(&mut app, "state");
    assert!(app.status.is_some());
}

#[test]
fn invalidate_command_no_repo_shows_warning() {
    let mut app = create_app(MockRegistry::empty());
    type_command(&mut app, "invalidate");
    assert!(app.status.is_some());
}

// ===== Completion system test helpers =====

fn make_command(
    name: &str,
    desc: &str,
    args: Option<Vec<graft_common::ArgDef>>,
) -> (String, graft_common::CommandDef) {
    (
        name.to_string(),
        graft_common::CommandDef {
            run: "echo test".to_string(),
            description: Some(desc.to_string()),
            args,
            category: None,
            example: None,
            working_dir: None,
            env: None,
            stdin: None,
            context: None,
            writes: Vec::new(),
            reads: Vec::new(),
        },
    )
}

fn make_arg(
    name: &str,
    arg_type: graft_common::ArgType,
    required: bool,
    options: Option<Vec<String>>,
) -> graft_common::ArgDef {
    graft_common::ArgDef {
        name: name.to_string(),
        arg_type,
        description: None,
        required,
        default: None,
        options,
        options_from: None,
        positional: false,
    }
}

// ===== extract_command_prefix tests =====

#[test]
fn extract_command_prefix_with_space() {
    assert_eq!(extract_command_prefix("run build"), "run ");
}

#[test]
fn extract_command_prefix_trailing_space() {
    assert_eq!(extract_command_prefix("run "), "run ");
}

#[test]
fn extract_command_prefix_no_space() {
    assert_eq!(extract_command_prefix("run"), "run");
}

// ===== ghost_hint_suffix tests =====

#[test]
fn ghost_hint_suffix_partial() {
    assert_eq!(
        ghost_hint_suffix("run bu", "build"),
        Some("ild".to_string())
    );
}

#[test]
fn ghost_hint_suffix_exact() {
    assert_eq!(ghost_hint_suffix("run build", "build"), None);
}

#[test]
fn ghost_hint_suffix_case_insensitive() {
    assert_eq!(
        ghost_hint_suffix("run BU", "build"),
        Some("ild".to_string())
    );
}

#[test]
fn ghost_hint_suffix_no_space() {
    assert_eq!(ghost_hint_suffix("build", "build"), None);
}

// ===== extract_options_from_state tests =====

#[test]
fn options_from_state_extracts_path_array() {
    let data = serde_json::json!({
        "slices": [
            {"path": "slices/foo/plan.md", "status": "draft"},
            {"path": "slices/bar/plan.md", "status": "in-progress"},
            {"path": "slices/baz/plan.md", "status": "accepted"},
        ]
    });
    let opts = extract_options_from_state("slices", &data, None);
    assert_eq!(opts, vec!["slices/foo", "slices/bar", "slices/baz"]);
}

#[test]
fn options_from_state_excludes_done_items() {
    let data = serde_json::json!({
        "slices": [
            {"path": "slices/active/plan.md", "status": "draft"},
            {"path": "slices/finished/plan.md", "status": "done"},
            {"path": "slices/wip/plan.md", "status": "in-progress"},
        ]
    });
    let opts = extract_options_from_state("slices", &data, None);
    assert_eq!(opts, vec!["slices/active", "slices/wip"]);
}

#[test]
fn options_from_state_extracts_string_array() {
    let data = serde_json::json!({"tags": ["alpha", "beta", "gamma"]});
    let opts = extract_options_from_state("tags", &data, None);
    assert_eq!(opts, vec!["alpha", "beta", "gamma"]);
}

#[test]
fn options_from_state_extracts_name_field() {
    let data = serde_json::json!({
        "envs": [{"name": "staging"}, {"name": "production"}]
    });
    let opts = extract_options_from_state("envs", &data, None);
    assert_eq!(opts, vec!["staging", "production"]);
}

#[test]
fn options_from_state_missing_key_returns_empty() {
    let data = serde_json::json!({"other": ["x", "y"]});
    let opts = extract_options_from_state("slices", &data, None);
    assert!(opts.is_empty());
}

#[test]
fn options_from_state_entity_default_collection() {
    // entity.collection not set → use query_name as collection key
    let entity = graft_common::EntityDef {
        key: "slug".to_string(),
        collection: None,
    };
    let data = serde_json::json!({
        "slices": [
            {"slug": "retry-logic", "status": "draft"},
            {"slug": "entity-focus", "status": "done"},
        ]
    });
    let opts = extract_options_from_state("slices", &data, Some(&entity));
    // All items returned (no status filtering with entity)
    assert_eq!(opts, vec!["retry-logic", "entity-focus"]);
}

#[test]
fn options_from_state_entity_explicit_collection() {
    // entity.collection overrides the query name as the array key
    let entity = graft_common::EntityDef {
        key: "id".to_string(),
        collection: Some("tasks".to_string()),
    };
    let data = serde_json::json!({
        "tasks": [
            {"id": "task-a", "name": "Task A"},
            {"id": "task-b", "name": "Task B"},
        ]
    });
    let opts = extract_options_from_state("active-tasks", &data, Some(&entity));
    assert_eq!(opts, vec!["task-a", "task-b"]);
}

#[test]
fn options_from_state_entity_missing_collection_key_returns_empty() {
    // entity.collection points to a key not present in the data
    let entity = graft_common::EntityDef {
        key: "name".to_string(),
        collection: Some("missing".to_string()),
    };
    let data = serde_json::json!({"environments": [{"name": "staging"}]});
    let opts = extract_options_from_state("environments", &data, Some(&entity));
    assert!(opts.is_empty());
}

#[test]
fn options_from_state_entity_skips_items_without_key() {
    // Items that lack the entity key field are silently skipped
    let entity = graft_common::EntityDef {
        key: "name".to_string(),
        collection: None,
    };
    let data = serde_json::json!({
        "envs": [
            {"name": "staging"},
            {"id": "no-name-here"},
            {"name": "production"},
        ]
    });
    let opts = extract_options_from_state("envs", &data, Some(&entity));
    assert_eq!(opts, vec!["staging", "production"]);
}

#[test]
fn run_completions_options_from_resolved() {
    // Simulate a command where options_from has been pre-resolved into options
    let commands = vec![make_command(
        "software-factory:implement",
        "Implement a slice",
        Some(vec![make_arg(
            "slice",
            graft_common::ArgType::Choice,
            true,
            Some(vec!["slices/foo".to_string(), "slices/bar".to_string()]),
        )]),
    )];
    let cs = compute_run_completions("software-factory:implement ", &commands);
    assert_eq!(cs.completions.len(), 2);
    let values: Vec<&str> = cs.completions.iter().map(|c| c.value.as_str()).collect();
    assert!(values.contains(&"slices/foo"));
    assert!(values.contains(&"slices/bar"));
    assert!(cs.requires_more_input);
}

// ===== compute_completions tests =====

#[test]
fn completions_empty_when_no_space() {
    // Simulate typing "run" with no space
    let mut p = super::prompt::PromptState::new();
    p.open();
    for c in "run".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let cs = p.compute_completions(&[], &[], &[], &HashMap::default(), &[]);
    assert!(cs.completions.is_empty());
}

#[test]
fn completions_run_command_names() {
    let commands = vec![
        make_command("test", "Run tests", None),
        make_command("build", "Build project", None),
    ];
    let mut p = super::prompt::PromptState::new();
    p.open();
    for c in "run ".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let cs = p.compute_completions(&commands, &[], &[], &HashMap::default(), &[]);
    assert_eq!(cs.completions.len(), 2);
    let values: Vec<&str> = cs.completions.iter().map(|c| c.value.as_str()).collect();
    assert!(values.contains(&"test"));
    assert!(values.contains(&"build"));
}

#[test]
fn completions_run_command_partial() {
    let commands = vec![
        make_command("test", "Run tests", None),
        make_command("build", "Build project", None),
    ];
    let mut p = super::prompt::PromptState::new();
    p.open();
    for c in "run te".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let cs = p.compute_completions(&commands, &[], &[], &HashMap::default(), &[]);
    assert_eq!(cs.completions.len(), 1);
    assert_eq!(cs.completions[0].value, "test");
}

#[test]
fn completions_repo_names() {
    let repos = vec!["graft".to_string(), "grove".to_string()];
    let mut p = super::prompt::PromptState::new();
    p.open();
    for c in "repo ".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let cs = p.compute_completions(&[], &repos, &[], &HashMap::default(), &[]);
    assert_eq!(cs.completions.len(), 2);
}

#[test]
fn completions_repo_partial() {
    let repos = vec![
        "graft".to_string(),
        "grove".to_string(),
        "other".to_string(),
    ];
    let mut p = super::prompt::PromptState::new();
    p.open();
    for c in "repo gr".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let cs = p.compute_completions(&[], &repos, &[], &HashMap::default(), &[]);
    assert_eq!(cs.completions.len(), 2);
    let values: Vec<&str> = cs.completions.iter().map(|c| c.value.as_str()).collect();
    assert!(values.contains(&"graft"));
    assert!(values.contains(&"grove"));
}

#[test]
fn completions_state_query_names() {
    let queries = vec!["coverage".to_string(), "deps".to_string()];
    let mut p = super::prompt::PromptState::new();
    p.open();
    for c in "state ".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let cs = p.compute_completions(&[], &[], &queries, &HashMap::default(), &[]);
    assert_eq!(cs.completions.len(), 2);
}

#[test]
fn completions_catalog_categories() {
    let mut p = super::prompt::PromptState::new();
    p.open();
    for c in "catalog ".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let cs = p.compute_completions(&[], &[], &[], &HashMap::default(), &[]);
    let values: Vec<&str> = cs.completions.iter().map(|c| c.value.as_str()).collect();
    assert!(values.contains(&"core"));
    assert!(values.contains(&"diagnostic"));
    assert!(values.contains(&"optional"));
    assert!(values.contains(&"advanced"));
    assert!(values.contains(&"uncategorized"));
}

#[test]
fn completions_cursor_not_at_end() {
    let mut p = super::prompt::PromptState::new();
    p.open();
    for c in "run ".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    // Move cursor to position 2 (not at end)
    p.command_line.as_mut().unwrap().text.cursor_pos = 2;
    let cs = p.compute_completions(&[], &[], &[], &HashMap::default(), &[]);
    assert!(cs.completions.is_empty());
}

// ===== compute_run_completions multi-arg tests =====

#[test]
fn run_completions_choice_arg() {
    let commands = vec![make_command(
        "deploy",
        "Deploy",
        Some(vec![make_arg(
            "env",
            graft_common::ArgType::Choice,
            true,
            Some(vec!["staging".to_string(), "production".to_string()]),
        )]),
    )];
    let cs = compute_run_completions("deploy ", &commands);
    assert_eq!(cs.completions.len(), 2);
    let values: Vec<&str> = cs.completions.iter().map(|c| c.value.as_str()).collect();
    assert!(values.contains(&"staging"));
    assert!(values.contains(&"production"));
    assert!(cs.requires_more_input);
}

#[test]
fn run_completions_flag_arg() {
    let commands = vec![make_command(
        "build",
        "Build",
        Some(vec![make_arg(
            "verbose",
            graft_common::ArgType::Flag,
            false,
            None,
        )]),
    )];
    let cs = compute_run_completions("build ", &commands);
    assert_eq!(cs.completions.len(), 2);
    let values: Vec<&str> = cs.completions.iter().map(|c| c.value.as_str()).collect();
    assert!(values.contains(&"true"));
    assert!(values.contains(&"false"));
}

#[test]
fn run_completions_string_arg() {
    let commands = vec![make_command(
        "greet",
        "Greet",
        Some(vec![make_arg(
            "name",
            graft_common::ArgType::String,
            false,
            None,
        )]),
    )];
    let cs = compute_run_completions("greet ", &commands);
    assert!(cs.completions.is_empty());
    assert_eq!(cs.arg_hint, Some("<name>".to_string()));
}

#[test]
fn run_completions_required_blocks_enter() {
    // Command with 2 required args: env (Choice) + region (Choice)
    let commands = vec![make_command(
        "deploy",
        "Deploy",
        Some(vec![
            make_arg(
                "env",
                graft_common::ArgType::Choice,
                true,
                Some(vec!["staging".to_string(), "production".to_string()]),
            ),
            make_arg(
                "region",
                graft_common::ArgType::Choice,
                true,
                Some(vec!["us-east".to_string(), "eu-west".to_string()]),
            ),
        ]),
    )];
    // First arg filled, second still missing
    let cs = compute_run_completions("deploy staging ", &commands);
    assert!(cs.requires_more_input);
    assert_eq!(cs.completions.len(), 2); // region options
    let values: Vec<&str> = cs.completions.iter().map(|c| c.value.as_str()).collect();
    assert!(values.contains(&"us-east"));
}

#[test]
fn run_completions_all_required_filled() {
    let commands = vec![make_command(
        "deploy",
        "Deploy",
        Some(vec![make_arg(
            "env",
            graft_common::ArgType::Choice,
            true,
            Some(vec!["staging".to_string(), "production".to_string()]),
        )]),
    )];
    // "deploy staging " — required arg has been filled
    let cs = compute_run_completions("deploy staging ", &commands);
    assert!(!cs.requires_more_input);
}

// ===== Integration test: requires_more_input blocks Enter =====

#[test]
fn enter_blocked_when_required_arg_missing() {
    let mut prompt = super::prompt::PromptState::new();
    prompt.open();

    // Type "run deploy " — deploy has a required Choice arg
    for c in "run deploy ".chars() {
        prompt.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }

    // Build a CompletionState that mimics what compute_completions would return
    let cs = CompletionState {
        completions: vec![
            ArgCompletion {
                value: "staging".to_string(),
                description: String::new(),
            },
            ArgCompletion {
                value: "production".to_string(),
                description: String::new(),
            },
        ],
        requires_more_input: true,
        arg_hint: None,
    };

    // Press Enter — should fill selected completion, not submit
    let result = prompt.handle_key(KeyCode::Enter, KeyModifiers::NONE, &cs);
    assert!(
        result.is_none(),
        "Enter should not submit when required arg is missing"
    );
    assert!(prompt.is_active(), "Prompt should stay active");
    // Buffer should have the completion filled with a trailing space
    let buffer = &prompt.command_line.as_ref().unwrap().text.buffer;
    assert_eq!(buffer, "run deploy staging ");
}

// ===== PickerState tests =====

fn make_picker_items() -> Vec<PickerItem> {
    vec![
        PickerItem {
            label: "alpha".to_string(),
            description: "First item".to_string(),
            action: CliCommand::Help,
        },
        PickerItem {
            label: "beta".to_string(),
            description: "Second item".to_string(),
            action: CliCommand::Refresh,
        },
        PickerItem {
            label: "gamma".to_string(),
            description: "Third item".to_string(),
            action: CliCommand::Repos,
        },
    ]
}

#[test]
fn picker_filter_narrows_items() {
    let mut picker = PickerState::new(make_picker_items());

    // No filter — all items visible
    assert_eq!(picker.filtered_items().len(), 3);

    // Filter "al" matches only "alpha"
    picker.filter = "al".to_string();
    let filtered = picker.filtered_items();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].label, "alpha");

    // Case-insensitive: "AL" still matches "alpha"
    picker.filter = "AL".to_string();
    assert_eq!(picker.filtered_items().len(), 1);

    // Filter with no match
    picker.filter = "zzz".to_string();
    assert_eq!(picker.filtered_items().len(), 0);
}

#[test]
fn picker_navigation_wraps_around() {
    let mut picker = PickerState::new(make_picker_items());
    assert_eq!(picker.selected, 0);

    // j moves down
    picker.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(picker.selected, 1);

    picker.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(picker.selected, 2);

    // j at last item wraps to 0
    picker.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(picker.selected, 0);

    // k at first item wraps to last
    picker.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
    assert_eq!(picker.selected, 2);

    // k moves up
    picker.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
    assert_eq!(picker.selected, 1);
}

#[test]
fn picker_enter_returns_action() {
    let mut picker = PickerState::new(make_picker_items());

    // Enter on first item returns Help
    let outcome = picker.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(outcome, PickerOutcome::Select(CliCommand::Help));

    // Move to second item, Enter returns Refresh
    picker.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    let outcome = picker.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(outcome, PickerOutcome::Select(CliCommand::Refresh));
}

#[test]
fn picker_esc_returns_dismiss() {
    let mut picker = PickerState::new(make_picker_items());
    let outcome = picker.handle_key(KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(outcome, PickerOutcome::Dismiss);
}

#[test]
fn picker_typing_filters_and_resets_selection() {
    let mut picker = PickerState::new(make_picker_items());
    picker.handle_key(KeyCode::Char('j'), KeyModifiers::NONE); // selected = 1

    // Typing resets selection to 0; 'l' only appears in "alpha"
    picker.handle_key(KeyCode::Char('l'), KeyModifiers::NONE);
    assert_eq!(picker.selected, 0);
    assert_eq!(picker.filter, "l");
    assert_eq!(picker.filtered_items().len(), 1);
    assert_eq!(picker.filtered_items()[0].label, "alpha");

    // Backspace removes last char, restoring all 3 items
    picker.handle_key(KeyCode::Backspace, KeyModifiers::NONE);
    assert_eq!(picker.filter, "");
    assert_eq!(picker.filtered_items().len(), 3);
}

#[test]
fn picker_enter_on_empty_filter_returns_nothing() {
    let mut picker = PickerState::new(vec![]);
    let outcome = picker.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(outcome, PickerOutcome::Nothing);
}

// ===== Actionable table / picker integration tests =====

/// Build a table `ContentBlock` with per-row actions.
fn make_actionable_table() -> ContentBlock {
    use ratatui::text::Span;
    ContentBlock::Table {
        id: BlockId::new(),
        title: "Test Table".to_string(),
        headers: vec!["Name".to_string(), "Desc".to_string()],
        rows: vec![
            vec![Span::raw("alpha"), Span::raw("First")],
            vec![Span::raw("beta"), Span::raw("Second")],
        ],
        collapsed: false,
        actions: Some(vec![CliCommand::Repos, CliCommand::Help]),
    }
}

#[test]
fn enter_on_actionable_table_opens_picker() {
    let mut app = create_app(MockRegistry::with_repos(1));

    // Push an actionable table and focus it
    app.scroll.push(make_actionable_table());
    // Focus the actionable table (last block)
    let last_idx = app.scroll.blocks.len() - 1;
    app.scroll.focused_block = Some(last_idx);

    // Picker should be None before Enter
    assert!(app.picker.is_none());

    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    // Picker should now be open with 2 items (one per row)
    assert!(app.picker.is_some());
    let picker = app.picker.as_ref().unwrap();
    assert_eq!(picker.items.len(), 2);
    assert_eq!(picker.items[0].label, "alpha");
    assert_eq!(picker.items[1].label, "beta");
}

#[test]
fn enter_on_non_actionable_block_toggles_collapse() {
    let mut app = create_app(MockRegistry::with_repos(1));

    // Push a plain text block and focus it
    app.scroll.push(ContentBlock::Text {
        id: BlockId::new(),
        lines: vec![ratatui::text::Line::from("hello")],
        collapsed: false,
    });
    let last_idx = app.scroll.blocks.len() - 1;
    app.scroll.focused_block = Some(last_idx);

    // Enter should toggle collapse, not open picker
    assert!(app.picker.is_none());
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    assert!(app.picker.is_none());

    // Block should now be collapsed
    assert!(app.scroll.blocks[last_idx].is_collapsed());
}

#[test]
fn c_key_toggles_collapse_on_focused_block() {
    let mut app = create_app(MockRegistry::with_repos(1));

    // Push a table (with or without actions) and focus it
    app.scroll.push(make_actionable_table());
    let last_idx = app.scroll.blocks.len() - 1;
    app.scroll.focused_block = Some(last_idx);

    assert!(!app.scroll.blocks[last_idx].is_collapsed());
    app.handle_key(KeyCode::Char('c'), KeyModifiers::NONE);
    assert!(app.scroll.blocks[last_idx].is_collapsed());

    // Press c again to expand
    app.handle_key(KeyCode::Char('c'), KeyModifiers::NONE);
    assert!(!app.scroll.blocks[last_idx].is_collapsed());
}

#[test]
fn picker_esc_dismisses_without_side_effects() {
    let mut app = create_app(MockRegistry::with_repos(1));

    // Open picker via Enter on actionable table
    app.scroll.push(make_actionable_table());
    let last_idx = app.scroll.blocks.len() - 1;
    app.scroll.focused_block = Some(last_idx);
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    assert!(app.picker.is_some());

    // Esc should dismiss picker
    let blocks_before = app.scroll.blocks.len();
    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
    assert!(app.picker.is_none());
    // No side effects: same number of blocks
    assert_eq!(app.scroll.blocks.len(), blocks_before);
}

#[test]
fn picker_selection_executes_command() {
    let mut app = create_app(MockRegistry::with_repos(3));

    // Push a table whose first action is CliCommand::Repos (produces a table block)
    app.scroll.push(make_actionable_table());
    let last_idx = app.scroll.blocks.len() - 1;
    app.scroll.focused_block = Some(last_idx);

    // Open picker
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    assert!(app.picker.is_some());

    let blocks_before = app.scroll.blocks.len();

    // Enter in picker selects first item (Repos command)
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    // Picker should be dismissed
    assert!(app.picker.is_none());
    // Repos command should have pushed a new table block
    assert!(app.scroll.blocks.len() > blocks_before);
}

#[test]
fn focused_block_actions_returns_none_for_non_actionable() {
    let mut buf = ScrollBuffer::new();
    buf.push(ContentBlock::Text {
        id: BlockId::new(),
        lines: vec![ratatui::text::Line::from("text")],
        collapsed: false,
    });
    buf.focused_block = Some(0);
    assert!(buf.focused_block_actions().is_none());
}

#[test]
fn focused_block_actions_returns_actions_for_actionable_table() {
    let mut buf = ScrollBuffer::new();
    buf.push(make_actionable_table());
    buf.focused_block = Some(0);
    let actions = buf.focused_block_actions();
    assert!(actions.is_some());
    assert_eq!(actions.unwrap().len(), 2);
}

#[test]
fn help_output_documents_c_binding() {
    let mut app = create_app(MockRegistry::with_repos(1));
    let initial_blocks = app.scroll.blocks.len();

    type_command(&mut app, "help");

    assert!(app.scroll.blocks.len() > initial_blocks);
    // Find the help block and check for 'c' binding
    let help_block = app.scroll.blocks.last().unwrap();
    if let ContentBlock::Text { lines, .. } = help_block {
        let all_text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            all_text.contains('c'),
            "Help output should mention 'c' keybinding"
        );
        assert!(
            all_text.to_lowercase().contains("collapse"),
            "Help output should mention 'collapse'"
        );
    } else {
        panic!("Expected Text block from :help");
    }
}

// ===== Actionable table integration tests =====

#[test]
fn repos_table_has_actionable_rows() {
    let mut app = create_app(MockRegistry::with_repos(3));
    type_command(&mut app, "repos");

    let last = app.scroll.blocks.last().unwrap();
    if let ContentBlock::Table { actions, rows, .. } = last {
        let acts = actions.as_ref().expect("repos table should have actions");
        assert_eq!(acts.len(), rows.len());
        assert_eq!(acts.len(), 3);
        // Each action should be Repo(<basename>)
        assert_eq!(acts[0], CliCommand::Repo("repo0".to_string()));
        assert_eq!(acts[1], CliCommand::Repo("repo1".to_string()));
        assert_eq!(acts[2], CliCommand::Repo("repo2".to_string()));
    } else {
        panic!("Expected Table block from :repos");
    }
}

#[test]
fn repos_enter_opens_picker_with_repo_names() {
    let mut app = create_app(MockRegistry::with_repos(3));
    type_command(&mut app, "repos");

    // Focus the repos table (last block)
    let last_idx = app.scroll.blocks.len() - 1;
    app.scroll.focused_block = Some(last_idx);

    assert!(app.picker.is_none());
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert!(app.picker.is_some());
    let picker = app.picker.as_ref().unwrap();
    assert_eq!(picker.items.len(), 3);
    // Labels come from first column: Repository basename
    assert_eq!(picker.items[0].label, "repo0");
    assert_eq!(picker.items[1].label, "repo1");
    assert_eq!(picker.items[2].label, "repo2");
}

#[test]
fn repos_picker_selection_switches_repo() {
    let mut app = create_app(MockRegistry::with_repos(3));
    // App starts at repo0 (index 0)
    assert_eq!(app.context.selected_index, Some(0));

    type_command(&mut app, "repos");
    let last_idx = app.scroll.blocks.len() - 1;
    app.scroll.focused_block = Some(last_idx);

    // Open picker
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    assert!(app.picker.is_some());

    // Navigate to repo1 (press j)
    app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(app.picker.as_ref().unwrap().selected, 1);

    // Select repo1
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    // Picker should be dismissed
    assert!(app.picker.is_none());
    // Context should have switched to repo1
    assert_eq!(app.context.selected_index, Some(1));
    assert!(app
        .context
        .selected_repo_path
        .as_deref()
        .unwrap_or("")
        .contains("repo1"));
}

#[test]
fn catalog_table_has_actionable_rows() {
    let mut app = create_app(MockRegistry::with_repos(1));

    // Pre-populate available_commands so catalog produces a table
    app.context.available_commands = vec![
        (
            "build".to_string(),
            graft_common::CommandDef {
                run: "cargo build".to_string(),
                description: Some("Build the project".to_string()),
                category: Some("core".to_string()),
                example: None,
                working_dir: None,
                env: None,
                args: None,
                stdin: None,
                context: None,
                writes: vec![],
                reads: vec![],
            },
        ),
        (
            "test".to_string(),
            graft_common::CommandDef {
                run: "cargo test".to_string(),
                description: Some("Run tests".to_string()),
                category: Some("core".to_string()),
                example: None,
                working_dir: None,
                env: None,
                args: None,
                stdin: None,
                context: None,
                writes: vec![],
                reads: vec![],
            },
        ),
    ];

    type_command(&mut app, "catalog");

    let last = app.scroll.blocks.last().unwrap();
    if let ContentBlock::Table { actions, rows, .. } = last {
        let acts = actions.as_ref().expect("catalog table should have actions");
        assert_eq!(acts.len(), rows.len());
        assert_eq!(acts.len(), 2);
        // Actions should be Run(name, [])
        assert_eq!(acts[0], CliCommand::Run("build".to_string(), vec![]));
        assert_eq!(acts[1], CliCommand::Run("test".to_string(), vec![]));
    } else {
        panic!("Expected Table block from :catalog");
    }
}

#[test]
fn catalog_enter_opens_picker() {
    let mut app = create_app(MockRegistry::with_repos(1));
    app.context.available_commands = vec![(
        "deploy".to_string(),
        graft_common::CommandDef {
            run: "make deploy".to_string(),
            description: Some("Deploy".to_string()),
            category: None,
            example: None,
            working_dir: None,
            env: None,
            args: None,
            stdin: None,
            context: None,
            writes: vec![],
            reads: vec![],
        },
    )];

    type_command(&mut app, "catalog");
    let last_idx = app.scroll.blocks.len() - 1;
    app.scroll.focused_block = Some(last_idx);

    assert!(app.picker.is_none());
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    assert!(app.picker.is_some());

    let picker = app.picker.as_ref().unwrap();
    assert_eq!(picker.items.len(), 1);
    assert_eq!(picker.items[0].label, "deploy");
    assert_eq!(
        picker.items[0].action,
        CliCommand::Run("deploy".to_string(), vec![])
    );
}

#[test]
fn state_table_has_actionable_rows() {
    let mut app = create_app(MockRegistry::with_repos(1));

    // Pre-populate cached_state_queries
    app.context.cached_state_queries = Some(vec![
        crate::state::StateQuery {
            name: "coverage".to_string(),
            run: "cargo tarpaulin".to_string(),
            description: Some("Test coverage".to_string()),
            inputs: Some(vec!["**/*.rs".to_string()]),
            timeout: None,
            working_dir: std::path::PathBuf::from("/tmp/repo0"),
            entity: None,
        },
        crate::state::StateQuery {
            name: "deps".to_string(),
            run: "cargo tree".to_string(),
            description: None,
            inputs: None,
            timeout: None,
            working_dir: std::path::PathBuf::from("/tmp/repo0"),
            entity: None,
        },
    ]);

    type_command(&mut app, "state");

    let last = app.scroll.blocks.last().unwrap();
    if let ContentBlock::Table { actions, rows, .. } = last {
        let acts = actions.as_ref().expect("state table should have actions");
        assert_eq!(acts.len(), rows.len());
        assert_eq!(acts.len(), 2);
        assert_eq!(acts[0], CliCommand::State(Some("coverage".to_string())));
        assert_eq!(acts[1], CliCommand::State(Some("deps".to_string())));
    } else {
        panic!("Expected Table block from :state");
    }
}

#[test]
fn state_enter_opens_picker_with_query_names() {
    let mut app = create_app(MockRegistry::with_repos(1));
    app.context.cached_state_queries = Some(vec![crate::state::StateQuery {
        name: "metrics".to_string(),
        run: "compute-metrics".to_string(),
        description: None,
        inputs: None,
        timeout: None,
        working_dir: std::path::PathBuf::from("/tmp/repo0"),
        entity: None,
    }]);

    type_command(&mut app, "state");
    let last_idx = app.scroll.blocks.len() - 1;
    app.scroll.focused_block = Some(last_idx);

    assert!(app.picker.is_none());
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    assert!(app.picker.is_some());

    let picker = app.picker.as_ref().unwrap();
    assert_eq!(picker.items.len(), 1);
    assert_eq!(picker.items[0].label, "metrics");
    assert_eq!(
        picker.items[0].action,
        CliCommand::State(Some("metrics".to_string()))
    );
}

// ===== Focus / Unfocus command parsing =====

#[test]
fn parse_focus_no_args() {
    assert_eq!(parse_command("focus"), CliCommand::Focus(None, None));
    assert_eq!(parse_command("f"), CliCommand::Focus(None, None));
}

#[test]
fn parse_focus_query_only() {
    assert_eq!(
        parse_command("focus environments"),
        CliCommand::Focus(Some("environments".to_string()), None)
    );
    assert_eq!(
        parse_command("f slices"),
        CliCommand::Focus(Some("slices".to_string()), None)
    );
}

#[test]
fn parse_focus_query_and_value() {
    assert_eq!(
        parse_command("focus environments staging"),
        CliCommand::Focus(
            Some("environments".to_string()),
            Some("staging".to_string())
        )
    );
    assert_eq!(
        parse_command("f slices retry-logic"),
        CliCommand::Focus(Some("slices".to_string()), Some("retry-logic".to_string()))
    );
}

#[test]
fn parse_unfocus_no_args() {
    assert_eq!(parse_command("unfocus"), CliCommand::Unfocus(None));
    assert_eq!(parse_command("uf"), CliCommand::Unfocus(None));
}

#[test]
fn parse_unfocus_with_query() {
    assert_eq!(
        parse_command("unfocus environments"),
        CliCommand::Unfocus(Some("environments".to_string()))
    );
    assert_eq!(
        parse_command("uf slices"),
        CliCommand::Unfocus(Some("slices".to_string()))
    );
}

// ===== Focus / Unfocus command integration =====

#[test]
fn focus_direct_set_stores_value() {
    let mut app = create_app(MockRegistry::with_repos(1));
    assert!(app.focus.is_empty());

    // :focus slices retry-logic sets focus directly
    type_command(&mut app, "focus slices retry-logic");

    assert_eq!(
        app.focus.get("slices").map(String::as_str),
        Some("retry-logic")
    );
    assert!(app.status.is_some());
}

#[test]
fn focus_direct_set_overwrites_previous_value() {
    let mut app = create_app(MockRegistry::with_repos(1));

    type_command(&mut app, "focus slices old-value");
    assert_eq!(
        app.focus.get("slices").map(String::as_str),
        Some("old-value")
    );

    type_command(&mut app, "focus slices new-value");
    assert_eq!(
        app.focus.get("slices").map(String::as_str),
        Some("new-value")
    );
}

#[test]
fn focus_multiple_queries_independent() {
    let mut app = create_app(MockRegistry::with_repos(1));

    type_command(&mut app, "focus slices retry-logic");
    type_command(&mut app, "focus environments staging");

    assert_eq!(
        app.focus.get("slices").map(String::as_str),
        Some("retry-logic")
    );
    assert_eq!(
        app.focus.get("environments").map(String::as_str),
        Some("staging")
    );
}

#[test]
fn unfocus_single_query_clears_it() {
    let mut app = create_app(MockRegistry::with_repos(1));

    type_command(&mut app, "focus slices retry-logic");
    type_command(&mut app, "focus environments staging");
    assert_eq!(app.focus.len(), 2);

    type_command(&mut app, "unfocus slices");
    assert!(!app.focus.contains_key("slices"));
    assert_eq!(
        app.focus.get("environments").map(String::as_str),
        Some("staging")
    );
}

#[test]
fn unfocus_all_clears_everything() {
    let mut app = create_app(MockRegistry::with_repos(1));

    type_command(&mut app, "focus slices retry-logic");
    type_command(&mut app, "focus environments staging");
    assert_eq!(app.focus.len(), 2);

    type_command(&mut app, "unfocus");
    assert!(app.focus.is_empty());
    assert!(app.status.is_some());
}

#[test]
fn unfocus_nonexistent_query_shows_info() {
    let mut app = create_app(MockRegistry::with_repos(1));

    // No focus set — unfocusing a query that doesn't exist should show info
    type_command(&mut app, "unfocus slices");
    assert!(app.status.is_some());
    assert!(app.focus.is_empty());
}

#[test]
fn focus_list_mode_pushes_text_block() {
    let mut app = create_app(MockRegistry::with_repos(1));

    // Pre-populate cached_state_queries so the list shows something
    app.context.cached_state_queries = Some(vec![crate::state::StateQuery {
        name: "slices".to_string(),
        run: "list-slices".to_string(),
        description: None,
        inputs: None,
        timeout: None,
        working_dir: std::path::PathBuf::from("/tmp/repo0"),
        entity: None,
    }]);

    let initial_blocks = app.scroll.blocks.len();
    type_command(&mut app, "focus");

    // A text block should have been pushed
    assert!(app.scroll.blocks.len() > initial_blocks);
}

#[test]
fn palette_commands_include_focus_and_unfocus() {
    assert!(PALETTE_COMMANDS.iter().any(|e| e.command == "focus"));
    assert!(PALETTE_COMMANDS.iter().any(|e| e.command == "unfocus"));
}

// ===== Auto-fill focused args in :run =====

fn make_command_with_options_from(
    name: &str,
    arg_name: &str,
    options_from: &str,
) -> (String, graft_common::CommandDef) {
    (
        name.to_string(),
        graft_common::CommandDef {
            run: format!("echo {{{arg_name}}}"),
            description: None,
            category: None,
            example: None,
            working_dir: None,
            env: None,
            args: Some(vec![graft_common::ArgDef {
                name: arg_name.to_string(),
                arg_type: graft_common::ArgType::Choice,
                description: None,
                required: true,
                default: None,
                options: None,
                options_from: Some(options_from.to_string()),
                positional: true,
            }]),
            stdin: None,
            context: None,
            writes: vec![],
            reads: vec![],
        },
    )
}

#[test]
fn run_auto_fills_from_focus() {
    let mut app = create_app(MockRegistry::with_repos(1));
    // Set up a command with a required arg that uses options_from: slices
    app.context.available_commands =
        vec![make_command_with_options_from("deploy", "slice", "slices")];
    // Set focus for slices
    app.focus
        .insert("slices".to_string(), "retry-logic".to_string());

    // Run the command without providing the arg
    type_command(&mut app, "run deploy");

    // Focus should have caused execution to start (Running block pushed)
    assert_eq!(
        app.execution.command_state,
        grove_core::CommandState::Running,
        "Expected command to start running after focus auto-fill"
    );
    // Status should report which focused value was used
    let status_dbg = format!("{:?}", app.status);
    assert!(
        status_dbg.contains("slices") && status_dbg.contains("retry-logic"),
        "Expected status to mention 'slices: retry-logic', got: {status_dbg:?}"
    );
}

#[test]
fn run_explicit_arg_overrides_focus() {
    let mut app = create_app(MockRegistry::with_repos(1));
    app.context.available_commands =
        vec![make_command_with_options_from("deploy", "slice", "slices")];
    // Set focus for slices
    app.focus
        .insert("slices".to_string(), "retry-logic".to_string());

    // Provide the arg explicitly — should override focus.
    // Trailing space is required: without it the completion system sees "explicit-slice" as a
    // partial arg and blocks submission (requires_more_input). With it, submission proceeds.
    type_command(&mut app, "run deploy explicit-slice ");

    // Execution should start
    assert_eq!(
        app.execution.command_state,
        grove_core::CommandState::Running
    );
    // Status should NOT report auto-fill (explicit arg was used)
    let status_dbg = format!("{:?}", app.status);
    assert!(
        !status_dbg.contains("Using focused"),
        "Expected no 'Using focused' message when explicit arg provided, got: {status_dbg:?}"
    );
}

#[test]
fn run_no_focus_missing_arg_shows_error() {
    let mut app = create_app(MockRegistry::with_repos(1));
    app.context.available_commands =
        vec![make_command_with_options_from("deploy", "slice", "slices")];
    // No focus set

    type_command(&mut app, "run deploy");

    // Execution should NOT have started
    assert_ne!(
        app.execution.command_state,
        grove_core::CommandState::Running,
        "Expected no execution when required arg is missing and no focus is set"
    );
}

#[test]
fn run_multi_arg_partial_focus_fills_first_stops_at_second() {
    let mut app = create_app(MockRegistry::with_repos(1));
    // Command with two required args: slice (options_from: slices) and env (options_from: environments)
    app.context.available_commands = vec![(
        "deploy".to_string(),
        graft_common::CommandDef {
            run: "echo {slice} {env}".to_string(),
            description: None,
            category: None,
            example: None,
            working_dir: None,
            env: None,
            args: Some(vec![
                graft_common::ArgDef {
                    name: "slice".to_string(),
                    arg_type: graft_common::ArgType::Choice,
                    description: None,
                    required: true,
                    default: None,
                    options: None,
                    options_from: Some("slices".to_string()),
                    positional: true,
                },
                graft_common::ArgDef {
                    name: "env".to_string(),
                    arg_type: graft_common::ArgType::Choice,
                    description: None,
                    required: true,
                    default: None,
                    options: None,
                    options_from: Some("environments".to_string()),
                    positional: true,
                },
            ]),
            stdin: None,
            context: None,
            writes: vec![],
            reads: vec![],
        },
    )];
    // Only focus slices, not environments
    app.focus
        .insert("slices".to_string(), "retry-logic".to_string());

    // Run without any args
    type_command(&mut app, "run deploy");

    // First arg filled from focus, second still missing → execution should NOT start
    assert_ne!(
        app.execution.command_state,
        grove_core::CommandState::Running,
        "Expected no execution when second required arg is still missing"
    );
    // But status should show that the first was auto-filled before we hit the error
    let status_dbg = format!("{:?}", app.status);
    assert!(
        status_dbg.contains("slices") && status_dbg.contains("retry-logic"),
        "Expected status to mention partial auto-fill, got: {status_dbg:?}"
    );
}

// ===== Header focus display =====

#[test]
fn compute_stale_focus_empty_when_no_in_memory_data() {
    let mut app = create_app(MockRegistry::with_repos(1));
    app.focus
        .insert("slices".to_string(), "retry-logic".to_string());
    // No in-memory state → stale check is skipped (opportunistic)
    let stale = app.compute_stale_focus();
    assert!(stale.is_empty(), "No in-memory data → not stale");
}

#[test]
fn compute_stale_focus_empty_when_value_still_present() {
    let mut app = create_app(MockRegistry::with_repos(1));
    app.focus
        .insert("slices".to_string(), "retry-logic".to_string());
    // Populate in-memory state with data that includes the focused value
    app.context.in_memory_state.insert(
        "slices".to_string(),
        serde_json::json!({"slices": [{"name": "retry-logic"}, {"name": "other"}]}),
    );
    let stale = app.compute_stale_focus();
    assert!(stale.is_empty(), "Value still present → not stale");
}

#[test]
fn compute_stale_focus_detects_missing_value() {
    let mut app = create_app(MockRegistry::with_repos(1));
    app.focus
        .insert("slices".to_string(), "old-slice".to_string());
    // In-memory state no longer contains "old-slice"
    app.context.in_memory_state.insert(
        "slices".to_string(),
        serde_json::json!({"slices": [{"name": "new-slice"}]}),
    );
    let stale = app.compute_stale_focus();
    assert!(stale.contains("slices"), "Value missing → stale");
}

#[test]
fn compute_stale_focus_multiple_queries_independent() {
    let mut app = create_app(MockRegistry::with_repos(1));
    app.focus
        .insert("slices".to_string(), "present".to_string());
    app.focus
        .insert("environments".to_string(), "gone".to_string());

    // slices: "present" is in-memory → not stale
    app.context.in_memory_state.insert(
        "slices".to_string(),
        serde_json::json!({"slices": ["present", "other"]}),
    );
    // environments: "gone" is NOT in-memory results
    app.context.in_memory_state.insert(
        "environments".to_string(),
        serde_json::json!({"environments": [{"name": "staging"}]}),
    );

    let stale = app.compute_stale_focus();
    assert!(
        !stale.contains("slices"),
        "slices value still present → not stale"
    );
    assert!(
        stale.contains("environments"),
        "environments value missing → stale"
    );
}

// ===== Focus completions =====

#[test]
fn completions_focus_first_arg_shows_query_names() {
    let queries = vec!["slices".to_string(), "environments".to_string()];
    let mut p = super::prompt::PromptState::new();
    p.open();
    for c in "focus ".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let cs = p.compute_completions(&[], &[], &queries, &HashMap::default(), &[]);
    assert_eq!(cs.completions.len(), 2);
    let values: Vec<&str> = cs.completions.iter().map(|c| c.value.as_str()).collect();
    assert!(values.contains(&"slices"));
    assert!(values.contains(&"environments"));
}

#[test]
fn completions_focus_first_arg_partial_match() {
    let queries = vec!["slices".to_string(), "environments".to_string()];
    let mut p = super::prompt::PromptState::new();
    p.open();
    for c in "focus sl".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let cs = p.compute_completions(&[], &[], &queries, &HashMap::default(), &[]);
    assert_eq!(cs.completions.len(), 1);
    assert_eq!(cs.completions[0].value, "slices");
}

#[test]
fn completions_focus_second_arg_shows_entity_values() {
    let queries = vec!["slices".to_string()];
    let mut focus_opts = std::collections::HashMap::new();
    focus_opts.insert(
        "slices".to_string(),
        vec!["retry-logic".to_string(), "entity-focus".to_string()],
    );

    let mut p = super::prompt::PromptState::new();
    p.open();
    for c in "focus slices ".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let cs = p.compute_completions(&[], &[], &queries, &focus_opts, &[]);
    assert_eq!(cs.completions.len(), 2);
    let values: Vec<&str> = cs.completions.iter().map(|c| c.value.as_str()).collect();
    assert!(values.contains(&"retry-logic"));
    assert!(values.contains(&"entity-focus"));
}

#[test]
fn completions_focus_second_arg_partial_match() {
    let queries = vec!["slices".to_string()];
    let mut focus_opts = std::collections::HashMap::new();
    focus_opts.insert(
        "slices".to_string(),
        vec!["retry-logic".to_string(), "entity-focus".to_string()],
    );

    let mut p = super::prompt::PromptState::new();
    p.open();
    for c in "focus slices retry".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let cs = p.compute_completions(&[], &[], &queries, &focus_opts, &[]);
    assert_eq!(cs.completions.len(), 1);
    assert_eq!(cs.completions[0].value, "retry-logic");
}

#[test]
fn completions_focus_alias_f_works() {
    let queries = vec!["slices".to_string(), "environments".to_string()];
    let mut p = super::prompt::PromptState::new();
    p.open();
    for c in "f ".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let cs = p.compute_completions(&[], &[], &queries, &HashMap::default(), &[]);
    assert_eq!(cs.completions.len(), 2);
}

// ===== scion completion tests =====

#[test]
fn completions_scion_subcommands() {
    let mut p = PromptState::new();
    p.open();
    for c in "scion ".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let cs = p.compute_completions(&[], &[], &[], &HashMap::default(), &[]);
    let values: Vec<&str> = cs.completions.iter().map(|c| c.value.as_str()).collect();
    assert!(values.contains(&"list"));
    assert!(values.contains(&"create"));
    assert!(values.contains(&"start"));
    assert!(values.contains(&"stop"));
    assert!(values.contains(&"prune"));
    assert!(values.contains(&"fuse"));
    assert_eq!(cs.completions.len(), 6);
    assert!(cs.requires_more_input);
}

#[test]
fn completions_scion_subcommand_partial() {
    let mut p = PromptState::new();
    p.open();
    for c in "scion st".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let cs = p.compute_completions(&[], &[], &[], &HashMap::default(), &[]);
    let values: Vec<&str> = cs.completions.iter().map(|c| c.value.as_str()).collect();
    assert!(values.contains(&"start"));
    assert!(values.contains(&"stop"));
    assert_eq!(cs.completions.len(), 2);
}

#[test]
fn completions_scion_start_shows_scion_names() {
    let mut p = PromptState::new();
    p.open();
    for c in "scion start ".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let scions = vec![
        ArgCompletion {
            value: "my-feature".to_string(),
            description: "+3".to_string(),
        },
        ArgCompletion {
            value: "bugfix".to_string(),
            description: "+1 [session]".to_string(),
        },
    ];
    let cs = p.compute_completions(&[], &[], &[], &HashMap::default(), &scions);
    assert_eq!(cs.completions.len(), 2);
    let values: Vec<&str> = cs.completions.iter().map(|c| c.value.as_str()).collect();
    assert!(values.contains(&"my-feature"));
    assert!(values.contains(&"bugfix"));
}

#[test]
fn completions_scion_start_partial_filters() {
    let mut p = PromptState::new();
    p.open();
    for c in "scion start my".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let scions = vec![
        ArgCompletion {
            value: "my-feature".to_string(),
            description: "+3".to_string(),
        },
        ArgCompletion {
            value: "bugfix".to_string(),
            description: String::new(),
        },
    ];
    let cs = p.compute_completions(&[], &[], &[], &HashMap::default(), &scions);
    assert_eq!(cs.completions.len(), 1);
    assert_eq!(cs.completions[0].value, "my-feature");
}

#[test]
fn completions_attach_shows_scion_names() {
    let mut p = PromptState::new();
    p.open();
    for c in "attach ".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let scions = vec![ArgCompletion {
        value: "dev".to_string(),
        description: "+2 [session]".to_string(),
    }];
    let cs = p.compute_completions(&[], &[], &[], &HashMap::default(), &scions);
    assert_eq!(cs.completions.len(), 1);
    assert_eq!(cs.completions[0].value, "dev");
}

#[test]
fn completions_review_shows_scion_names() {
    let mut p = PromptState::new();
    p.open();
    for c in "review ".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let scions = vec![
        ArgCompletion {
            value: "alpha".to_string(),
            description: String::new(),
        },
        ArgCompletion {
            value: "beta".to_string(),
            description: String::new(),
        },
    ];
    let cs = p.compute_completions(&[], &[], &[], &HashMap::default(), &scions);
    assert_eq!(cs.completions.len(), 2);
}

#[test]
fn completions_scion_create_requires_name() {
    let mut p = PromptState::new();
    p.open();
    for c in "scion create ".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let cs = p.compute_completions(&[], &[], &[], &HashMap::default(), &[]);
    assert!(cs.completions.is_empty());
    assert!(cs.requires_more_input);
    assert_eq!(cs.arg_hint, Some("<name>".to_string()));
}

#[test]
fn completions_sc_alias_shows_subcommands() {
    let mut p = PromptState::new();
    p.open();
    for c in "sc ".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let cs = p.compute_completions(&[], &[], &[], &HashMap::default(), &[]);
    assert_eq!(cs.completions.len(), 6);
    assert!(cs.requires_more_input);
}

#[test]
fn completions_scion_stop_shows_scion_names() {
    let mut p = PromptState::new();
    p.open();
    for c in "scion stop ".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let scions = vec![ArgCompletion {
        value: "my-feature".to_string(),
        description: "+1 [session]".to_string(),
    }];
    let cs = p.compute_completions(&[], &[], &[], &HashMap::default(), &scions);
    assert_eq!(cs.completions.len(), 1);
    assert_eq!(cs.completions[0].value, "my-feature");
}

#[test]
fn completions_scion_prune_shows_scion_names() {
    let mut p = PromptState::new();
    p.open();
    for c in "scion prune ".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let scions = vec![ArgCompletion {
        value: "old-branch".to_string(),
        description: String::new(),
    }];
    let cs = p.compute_completions(&[], &[], &[], &HashMap::default(), &scions);
    assert_eq!(cs.completions.len(), 1);
    assert_eq!(cs.completions[0].value, "old-branch");
}

#[test]
fn completions_scion_fuse_shows_scion_names() {
    let mut p = PromptState::new();
    p.open();
    for c in "scion fuse ".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let scions = vec![ArgCompletion {
        value: "ready-branch".to_string(),
        description: "+5".to_string(),
    }];
    let cs = p.compute_completions(&[], &[], &[], &HashMap::default(), &scions);
    assert_eq!(cs.completions.len(), 1);
    assert_eq!(cs.completions[0].value, "ready-branch");
}

#[test]
fn completions_scion_list_no_extra_input_needed() {
    // :scion list should not require more input (no args needed)
    let mut p = PromptState::new();
    p.open();
    for c in "scion l".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let cs = p.compute_completions(&[], &[], &[], &HashMap::default(), &[]);
    // "list" is the only match for "l", so requires_more_input should be false
    assert_eq!(cs.completions.len(), 1);
    assert_eq!(cs.completions[0].value, "list");
    assert!(
        !cs.requires_more_input,
        "list takes no args, should submit on Enter"
    );
}

#[test]
fn completions_scion_bogus_subcommand_no_completions() {
    let mut p = PromptState::new();
    p.open();
    for c in "scion zzz".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let cs = p.compute_completions(&[], &[], &[], &HashMap::default(), &[]);
    assert!(cs.completions.is_empty(), "no subcommand matches 'zzz'");
}

#[test]
fn completions_scion_create_with_name_submittable() {
    // :scion create my-feature should be submittable (has a name)
    let mut p = PromptState::new();
    p.open();
    for c in "scion create my-feature".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let cs = p.compute_completions(&[], &[], &[], &HashMap::default(), &[]);
    assert!(
        !cs.requires_more_input,
        "name provided, should be submittable"
    );
}

#[test]
fn completions_scion_start_no_trailing_space_completes_subcommand() {
    // :scion start (no trailing space) should complete the subcommand, not show names
    let mut p = PromptState::new();
    p.open();
    for c in "scion st".chars() {
        p.handle_key(
            KeyCode::Char(c),
            KeyModifiers::NONE,
            &CompletionState::default(),
        );
    }
    let scions = vec![ArgCompletion {
        value: "my-feature".to_string(),
        description: String::new(),
    }];
    let cs = p.compute_completions(&[], &[], &[], &HashMap::default(), &scions);
    // Should show "start" and "stop" subcommands, not scion names
    let values: Vec<&str> = cs.completions.iter().map(|c| c.value.as_str()).collect();
    assert!(values.contains(&"start"), "should show start subcommand");
    assert!(values.contains(&"stop"), "should show stop subcommand");
    assert!(
        !values.contains(&"my-feature"),
        "should not show scion names without trailing space"
    );
}

/// Build a scion table block matching the shape that `cmd_scion_list` produces.
///
/// Note: we construct the block directly rather than calling `cmd_scion_list`
/// because that function requires a real git repo with worktrees. The column
/// layout, styling, and action wiring tested here mirror the implementation in
/// `transcript.rs` — if the implementation changes, this helper must be updated
/// to match.
fn build_scion_table_block(
    scions: &[(
        &str,         // name
        Option<u32>,  // ahead
        Option<u32>,  // behind
        bool,         // dirty
        Option<bool>, // session_active
    )],
) -> ContentBlock {
    use ratatui::style::{Color, Style};
    use ratatui::text::Span;

    let headers = vec![
        "Name".to_string(),
        "Ahead/Behind".to_string(),
        "Dirty".to_string(),
        "Session".to_string(),
    ];
    let mut rows = Vec::new();
    let mut actions = Vec::new();
    for &(name, ahead, behind, dirty, session_active) in scions {
        let ahead_str = ahead.map_or("?".to_string(), |a| a.to_string());
        let behind_str = behind.map_or("?".to_string(), |b| b.to_string());
        let dirty_span = if dirty {
            Span::styled("\u{25cf}", Style::default().fg(Color::Yellow))
        } else {
            Span::styled("\u{25cb}", Style::default().fg(Color::Green))
        };
        let session_span = match session_active {
            Some(true) => Span::styled("\u{25cf} active", Style::default().fg(Color::Green)),
            Some(false) => Span::styled("\u{2013}", Style::default().fg(Color::DarkGray)),
            None => Span::styled("?", Style::default().fg(Color::DarkGray)),
        };

        let mut summary_parts = Vec::new();
        summary_parts.push(format!("\u{2191}{ahead_str} \u{2193}{behind_str}"));
        if dirty {
            summary_parts.push("dirty".to_string());
        }
        if session_active == Some(true) {
            summary_parts.push("active".to_string());
        }
        let summary = summary_parts.join(", ");

        actions.push(CliCommand::Review(name.to_string(), false));
        rows.push(vec![
            Span::styled(name.to_string(), Style::default().fg(Color::Cyan)),
            Span::styled(summary, Style::default().fg(Color::Yellow)),
            dirty_span,
            session_span,
        ]);
    }

    ContentBlock::Table {
        id: BlockId::new(),
        title: "Scions".to_string(),
        headers,
        rows,
        collapsed: false,
        actions: Some(actions),
    }
}

#[test]
fn scion_list_table_structure_and_actions() {
    let block = build_scion_table_block(&[("my-feature", Some(3), Some(0), false, Some(false))]);

    // Verify structure
    if let ContentBlock::Table {
        title,
        headers: h,
        rows: r,
        actions: a,
        ..
    } = &block
    {
        assert_eq!(title, "Scions");
        assert_eq!(h.len(), 4);
        assert_eq!(h[0], "Name");
        assert_eq!(h[1], "Ahead/Behind");
        assert_eq!(h[2], "Dirty");
        assert_eq!(h[3], "Session");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0][0].content, "my-feature");
        // Column 1 is the picker description: human-readable summary
        assert_eq!(r[0][1].content, "\u{2191}3 \u{2193}0");
        // Clean, inactive session
        assert_eq!(r[0][2].content, "\u{25cb}");
        assert_eq!(r[0][3].content, "\u{2013}");

        let acts = a.as_ref().expect("scion table should have actions");
        assert_eq!(acts.len(), 1);
        assert_eq!(acts[0], CliCommand::Review("my-feature".to_string(), false));
    } else {
        panic!("Expected Table block");
    }

    // Verify picker opens when table is focused and Enter is pressed
    let mut app = create_app(MockRegistry::with_repos(1));
    app.scroll.push(block);
    let last_idx = app.scroll.blocks.len() - 1;
    app.scroll.focused_block = Some(last_idx);

    assert!(app.picker.is_none());
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    assert!(app.picker.is_some());

    let picker = app.picker.as_ref().unwrap();
    assert_eq!(picker.items.len(), 1);
    assert_eq!(picker.items[0].label, "my-feature");
    assert_eq!(
        picker.items[0].action,
        CliCommand::Review("my-feature".to_string(), false)
    );
}

#[test]
fn scion_list_table_multiple_scions() {
    // Multiple scions covering dirty, active session, unknown ahead/behind,
    // and no-tmux (None session) variants.
    let block = build_scion_table_block(&[
        ("clean-ahead", Some(5), Some(0), false, Some(false)),
        ("dirty-active", Some(0), Some(2), true, Some(true)),
        ("unknown-no-tmux", None, None, false, None),
    ]);

    if let ContentBlock::Table { rows, actions, .. } = &block {
        assert_eq!(rows.len(), 3);
        let acts = actions.as_ref().unwrap();
        assert_eq!(acts.len(), 3);

        // Row 0: clean, inactive — summary has only arrows
        assert_eq!(rows[0][0].content, "clean-ahead");
        assert_eq!(rows[0][1].content, "\u{2191}5 \u{2193}0");
        assert_eq!(rows[0][2].content, "\u{25cb}"); // clean
        assert_eq!(rows[0][3].content, "\u{2013}"); // inactive session

        // Row 1: dirty + active — summary includes dirty & active
        assert_eq!(rows[1][0].content, "dirty-active");
        assert_eq!(rows[1][1].content, "\u{2191}0 \u{2193}2, dirty, active");
        assert_eq!(rows[1][2].content, "\u{25cf}"); // dirty
        assert_eq!(rows[1][3].content, "\u{25cf} active"); // active session

        // Row 2: unknown counts, no tmux — summary has ? counts, session shows ?
        assert_eq!(rows[2][0].content, "unknown-no-tmux");
        assert_eq!(rows[2][1].content, "\u{2191}? \u{2193}?");
        assert_eq!(rows[2][2].content, "\u{25cb}"); // clean
        assert_eq!(rows[2][3].content, "?"); // no tmux — distinct from "–"

        // Actions wire to :review for each
        assert_eq!(
            acts[0],
            CliCommand::Review("clean-ahead".to_string(), false)
        );
        assert_eq!(
            acts[1],
            CliCommand::Review("dirty-active".to_string(), false)
        );
        assert_eq!(
            acts[2],
            CliCommand::Review("unknown-no-tmux".to_string(), false)
        );
    } else {
        panic!("Expected Table block");
    }
}

#[test]
fn scion_list_table_picker_shows_descriptions() {
    // When the picker opens, col 0 = label, col 1 = description.
    // Verify the description carries the human-readable summary.
    let block = build_scion_table_block(&[
        ("feat-a", Some(1), Some(0), true, Some(true)),
        ("feat-b", Some(0), Some(0), false, None),
    ]);

    let mut app = create_app(MockRegistry::with_repos(1));
    app.scroll.push(block);
    let last_idx = app.scroll.blocks.len() - 1;
    app.scroll.focused_block = Some(last_idx);

    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    let picker = app.picker.as_ref().expect("picker should open");

    assert_eq!(picker.items.len(), 2);
    assert_eq!(picker.items[0].label, "feat-a");
    assert_eq!(
        picker.items[0].description,
        "\u{2191}1 \u{2193}0, dirty, active"
    );
    assert_eq!(picker.items[1].label, "feat-b");
    assert_eq!(picker.items[1].description, "\u{2191}0 \u{2193}0");
}
