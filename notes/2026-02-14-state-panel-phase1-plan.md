---
status: deprecated
date: 2026-02-14
completed: 2026-02-14
archived-reason: "Phase 1 implementation complete - see phase1-complete.md for summary"
---

# State Panel Phase 1: Critical UX Improvements

**Goal**: Add cache freshness indicators and refresh capability
**Effort**: 3-4 hours (actual: 3.5 hours)
**Priority**: HIGH
**Grade Impact**: B+ (85%) → A- (90%)

---

## Overview

Users currently cannot tell if cached state data is stale or refresh it without leaving Grove. This phase adds:
1. **Cache age display** - Show "(5m ago)" next to each query
2. **Refresh action** - Press 'r' to update selected query
3. **Empty state help** - Show example when no queries defined

---

## Task 1: Add Cache Age Display

### Current State
```
State Queries
┌────────────────────────────────────────────┐
│ ▶ writing    5000 words total, 250 today  │
│   tasks      59 open, 49 done             │
│   graph      2223 broken links, 463 orph  │
└────────────────────────────────────────────┘
```

### Target State
```
State Queries
┌────────────────────────────────────────────────────────┐
│ ▶ writing    5000 words total, 250 today     (5m ago) │
│   tasks      59 open, 49 done                (2h ago) │
│   graph      2223 broken links, 463 orph     (3d ago) │
└────────────────────────────────────────────────────────┘
```

### Implementation

**File**: `grove/src/tui.rs` (~line 1230)

**Current code**:
```rust
let items: Vec<ListItem> = self
    .state_queries
    .iter()
    .enumerate()
    .map(|(idx, query)| {
        let result = &self.state_results[idx];

        let line = match result {
            Some(result) => {
                let summary = result.summary();
                Line::from(vec![
                    Span::styled(
                        format!("{:12}", query.name),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::raw("  "),
                    Span::raw(summary),
                ])
            }
            None => {
                Line::from(vec![
                    Span::styled(
                        format!("{:12}", query.name),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        "(no cached data)",
                        Style::default().fg(Color::DarkGray),
                    ),
                ])
            }
        };

        ListItem::new(line)
    })
    .collect();
```

**New code**:
```rust
let items: Vec<ListItem> = self
    .state_queries
    .iter()
    .enumerate()
    .map(|(idx, query)| {
        let result = &self.state_results[idx];

        let line = match result {
            Some(result) => {
                let summary = result.summary();
                let age = result.metadata.time_ago(); // Already exists!

                Line::from(vec![
                    Span::styled(
                        format!("{:12}", query.name),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::raw("  "),
                    Span::raw(format!("{:40}", summary)), // Pad for alignment
                    Span::styled(
                        format!("({})", age),
                        Style::default().fg(Color::DarkGray),
                    ),
                ])
            }
            None => {
                Line::from(vec![
                    Span::styled(
                        format!("{:12}", query.name),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        "(no cached data)",
                        Style::default().fg(Color::DarkGray),
                    ),
                ])
            }
        };

        ListItem::new(line)
    })
    .collect();
```

**Key Changes**:
1. Call `result.metadata.time_ago()` - method already exists in StateMetadata!
2. Pad summary to 40 chars for alignment
3. Add age in gray at the end

**Testing**:
```rust
#[test]
fn state_panel_shows_cache_age() {
    use crate::state::{StateQuery, StateResult, StateMetadata};
    use serde_json::json;

    let mut app = App::new(
        MockRegistry::empty(),
        MockDetailProvider::empty(),
        "test-workspace".to_string()
    );

    app.state_queries = vec![
        StateQuery {
            name: "coverage".to_string(),
            description: None,
            deterministic: true,
            timeout: None,
        },
    ];

    app.state_results = vec![
        Some(StateResult {
            metadata: StateMetadata {
                query_name: "coverage".to_string(),
                commit_hash: "abc123".to_string(),
                timestamp: (chrono::Utc::now() - chrono::Duration::minutes(5))
                    .to_rfc3339(),
                command: "pytest --cov".to_string(),
                deterministic: true,
            },
            data: json!({"lines": 85}),
        }),
    ];

    // Render and check output contains "(5m ago)" or similar
    // (Would need to expose render logic for testing, or do visual inspection)
}
```

