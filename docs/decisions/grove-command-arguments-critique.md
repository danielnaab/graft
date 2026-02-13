---
status: review
date: 2026-02-13
reviewers: [human, agent]
---

# Grove Command Arguments Implementation - Critique & Improvement Plan

## Executive Summary

**What we built**: Modal text input dialog for command arguments with shell-style parsing
**Status**: ✅ Functional, but has significant UX limitations
**Recommendation**: Ship as-is for MVP, prioritize cursor navigation and error feedback for next iteration

---

## Implementation Review

### What Went Well ✅

1. **Clean architecture**
   - Followed existing modal UI pattern (Help, CommandPicker, ArgumentInput)
   - Proper separation: state management → UI rendering → command execution
   - Each phase built logically on the previous one

2. **Solid test coverage**
   - 8 unit tests covering all key behaviors
   - 1 integration test verifying end-to-end argument passing
   - All tests passing, including shell-style parsing validation

3. **User-driven design**
   - Directly solves user's quick-capture workflow (`notecap capture Personal "note"`)
   - Non-breaking: empty input skips arguments (backwards compatible)
   - Clear help text with quoting examples

4. **Minimal dependencies**
   - Only added `shell-words` (1.1.0) - small, focused crate
   - No heavyweight text editing libraries
   - Keeps Grove lightweight

5. **Iterative problem-solving**
   - Caught and fixed dialog sizing bug (percentage vs absolute)
   - Fixed output pane background bleed-through
   - Added proper shell-style parsing after simple split proved inadequate

### What Needs Improvement 🔧

#### **Critical UX Issues** (Blocks power users)

1. **No cursor position control**
   ```
   Current:  > This is a typo_
   Can't do: > This is |a typo  (move cursor to middle)
   ```
   - User can only delete from end (Backspace)
   - Can't fix typos in middle of input
   - No left/right arrow support
   - No Home/End keys

2. **Limited editing operations**
   - ❌ No Delete key (only Backspace)
   - ❌ No Ctrl+U (clear line)
   - ❌ No Ctrl+W (delete word)
   - ❌ No Ctrl+A (select all)
   - ❌ No Ctrl+K (kill to end)
   - **Impact**: Fixing mistakes is tedious, especially for long arguments

3. **No visual feedback for parsing errors**
   ```
   Input:  arg1 "unclosed quote
   Result: Silently treated as 1 argument: arg1 "unclosed quote
   User:   No idea their quoting failed
   ```
   - Parse errors swallowed by fallback (`Err(_) => vec![whole_string]`)
   - User doesn't know if shell parsing succeeded
   - Can't tell how many arguments they'll actually pass

4. **No command preview**
   - User types: `Personal "This is a test"`
   - No way to see: `Will execute: graft run capture Personal 'This is a test'`
   - Can't verify argument parsing before execution

#### **Technical Limitations**

1. **Fixed dialog width**
   ```rust
   let dialog_width = 60.min(area.width.saturating_sub(4));
   ```
   - 60 chars might be too narrow for complex commands
   - Could use 70-80% of screen width dynamically
   - No horizontal scrolling for overflow

2. **Error handling is basic**
   ```rust
   Err(_) => vec![self.argument_buffer.clone()]  // Swallows error
   ```
   - No distinction between parse failures and success
   - User gets no feedback about malformed input
   - Potential for confusing error messages from graft

3. **State management could be cleaner**
   - `argument_buffer: String` and `argument_command_name: Option<String>` are separate
   - Could encapsulate in `ArgumentInputState` struct
   - Using `Option` when we know command name exists (in ArgumentInput pane)

4. **No argument validation**
   - Can't check if command expects 2 args but got 5
   - No way to show "Expected: <section> <content>" hint
   - Commands with complex arg patterns unsupported

#### **Missing Features**

1. **No command history**
   - Can't press ↑ to recall last arguments for this command
   - Each execution starts from blank slate
   - Power users expect history (like shell)

2. **No multi-line support**
   - Very long argument strings get truncated
   - No way to see full input if it exceeds 60 chars
   - No scrolling within input field

3. **Cancel loses state**
   - Esc clears buffer and returns to repo list
   - Can't partially enter args, cancel to check something, and resume
   - Workflow interruption is jarring

4. **No clipboard support**
   - Can't paste pre-formatted arguments from notes
   - Can't copy arguments to refine elsewhere
   - Manual typing only

