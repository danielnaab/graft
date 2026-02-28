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
    CompletionState,
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
    // Find the first palette entry that doesn't take args
    let first_no_args = PALETTE_COMMANDS.iter().find(|e| !e.takes_args).unwrap();
    let expected = parse_command(first_no_args.command);

    let mut prompt = super::prompt::PromptState::new();
    prompt.open();

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
    let opts = extract_options_from_state("slices", &data);
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
    let opts = extract_options_from_state("slices", &data);
    assert_eq!(opts, vec!["slices/active", "slices/wip"]);
}

#[test]
fn options_from_state_extracts_string_array() {
    let data = serde_json::json!({"tags": ["alpha", "beta", "gamma"]});
    let opts = extract_options_from_state("tags", &data);
    assert_eq!(opts, vec!["alpha", "beta", "gamma"]);
}

#[test]
fn options_from_state_extracts_name_field() {
    let data = serde_json::json!({
        "envs": [{"name": "staging"}, {"name": "production"}]
    });
    let opts = extract_options_from_state("envs", &data);
    assert_eq!(opts, vec!["staging", "production"]);
}

#[test]
fn options_from_state_missing_key_returns_empty() {
    let data = serde_json::json!({"other": ["x", "y"]});
    let opts = extract_options_from_state("slices", &data);
    assert!(opts.is_empty());
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
    let cs = p.compute_completions(&[], &[], &[]);
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
    let cs = p.compute_completions(&commands, &[], &[]);
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
    let cs = p.compute_completions(&commands, &[], &[]);
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
    let cs = p.compute_completions(&[], &repos, &[]);
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
    let cs = p.compute_completions(&[], &repos, &[]);
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
    let cs = p.compute_completions(&[], &[], &queries);
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
    let cs = p.compute_completions(&[], &[], &[]);
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
    let cs = p.compute_completions(&[], &[], &[]);
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
