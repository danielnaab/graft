# Grove Slice 1 Review & Improvement Plan

## Context

Slice 1 ("Workspace Config + Repo List TUI") is marked as **completed** (2026-02-10). This review audits the implementation against the specification to identify gaps, quality issues, and opportunities for polish before declaring Slice 1 "production ready."

## Spec vs Implementation Audit

### ✅ Delivered Features (Slice 1 Scope)

| Feature | Spec | Implementation | Quality |
|---------|------|----------------|---------|
| workspace.yaml parsing | ✓ | ✓ | Excellent - helpful error messages |
| Repo registry | ✓ | ✓ | Good - clean trait separation |
| Git status (branch, dirty, ahead/behind) | ✓ | ✓ | Excellent - gitoxide integration |
| TUI event loop | ✓ | ✓ | Good - ratatui scaffold |
| Repo list widget | ✓ | ✓ | Excellent - adaptive path display |
| Status indicators | ✓ | ✓ | Good - color-coded, unicode |
| j/k navigation | ✓ | ✓ | Excellent - wrapping, vim-style |
| q to quit | ✓ | ✓ | Good - proper cleanup |
| Integration tests | ✓ | ✓ | Good - 7 tests covering end-to-end |

### ⚠️ Spec Features Marked "Unimplemented"

These are explicitly marked as future work in the spec:

- **Tags display** - `workspace.yaml` supports tags, TUI doesn't display them (marked [Unimplemented] in spec)
- **Repository status scripts** - Custom status checks from config (Slice 1 scope but deferred)
- **Workspace name display** - Config has `name` field, TUI doesn't show it anywhere

### 🐛 Quality Issues Found

#### High Priority

1. **No visual feedback during status refresh**
   - **Issue**: On launch, git status queries can take 1-2 seconds for large repos
   - **User sees**: Blank screen until everything loads
   - **Fix**: Show "Loading..." state or progress indicator
   - **Impact**: Users might think app hung

2. **Error state not well-indicated in list**
   - **Issue**: Repos with errors show `[error: ...]` in red/yellow, but long error messages get truncated
   - **User sees**: `[error: Failed to...]` - can't see full error
   - **Fix**: Truncate error messages sensibly, show full error in detail pane
   - **Impact**: Users can't diagnose issues

3. **No indication of empty workspace**
   - **Issue**: If `repositories: []` in config, TUI shows blank list with navigation keys active
   - **User sees**: Empty pane, unclear if it's working
   - **Fix**: Show "No repositories configured" message with help text
   - **Impact**: Confusing first-run experience

#### Medium Priority

4. **Workspace name not displayed**
   - **Issue**: Config has `name: "my-workspace"` but TUI doesn't show it
   - **User sees**: No indication which workspace they're in
   - **Fix**: Show workspace name in title or header
   - **Impact**: Users can't distinguish multiple workspace configs

5. **No keyboard shortcuts hint**
   - **Issue**: Only hint is in title bar: "(j/k navigate, Enter/Tab detail)"
   - **User sees**: Truncated hints on narrow terminals
   - **Fix**: Add `?` key to show help overlay, or footer with key hints
   - **Impact**: Discoverability of features

6. **Status refresh timestamp not shown**
   - **Issue**: Status is queried on launch, never re-queried
   - **User sees**: No way to know if status is stale
   - **Fix**: Show "Last updated: X seconds ago" or add `r` key to refresh
   - **Impact**: Users might look at stale status without realizing

#### Low Priority

7. **No graceful handling of terminal resize**
   - **Issue**: Terminal resize might cause layout issues
   - **Fix**: Handle `Event::Resize` and redraw
   - **Impact**: Minor UX glitch

8. **Arrow keys work but not documented**
   - **Issue**: Up/Down arrows work alongside j/k, but title only mentions j/k
   - **Fix**: Update title to "(↑↓/jk navigate...)"
   - **Impact**: Minor discoverability

9. **No visual distinction between focused/unfocused when single pane**
   - **Issue**: In Slice 1 (before split-pane), focus concept exists but isn't visual
   - **Fix**: Not applicable until Slice 2, but worth noting
   - **Impact**: None for Slice 1

### 🎯 Missing Polish (Not in Spec, But Expected)

10. **No version shown**
    - TUI doesn't show grove version anywhere
    - Fix: Show in title bar or help overlay
    - Useful for bug reports

11. **No indication of detached HEAD repos in summary**
    - Repos with `[detached]` branch blend in with normal repos
    - Fix: Use different indicator or color
    - Helps spot unusual states

12. **Status indicators could be more informative**
    - `●` vs `○` is good, but no legend
    - `↑4 ↓2` is clear, but first-time users might not know
    - Fix: Help overlay with legend

## Proposed Improvements

### P0: Critical UX Issues (Block "Production Ready")

**Issue #1: Loading state feedback**
- **Change**: Show "Refreshing status..." message during `refresh_all()`
- **Implementation**:
  - Add loading state to TUI
  - Render "Loading workspace..." on initial draw
  - Update after refresh completes