#### **Code Quality Issues**

1. **Magic numbers**
   ```rust
   let dialog_width = 60;  // Why 60?
   let dialog_height = 7;   // Why 7?
   ```
   - Should be named constants: `ARGUMENT_DIALOG_WIDTH`, `ARGUMENT_DIALOG_HEIGHT`

2. **Help text is hardcoded in render**
   ```rust
   let help = "Use quotes for spaces: arg1 \"arg with spaces\"  (Enter=run, Esc=cancel)";
   ```
   - Should be a constant for reuse/testing
   - Too long for 60-char dialog on small terminals

3. **Tests don't verify full integration**
   - We test `shell_words::split()` separately
   - We test `handle_key_argument_input()` separately
   - No test verifying parsed args actually reach subprocess correctly with quotes

#### **Platform/Compatibility Concerns**

1. **Windows compatibility unknown**
   - `shell-words` crate is Unix-focused
   - Quoting behavior might differ on Windows
   - No testing on Windows platform

2. **Escape sequence behavior**
   - Can users type `\"` inside quotes? Unclear.
   - What about `\\` for literal backslash?
   - No documentation or tests for edge cases

3. **Unicode handling**
   - Cursor position with wide chars (emoji, CJK) might break
   - `argument_buffer.pop()` might corrupt UTF-8
   - No tests for international input

---

## Critique Summary

| Aspect | Grade | Rationale |
|--------|-------|-----------|
| **Architecture** | A | Clean separation, follows existing patterns |
| **Test Coverage** | B+ | Good unit tests, but missing integration edge cases |
| **UX (Basic)** | B | Works for simple cases, frustrating for complex |
| **UX (Power Users)** | D | No cursor nav, no history, limited editing |
| **Error Handling** | C | Silent failures, no validation feedback |
| **Documentation** | B | Spec updated, but missing usage examples |
| **Code Quality** | B | Readable, but has magic numbers and hardcoded strings |
| **Overall** | **B-** | **Functional MVP, needs UX polish** |

---

## Improvement Plan

### Phase 1: Critical UX Fixes (Next Sprint)

**Goal**: Make editing comfortable for daily use

#### 1.1 Add Cursor Position Support
```rust
struct ArgumentInputState {
    buffer: String,
    cursor_pos: usize,  // NEW
}
```

**Implementation**:
- Add `cursor_pos` field to track position in buffer
- Handle `KeyCode::Left` / `KeyCode::Right` to move cursor
- Handle `KeyCode::Home` / `KeyCode::End`
- Render cursor at actual position, not just end
- Update `Char` and `Backspace` to respect cursor position

**Effort**: ~2-3 hours
**Impact**: 🔥 High - Dramatically improves editing experience

#### 1.2 Add Command Preview Line
```
┌─ Arguments for 'capture' ──────────────────────────┐
│                                                     │
│ > Personal "This is a test"                        │
│ Will execute: graft run capture Personal 'This is a test'
│                                                     │
│ Use quotes for spaces (Enter=run, Esc=cancel)      │
└─────────────────────────────────────────────────────┘
```

**Implementation**:
- Parse arguments on every keystroke (in render)
- Show parsed result in preview line
- Highlight parsing errors in red
- Increase dialog height to 8 or 9 lines

**Effort**: ~1-2 hours
**Impact**: 🔥 High - Instant feedback on parsing

#### 1.3 Show Parsing Errors Visually
```rust
let (parsed, error) = match shell_words::split(&buffer) {
    Ok(args) => (args, None),
    Err(e) => (vec![], Some(format!("Parse error: {}", e))),
};
```

**Implementation**:
- Capture parse errors instead of swallowing
- Show error in red in dialog
- Prevent execution (Enter) if parse failed
- Clear error as user types valid input

**Effort**: ~1 hour
**Impact**: 🔥 High - Prevents user confusion

**Total Phase 1**: ~4-6 hours, 3 high-impact improvements

---

### Phase 2: Enhanced Editing (Follow-up)

**Goal**: Match basic readline/shell editing expectations

#### 2.1 Add More Editing Keys
- `Delete`: Delete char at cursor
- `Ctrl+U`: Clear entire line
- `Ctrl+W`: Delete word backwards
- `Ctrl+K`: Kill to end of line
- `Ctrl+A` / `Ctrl+E`: Home/End (in addition to Home/End keys)

