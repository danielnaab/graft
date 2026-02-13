---
status: planned
date: 2026-02-13
priority: high
effort: 4-6 hours
---

# Grove Argument Input - Phase 1 Improvements

## Goal
Fix critical UX issues in argument input dialog to make editing comfortable for daily use.

## Success Criteria
- ✅ User can move cursor left/right with arrow keys
- ✅ User can see where cursor is positioned in text
- ✅ User sees preview of parsed command before execution
- ✅ User gets immediate feedback on parsing errors
- ✅ User cannot execute command if parsing failed

## Changes

### 1. Add Cursor Position Support (2-3 hours)

#### 1.1 Update State
**File**: `grove/src/tui.rs`

```rust
// Current:
argument_buffer: String,
argument_command_name: Option<String>,

// New:
struct ArgumentInputState {
    buffer: String,
    cursor_pos: usize,  // Character position, not byte position
    command_name: String,
}

// In App struct:
argument_input: Option<ArgumentInputState>,  // None when not in argument input mode
```

**Why struct**: Encapsulates related state, makes it clear when we're in argument input mode

#### 1.2 Update Key Handler
**File**: `grove/src/tui.rs`, function `handle_key_argument_input`

Add cursor movement:
```rust
KeyCode::Left => {
    if self.argument_input.as_ref().unwrap().cursor_pos > 0 {
        self.argument_input.as_mut().unwrap().cursor_pos -= 1;
    }
}
KeyCode::Right => {
    let state = self.argument_input.as_ref().unwrap();
    if state.cursor_pos < state.buffer.chars().count() {
        self.argument_input.as_mut().unwrap().cursor_pos += 1;
    }
}
KeyCode::Home => {
    self.argument_input.as_mut().unwrap().cursor_pos = 0;
}
KeyCode::End => {
    let len = self.argument_input.as_ref().unwrap().buffer.chars().count();
    self.argument_input.as_mut().unwrap().cursor_pos = len;
}
```

Update char insertion:
```rust
KeyCode::Char(c) => {
    let state = self.argument_input.as_mut().unwrap();
    let char_idx = state.cursor_pos;

    // Insert at cursor position (char index, not byte index)
    let mut chars: Vec<char> = state.buffer.chars().collect();
    chars.insert(char_idx, c);
    state.buffer = chars.into_iter().collect();
    state.cursor_pos += 1;
}
```

Update backspace:
```rust
KeyCode::Backspace => {
    let state = self.argument_input.as_mut().unwrap();
    if state.cursor_pos > 0 {
        let mut chars: Vec<char> = state.buffer.chars().collect();
        chars.remove(state.cursor_pos - 1);
        state.buffer = chars.into_iter().collect();
        state.cursor_pos -= 1;
    }
}
```

#### 1.3 Update Render
**File**: `grove/src/tui.rs`, function `render_argument_input_overlay`

Render cursor at actual position:
```rust
let state = self.argument_input.as_ref().unwrap();
let cursor_pos = state.cursor_pos;

// Split buffer at cursor position
let chars: Vec<char> = state.buffer.chars().collect();
let before_cursor: String = chars[..cursor_pos].iter().collect();
let after_cursor: String = chars[cursor_pos..].iter().collect();

// Render with cursor indicator
let input_text = if after_cursor.is_empty() {
    format!("> {}_", before_cursor)  // Cursor at end
} else {
    format!("> {}▊{}", before_cursor, after_cursor)  // Cursor in middle (block char)
};
```

**Test Plan**:
- [ ] Move cursor left/right with arrow keys
- [ ] Cursor stops at boundaries (0 and len)
- [ ] Home/End keys work
- [ ] Char insertion works at cursor position
- [ ] Backspace deletes char before cursor
- [ ] Cursor position updates correctly with each operation
- [ ] Unicode characters (emoji, CJK) handled correctly

---

### 2. Add Command Preview (1-2 hours)

#### 2.1 Update Dialog Layout
**File**: `grove/src/tui.rs`, function `render_argument_input_overlay`