- **Test**: Manual (launch with large repo)
- **Effort**: 30 min

**Issue #3: Empty workspace state**
- **Change**: Show helpful message when `repositories.is_empty()`
- **Implementation**:
  - Check `repos.is_empty()` in render
  - Display centered message: "No repositories configured\n\nEdit ~/.config/grove/workspace.yaml to add repositories"
- **Test**: Integration test with empty config
- **Effort**: 20 min

### P1: High-Value Polish (Improves UX significantly)

**Issue #4: Show workspace name**
- **Change**: Display workspace name in TUI title bar
- **Implementation**:
  - Pass workspace name from config to TUI App
  - Update title: "Grove: {workspace_name}"
- **Test**: Manual verification
- **Effort**: 15 min

**Issue #2: Better error display**
- **Change**: Truncate error messages in list, show full in detail pane
- **Implementation**:
  - Limit error message in list to 20 chars with "..."
  - Show full error in detail pane when selected
- **Test**: Manual (create repo with error state)
- **Effort**: 30 min

**Issue #6: Status refresh indicator**
- **Change**: Add `r` key to manually refresh status
- **Implementation**:
  - Add keybinding for `r` in repo list
  - Show "Refreshing..." message
  - Call `registry.refresh_all()`
  - Update TUI
- **Test**: Manual
- **Effort**: 45 min

### P2: Nice-to-Have (Improves discoverability)

**Issue #5: Help overlay**
- **Change**: Add `?` key to show help screen
- **Implementation**:
  - New help modal overlay
  - Lists all keybindings with descriptions
  - Shows status indicator legend
  - Press `?` or `Esc` to dismiss
- **Test**: Manual
- **Effort**: 60 min

**Issue #10: Version display**
- **Change**: Show version in title bar or help overlay
- **Implementation**: Include `env!("CARGO_PKG_VERSION")` in display
- **Effort**: 5 min (with help overlay) or 10 min (standalone)

### P3: Future Work (Defer to later slices)

**Issue #7: Terminal resize handling**
- Defer until we see it causing actual problems
- Not critical for Slice 1

**Tags display**
- Explicitly marked [Unimplemented] in spec
- Defer to future slice when filtering/sorting is added

**Repository status scripts**
- Slice 1 scope but complex feature
- Defer until Slice 3+ when we have more UI patterns established

## Implementation Plan

### Phase 1: Critical UX (P0) - ~50 min
1. Add loading state during refresh (#1) - 30 min
2. Show empty workspace message (#3) - 20 min

**Goal**: Make Slice 1 feel responsive and complete

### Phase 2: High-Value Polish (P1) - ~90 min
3. Display workspace name in title (#4) - 15 min
4. Better error message display (#2) - 30 min
5. Add manual refresh keybinding `r` (#6) - 45 min

**Goal**: Make Slice 1 feel professional and informative

### Phase 3: Discoverability (P2) - ~65 min
6. Add help overlay with `?` key (#5) - 60 min
7. Show version in help (#10) - 5 min

**Goal**: Make Slice 1 self-documenting

**Total effort**: ~3 hours of focused work

### Phase 4: Spec Update
- Update tui-behavior.md with new scenarios:
  - Loading state behavior
  - Empty workspace behavior
  - Manual refresh behavior
  - Help overlay behavior
- Add decisions:
  - `r` key for manual refresh (vs auto-refresh timer)
  - `?` key for help (vs `h` or dedicated help pane)
  - Workspace name in title bar (vs status bar)

## Success Criteria

**Slice 1 is "Production Ready" when:**
- [ ] No blank screens during loading
- [ ] Empty workspace shows helpful message
- [ ] Workspace name is visible
- [ ] Error messages are readable (truncated in list, full in detail)
- [ ] Users can manually refresh status with `r`
- [ ] Help is accessible with `?` key
- [ ] All behaviors are documented in tui-behavior.md
- [ ] Integration tests cover edge cases (empty workspace, errors)

## Out of Scope (Future Slices)

- Tags display and filtering (needs UI design)
- Custom status scripts (needs execution engine)
- Live auto-refresh / file watching
- Performance optimization (fast enough for <20 repos)
- Advanced error recovery (graceful degradation is sufficient)

## Risk Assessment

**Low risk improvements:**
- Loading state, empty workspace, workspace name, version - straightforward UI changes

**Medium risk:**
- Manual refresh - need to ensure TUI state consistency during re-query
- Error message handling - need to test various error types

**High risk:**
- Help overlay - new modal pattern, needs careful focus management
- Defer to Phase 3, implement after other improvements validated

## Next Steps

1. Review this plan with user
2. Implement Phase 1 (critical UX)
3. Test manually with real workspace
4. Implement Phase 2 (polish)
5. Implement Phase 3 (help)
6. Update specifications
7. Mark Slice 1 as "production ready"
8. Begin Slice 3 planning