**Effort**: 30 minutes

---

## Task 2: Add Refresh Action

### User Flow

```
1. User sees stale data:
   ▶ tasks      59 open, 49 done     (3d ago)  ← Old!

2. Presses 'r'

3. Status bar shows:
   ℹ Refreshing tasks...

4. Grove executes in background:
   graft state query tasks --refresh

5. On success:
   ✓ Refreshed tasks

6. Panel updates:
   ▶ tasks      62 open, 51 done     (just now)  ← Fresh!
```

### Implementation Strategy

**Challenge**: Need to spawn subprocess asynchronously without blocking UI.

**Option A: Blocking (Simple)**
- Pros: Easy to implement (30 min)
- Cons: UI freezes during refresh (bad UX)

**Option B: Async (Better)**
- Pros: UI stays responsive
- Cons: More complex (2-3 hours)

**Recommendation**: Start with **Option A** for MVP, upgrade to B later if needed.

### Implementation - Option A (Blocking)

**File**: `grove/src/tui.rs`

**Step 1**: Add refresh handler

```rust
fn handle_key_state_panel(&mut self, code: KeyCode) {
    match code {
        KeyCode::Char('j') | KeyCode::Down => {
            // ... existing code ...
        }
        KeyCode::Char('k') | KeyCode::Up => {
            // ... existing code ...
        }
        KeyCode::Char('r') => {
            // NEW: Refresh selected query
            self.refresh_selected_state_query();
        }
        KeyCode::Char('q') | KeyCode::Esc => {
            // ... existing code ...
        }
        _ => {}
    }
}
```

**Step 2**: Implement refresh method

```rust
fn refresh_selected_state_query(&mut self) {
    use std::process::Command;

    // Get selected query
    let selected = match self.state_panel_list_state.selected() {
        Some(i) => i,
        None => return,
    };

    let query = match self.state_queries.get(selected) {
        Some(q) => q,
        None => return,
    };

    // Get repo path
    let repos = self.registry.list_repos();
    let repo_path = match self.list_state.selected() {
        Some(i) => repos.get(i).map(|r| r.as_path()),
        None => return,
    };

    let repo_path = match repo_path {
        Some(p) => p,
        None => return,
    };

    // Show "Refreshing..." message
    self.status_message = Some(StatusMessage::info(
        format!("Refreshing {}...", query.name)
    ));

    // Need to render the status message before blocking
    // (This is the UX limitation of blocking approach)

    // Execute: graft state query <name> --refresh
    let result = Command::new("graft")
        .args(&["state", "query", &query.name, "--refresh"])
        .current_dir(repo_path)
        .output();

    match result {
        Ok(output) if output.status.success() => {
            // Reload cache for this query
            self.reload_state_query_cache(selected);

            self.status_message = Some(StatusMessage::success(
                format!("Refreshed {}", query.name)
            ));
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.status_message = Some(StatusMessage::error(
                format!("Failed to refresh {}: {}", query.name, stderr)
            ));
        }
        Err(e) => {
            self.status_message = Some(StatusMessage::error(
                format!("Failed to run graft: {}", e)
            ));
        }
    }
}
```

**Step 3**: Add cache reload helper

```rust
fn reload_state_query_cache(&mut self, query_index: usize) {
    use crate::state::{compute_workspace_hash, read_latest_cached};
    use std::path::Path;

    if query_index >= self.state_queries.len() {
        return;
    }

    let query = &self.state_queries[query_index];

    // Get repo info
    let repos = self.registry.list_repos();
    let selected = match self.list_state.selected() {
        Some(i) => i,
        None => return,
    };

    let repo_path = match repos.get(selected) {
        Some(r) => r.as_path(),
        None => return,
    };

    let workspace_hash = compute_workspace_hash(&self.workspace_name);
    let repo_name = Path::new(repo_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // Read updated cache
    match read_latest_cached(&workspace_hash, repo_name, &query.name) {
        Ok(result) => {
            self.state_results[query_index] = Some(result);
        }
        Err(e) => {
            log::warn!("Failed to reload cache for {}: {}", query.name, e);
            self.state_results[query_index] = None;
        }
    }
}
```

