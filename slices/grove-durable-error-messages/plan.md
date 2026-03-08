---
status: done
created: 2026-03-06
completed: 2026-03-06
completed_note: "Implemented during grove TUI buildout; show_error/show_warning dual-write to status bar + transcript block, all call sites migrated."
---

# Persist errors and warnings in the transcript

## Story

Errors and warnings in grove are displayed as transient status bar overlays
that disappear after 3 seconds. If the user blinks or is looking elsewhere, the
error is gone with no way to see what happened. This affects every error path in
the TUI — from scion failures to focus warnings to command errors. Users can't
diagnose issues because the evidence vanishes.

After this slice, errors and warnings are still flashed in the status bar (for
immediate visibility), but they also leave a durable styled block in the
transcript scroll buffer. Users can scroll back to see what went wrong. Info
and success messages remain ephemeral (they're confirmations, not actionable
diagnostics).

## Approach

1. **Add `show_error()` and `show_warning()` helper methods** on `TranscriptApp`
   that:
   - Set `self.status = Some(StatusMessage::error/warning(text))` (existing flash)
   - Push a compact styled `ContentBlock::Text` to the scroll buffer:
     - Errors: single line `✗ <text>` in `Color::Red`, dimmed
     - Warnings: single line `⚠ <text>` in `Color::Yellow`, dimmed

2. **Replace all 41 `self.status = Some(StatusMessage::error/warning(...))`
   call sites** with `self.show_error(text)` / `self.show_warning(text)`.
   This is a mechanical find-and-replace — no logic changes at any call site.

3. **Info and success messages unchanged** — the 18 `StatusMessage::info()` and
   `StatusMessage::success()` sites continue to use direct `self.status`
   assignment (ephemeral only).

### Transcript block style

The durable blocks are intentionally minimal — one line each, dimmed styling,
no title or collapse affordance. They serve as a log trail, not prominent
output. Exact styling:

- **Errors**: `Span::styled(format!("✗ {text}"), Style::default().fg(Color::Red).add_modifier(Modifier::DIM))`
- **Warnings**: `Span::styled(format!("⚠ {text}"), Style::default().fg(Color::Yellow).add_modifier(Modifier::DIM))`

Each is wrapped in a `ContentBlock::Text` with `collapsed: false`.

Example transcript after a failed scion start:

```
⚠ No values found for query: slices
✗ scion start failed: dependency 'software-factory' not found
✗ command execution error: no active session for scion 'test'
```

## Acceptance Criteria

- Every `StatusMessage::error()` and `StatusMessage::warning()` call produces
  both a status bar flash AND a durable transcript block
- Error blocks show `✗ <text>` in red/dimmed
- Warning blocks show `⚠ <text>` in yellow/dimmed
- Blocks persist in scroll history (user can scroll back to see them)
- Info and success messages remain ephemeral (status bar only)
- No change in behavior for any existing command — only the persistence of
  error/warning visibility is affected
- After migration, `grep -c 'StatusMessage::error\|StatusMessage::warning' transcript.rs`
  returns 0 (no direct assignments remain outside the helpers themselves)
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [x] **Add `show_error()` and `show_warning()` helper methods**
  - **Delivers** — centralized dual-write for error/warning messages
  - **Done when** — `TranscriptApp` has `show_error(text: impl Into<String>)`
    and `show_warning(text: impl Into<String>)` methods; each sets
    `self.status` to the appropriate `StatusMessage` AND pushes a single-line
    `ContentBlock::Text` block with the styled prefix (`✗` / `⚠`) to
    `self.scroll`
  - **Files** — `crates/grove-cli/src/tui/transcript.rs`

- [x] **Replace all error/warning call sites with helpers**
  - **Delivers** — all errors and warnings get durable transcript records
  - **Done when** — every `self.status = Some(StatusMessage::error(...))` is
    replaced with `self.show_error(...)` and every
    `self.status = Some(StatusMessage::warning(...))` is replaced with
    `self.show_warning(...)`; all 41 sites migrated; no direct
    `StatusMessage::error` or `StatusMessage::warning` assignments remain in
    transcript.rs
  - **Files** — `crates/grove-cli/src/tui/transcript.rs`