Increase height and add preview line:
```rust
let dialog_height = 9;  // Was 7, now 9 for preview + error

let content = vec![
    Line::from(""),
    Line::from(input_text).style(Style::default().fg(Color::Cyan)),
    Line::from(""),
    Line::from(preview_text).style(preview_style),  // NEW
    Line::from(""),
    Line::from(help).style(Style::default().fg(Color::DarkGray)),
];
```

#### 2.2 Generate Preview Text
```rust
fn format_argument_preview(&self) -> (String, Style) {
    let state = self.argument_input.as_ref().unwrap();

    if state.buffer.is_empty() {
        return (
            format!("Will execute: graft run {}", state.command_name),
            Style::default().fg(Color::DarkGray)
        );
    }

    match shell_words::split(&state.buffer) {
        Ok(args) => {
            // Show parsed arguments with shell quoting for clarity
            let quoted_args: Vec<String> = args.iter()
                .map(|arg| {
                    if arg.contains(' ') {
                        format!("'{}'", arg)
                    } else {
                        arg.clone()
                    }
                })
                .collect();

            (
                format!("Will execute: graft run {} {}",
                    state.command_name,
                    quoted_args.join(" ")
                ),
                Style::default().fg(Color::Green)
            )
        }
        Err(e) => {
            (
                format!("Parse error: {} - fix before running", e),
                Style::default().fg(Color::Red)
            )
        }
    }
}
```

**Test Plan**:
- [ ] Empty input shows basic preview
- [ ] Valid input shows parsed args with quoting
- [ ] Parse error shows red error message
- [ ] Preview updates on every keystroke

---

### 3. Block Execution on Parse Errors (1 hour)

#### 3.1 Validate Before Execution
**File**: `grove/src/tui.rs`, function `handle_key_argument_input`

```rust
KeyCode::Enter => {
    let state = self.argument_input.as_mut().unwrap();

    // Parse arguments
    let args = if state.buffer.is_empty() {
        Vec::new()
    } else {
        match shell_words::split(&state.buffer) {
            Ok(parsed_args) => parsed_args,
            Err(_) => {
                // Show error message in status bar
                self.status_message = Some(StatusMessage::error(
                    "Cannot execute: fix parsing error first"
                ));
                return;  // Don't execute, stay in argument input
            }
        }
    };

    let command_name = state.command_name.clone();

    // Reset state
    self.argument_input = None;
    self.active_pane = ActivePane::CommandOutput;

    // Execute command
    self.execute_command_with_args(command_name, args);
}
```

**Test Plan**:
- [ ] Unmatched quote prevents execution
- [ ] Error message shown in status bar
- [ ] Dialog remains open after failed Enter
- [ ] Valid input executes normally

---

### 4. Update Tests (1 hour)

#### 4.1 New Unit Tests
**File**: `grove/src/tui_tests.rs`

```rust
#[test]
fn argument_input_cursor_moves_left() {
    let mut app = App::new(MockRegistry::empty(), MockDetailProvider::empty(), "test".to_string());
    app.argument_input = Some(ArgumentInputState {
        buffer: "test".to_string(),
        cursor_pos: 4,
        command_name: "cmd".to_string(),
    });
    app.active_pane = ActivePane::ArgumentInput;

    app.handle_key(KeyCode::Left);

    assert_eq!(app.argument_input.as_ref().unwrap().cursor_pos, 3);
}

#[test]
fn argument_input_cursor_stops_at_start() {
    let mut app = App::new(MockRegistry::empty(), MockDetailProvider::empty(), "test".to_string());
    app.argument_input = Some(ArgumentInputState {
        buffer: "test".to_string(),
        cursor_pos: 0,
        command_name: "cmd".to_string(),
    });
    app.active_pane = ActivePane::ArgumentInput;

    app.handle_key(KeyCode::Left);

    assert_eq!(app.argument_input.as_ref().unwrap().cursor_pos, 0);
}

#[test]
fn argument_input_inserts_char_at_cursor() {
    let mut app = App::new(MockRegistry::empty(), MockDetailProvider::empty(), "test".to_string());
    app.argument_input = Some(ArgumentInputState {
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
    let mut app = App::new(MockRegistry::empty(), MockDetailProvider::empty(), "test".to_string());
    app.argument_input = Some(ArgumentInputState {
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
    let mut app = App::new(MockRegistry::empty(), MockDetailProvider::empty(), "test".to_string());
    app.argument_input = Some(ArgumentInputState {
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
    assert!(app.status_message.as_ref().unwrap().text.contains("parsing error"));
}

#[test]
fn argument_input_home_end_keys() {
    let mut app = App::new(MockRegistry::empty(), MockDetailProvider::empty(), "test".to_string());
    app.argument_input = Some(ArgumentInputState {
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
```