**Drawbacks of Blocking Approach**:
- UI freezes for 1-10 seconds during refresh
- Can't show spinner/progress
- Can't cancel once started

**Alternative - Option B: Async (Better UX)**

Would require:
1. Add `tokio` dependency
2. Spawn async task for refresh
3. Use channels to communicate completion
4. Add refresh state tracking (`RefreshState { InProgress, Completed, Failed }`)
5. Render spinner while in progress

**Effort**: 2-3 hours vs 30 min for blocking

**Decision**: Ship blocking version first, upgrade to async if users complain.

### Testing Refresh

```rust
#[test]
fn refresh_executes_graft_command() {
    // This would require mocking Command::new()
    // Or use integration test with real graft command

    // Pseudo-code:
    // 1. Set up state panel with query
    // 2. Press 'r' key
    // 3. Verify graft command was executed
    // 4. Verify cache was reloaded
}

#[test]
fn refresh_shows_error_on_failure() {
    // Mock graft command failure
    // Verify error status message shown
}
```

**Effort**: 1 hour (blocking version)

---

## Task 3: Improve Empty State

### Current State
When no queries defined, panel shows blank/empty.

### Target State
```
┌─ State Queries ─────────────────────────────────────────┐
│                                                          │
│  No state queries defined in graft.yaml                 │
│                                                          │
│  State queries track project metrics over time.         │
│                                                          │
│  Example:                                                │
│    state:                                                │
│      coverage:                                           │
│        run: "pytest --cov --cov-report=json"            │
│        cache:                                            │
│          deterministic: true                             │
│                                                          │
│  Press 'q' to close                                      │
└──────────────────────────────────────────────────────────┘
```

### Implementation

**File**: `grove/src/tui.rs` (~line 1265)

**Current code**:
```rust
// If no queries, show empty state
if self.state_queries.is_empty() {
    let empty_msg = Paragraph::new("No state queries defined")
        .alignment(Alignment::Center)
        .block(block);
    frame.render_widget(empty_msg, area);
} else {
    // Render list...
}
```

**New code**:
```rust
// If no queries, show helpful empty state
if self.state_queries.is_empty() {
    let help_text = vec![
        Line::from(""),
        Line::from("No state queries defined in graft.yaml"),
        Line::from(""),
        Line::from("State queries track project metrics over time."),
        Line::from(""),
        Line::from("Example:"),
        Line::from("  state:"),
        Line::from("    coverage:"),
        Line::from("      run: \"pytest --cov --cov-report=json\""),
        Line::from("      cache:"),
        Line::from("        deterministic: true"),
        Line::from(""),
        Line::from(Span::styled(
            "Press 'q' to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let empty_msg = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .block(block);
    frame.render_widget(empty_msg, area);
} else {
    // Render list...
}
```

**Effort**: 15 minutes

---

## Task 4: Update Documentation

### Update Help Overlay

**File**: `grove/src/tui.rs` (~line 1110)

**Add to Actions section**:
```rust
Line::from("  r            Refresh repository status"),
Line::from("  x            Execute command (from graft.yaml)"),
Line::from("  s            View state queries (from detail pane)"),
// Add this:
Line::from(""),
Line::from(Span::styled("State Panel", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
Line::from("  j, ↓         Select next query"),
Line::from("  k, ↑         Select previous query"),
Line::from("  r            Refresh selected query"),
Line::from("  q, Esc       Close panel"),
Line::from("  ?            Show this help"),
```

### Update TUI Specification

**File**: `docs/specifications/grove/tui-behavior.md`

**Add refresh scenarios**:

```gherkin
#### Refreshing State Queries

```gherkin
Given the state panel is open
And a query is selected
When the user presses 'r'
Then the query is re-executed via graft CLI
And the cache is updated
And the panel shows the fresh result
And a success message is displayed
```

```gherkin
Given the user presses 'r' to refresh a query
And the graft command fails
Then an error message is shown in the status bar
And the error details are logged
And the cached data remains unchanged
```

```gherkin
Given the state panel shows cached results
Then each query displays its cache age (e.g., "5m ago", "2h ago")
And users can see data freshness at a glance
```
```