**Effort**: ~2-3 hours (add to key handler, update tests)

#### 2.2 Increase Dialog Width
```rust
const ARGUMENT_DIALOG_WIDTH_PCT: f32 = 0.75;  // 75% of screen
let dialog_width = (area.width as f32 * ARGUMENT_DIALOG_WIDTH_PCT) as u16;
```

**Effort**: ~30 min

#### 2.3 Add Horizontal Scrolling
- If input exceeds visible width, scroll to keep cursor visible
- Show `<` / `>` indicators when content scrolled

**Effort**: ~2-3 hours

**Total Phase 2**: ~5-7 hours

---

### Phase 3: Advanced Features (Future)

**Goal**: Power-user productivity enhancements

#### 3.1 Command History
- Store last 10 argument sets per command name
- `↑` / `↓` to navigate history
- Persist in `~/.grove/command-history.json`

**Effort**: ~4-6 hours

#### 3.2 Argument Validation
- Parse command description for arg hints
- Warn if arg count doesn't match expected
- Show hint: "Expected: <section> <content>"

**Effort**: ~3-4 hours

#### 3.3 Tab Completion
- Complete file paths
- Complete common values from history
- Autocomplete based on command schema (if available)

**Effort**: ~6-8 hours (complex)

**Total Phase 3**: ~13-18 hours

---

## Recommendation

### Ship Current Implementation ✅
- **Why**: Functional for 80% of use cases (simple args, quoted strings)
- **Caveat**: Document limitation: "Basic text input, use graft.yaml for complex args"

### Prioritize Phase 1 🎯
- **Why**: Addresses most painful UX issues with minimal effort (4-6 hrs)
- **When**: Next available sprint (within 1-2 weeks)
- **Risk**: Low - localized changes, well-tested

### Consider Phase 2 📋
- **Why**: Matches user expectations from shell/terminal experience
- **When**: After Phase 1 proves stable (1-2 sprints later)
- **Risk**: Medium - horizontal scrolling can be tricky

### Defer Phase 3 ⏳
- **Why**: Complex features, diminishing returns for effort
- **When**: After user feedback on Phases 1-2 (2-3 months)
- **Risk**: Medium-High - persistence, validation, completion all have edge cases

---

## Risks & Mitigations

### Risk: Cursor position breaks with wide chars
**Mitigation**: Use `unicode-width` for cursor calculation (already a dependency)

### Risk: Shell-words behaves differently on Windows
**Mitigation**: Test on Windows, document known issues, consider platform-specific parsing

### Risk: Command history grows unbounded
**Mitigation**: Cap at 10 entries per command, LRU eviction

### Risk: Users expect full readline features
**Mitigation**: Document as "basic input", suggest graft.yaml for complex commands

---

## Conclusion

The current implementation is a **solid MVP** that solves the immediate user need (quick capture with arguments). However, **editing UX is the critical gap** - users can't fix typos in the middle of input, and lack of preview/error feedback causes confusion.

**Recommended path forward**:
1. ✅ Ship current version (it works!)
2. 🎯 Immediately prioritize Phase 1 (cursor nav + preview + error feedback)
3. 📋 Evaluate Phase 2 after user feedback
4. ⏳ Defer Phase 3 until clear user demand

This balances **delivering value now** with **planned iteration** based on real usage patterns.

---

## Appendix: Alternative Approaches Considered

### Alt 1: Use `tui-textarea` crate
- **Pro**: Full-featured text editing out of the box
- **Con**: Adds dependency (currently avoided), might be overkill for single-line input
- **Decision**: Rejected for now, reconsider if Phase 2 proves too complex

### Alt 2: Prompt user for each arg separately
- **Pro**: No parsing needed, very clear UX
- **Con**: Slow for multi-arg commands, breaks flow
- **Decision**: Rejected, single-input is more efficient

### Alt 3: Show command picker with args pre-filled in graft.yaml
- **Pro**: No typing needed, just select
- **Con**: Doesn't support dynamic content ("This is a test" changes each time)
- **Decision**: Rejected, dynamic args are the whole point

### Alt 4: Use external $EDITOR for complex args
- **Pro**: Full editor power for complex inputs
- **Con**: Breaks TUI flow, slow for simple inputs
- **Decision**: Rejected, but could be bonus feature (Ctrl+E to edit in $EDITOR)