#### 4.2 Update Existing Tests
All tests referencing `argument_buffer` and `argument_command_name` need to use new `ArgumentInputState` struct.

**Test Plan**:
- [ ] All new tests pass
- [ ] All existing argument tests still pass (after updates)
- [ ] Total test count: 87 → 93 (6 new tests)

---

## Migration Checklist

### Code Changes
- [ ] Create `ArgumentInputState` struct
- [ ] Replace `argument_buffer` + `argument_command_name` with `argument_input: Option<ArgumentInputState>`
- [ ] Update `execute_selected_command()` to create `ArgumentInputState`
- [ ] Update `handle_key_argument_input()` with cursor movement logic
- [ ] Update `render_argument_input_overlay()` with cursor rendering
- [ ] Add `format_argument_preview()` helper
- [ ] Update Enter handler to validate parsing
- [ ] Update Esc handler to clear `argument_input`

### Test Updates
- [ ] Add 6 new unit tests for cursor navigation
- [ ] Update existing 8 tests to use `ArgumentInputState`
- [ ] Verify all 93 tests pass

### Documentation
- [ ] Update specification with cursor navigation keys
- [ ] Update help text in dialog
- [ ] Add example screenshots to docs

### Verification
- [ ] Manual test: move cursor with arrows, insert/delete at position
- [ ] Manual test: preview shows correct parsed args
- [ ] Manual test: parse error prevents execution
- [ ] Manual test: Unicode input works correctly
- [ ] Run full test suite: `cargo test`
- [ ] Run clippy: `cargo clippy`
- [ ] Build release: `cargo build --release`

---

## Risk Assessment

### High Risk
- **Char/byte position confusion**: String indexing is tricky in Rust
  - **Mitigation**: Use `.chars().collect()` to work with char indices
  - **Test**: Unicode test cases with emoji/CJK

### Medium Risk
- **Cursor rendering with Unicode**: Wide chars might break visual alignment
  - **Mitigation**: Use `unicode-width` crate for accurate width calculation
  - **Test**: Render cursor with various Unicode inputs

### Low Risk
- **Performance**: Re-parsing on every keystroke
  - **Mitigation**: Acceptable for short inputs, profile if needed

---

## Timeline Estimate

| Task | Time | Assignee |
|------|------|----------|
| 1. Cursor position support | 2-3 hrs | Agent |
| 2. Command preview | 1-2 hrs | Agent |
| 3. Parse error blocking | 1 hr | Agent |
| 4. Update tests | 1 hr | Agent |
| **Total** | **5-7 hrs** | |

**Recommended**: Tackle in 2 sessions:
- **Session 1** (3 hrs): Cursor support + tests
- **Session 2** (2-3 hrs): Preview + error handling + tests

---

## Future Enhancements (Phase 2)

After Phase 1 is stable, consider:
- Delete key support (in addition to Backspace)
- Ctrl+U (clear line), Ctrl+W (delete word), Ctrl+K (kill to end)
- Horizontal scrolling for very long inputs
- Increase dialog width to 75% of screen

These are lower priority since they're "nice to have" rather than critical.