**Effort**: 15 minutes

---

## Testing Plan

### Unit Tests

**File**: `grove/src/tui_tests.rs`

Add tests for:

```rust
#[test]
fn state_panel_shows_cache_age_for_results() {
    // Verify age string rendered
}

#[test]
fn state_panel_refresh_key_triggers_refresh() {
    // Verify 'r' key calls refresh logic
}

#[test]
fn state_panel_shows_empty_state_with_example() {
    // Verify helpful empty state shown
}
```

**Effort**: 30 minutes

### Integration Tests

**File**: `grove/tests/test_state_panel.rs`

Add tests for:

```rust
#[test]
fn refresh_updates_cache_file() {
    // 1. Create temp graft.yaml with query
    // 2. Run graft state query to populate cache
    // 3. Verify cache exists
    // 4. Modify source data
    // 5. Simulate refresh
    // 6. Verify cache updated
}
```

**Effort**: 30 minutes

### Manual Testing

**Checklist**:
- [ ] Cache age displays correctly (5m, 2h, 3d formats)
- [ ] Age updates on panel reopen
- [ ] Refresh works with real graft query
- [ ] Refresh shows error if graft not installed
- [ ] Empty state shows helpful example
- [ ] Help overlay documents refresh key
- [ ] Status messages shown during refresh

**Effort**: 30 minutes

---

## Total Effort Breakdown

| Task | Effort |
|------|--------|
| Cache age display | 30 min |
| Refresh action (blocking) | 1 hour |
| Empty state improvement | 15 min |
| Documentation updates | 15 min |
| Unit tests | 30 min |
| Integration tests | 30 min |
| Manual testing | 30 min |
| **Total** | **3.5 hours** |

---

## Success Criteria

**Before Phase 1** (B+ / 85%):
- ✓ State panel shows cached data
- ✗ No indication of cache age
- ✗ No way to refresh from Grove
- ✗ Empty state is unhelpful

**After Phase 1** (A- / 90%):
- ✓ Cache age displayed for all queries
- ✓ Refresh works (blocking version)
- ✓ Empty state shows helpful example
- ✓ Help overlay documents refresh
- ✓ All tests pass

**Known Limitations** (acceptable for now):
- Refresh is blocking (UI freezes briefly)
- No refresh progress indicator
- No bulk refresh (must refresh one at a time)

**Future improvements** (Phase 2/3):
- Async refresh with spinner
- Detail view (Enter to see full JSON)
- Provider abstraction (decouple from graft CLI)

---

## Implementation Order

1. **Cache age display** (30 min)
   - Quick win, high value
   - No external dependencies

2. **Empty state help** (15 min)
   - Quick win, improves discoverability

3. **Refresh action** (1 hour)
   - Core functionality
   - Most complex piece

4. **Tests** (1.5 hours)
   - Verify everything works
   - Prevent regressions

5. **Documentation** (15 min)
   - Update help and spec
   - Final polish

**Total**: 3.5 hours end-to-end

---

## Risk Mitigation

**Risk 1**: `graft` command not available
- **Mitigation**: Show clear error "graft CLI not found"
- **Test**: Verify error message with graft not in PATH

**Risk 2**: Refresh takes too long (>10 seconds)
- **Mitigation**: Add timeout to graft command
- **Future**: Upgrade to async with cancel button

**Risk 3**: Cache file format changes
- **Mitigation**: Error handling already robust
- **Future**: Add version checking

**Risk 4**: Time calculation bugs (off-by-one in "5m ago")
- **Mitigation**: Use existing `time_ago()` method (already tested)
- **Test**: Verify with specific timestamps

---

## Next Steps

1. **Decide**: Ship Phase 1 now or defer?
   - **Recommend**: Ship soon - critical for usability

2. **If shipping**:
   - Create feature branch: `feat/state-panel-phase1`
   - Implement in order above
   - Write tests as you go
   - Manual testing before merge
   - Update CHANGELOG

3. **If deferring**:
   - Document decision in critique
   - Monitor user feedback on current version
   - Revisit based on demand

**Recommended decision**: **Ship Phase 1** - cache age and refresh are table stakes for a "state monitoring" feature.
