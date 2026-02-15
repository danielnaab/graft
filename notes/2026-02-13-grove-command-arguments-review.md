---
status: deprecated
date: 2026-02-13
archived-reason: "Phase 1 improvements shipped"
---

# Grove Command Arguments - Review

Review of the argument input implementation with improvement roadmap.

## Assessment

Architecture is clean (follows modal UI pattern, proper separation). Test coverage is solid (8 unit + 1 integration). Minimal dependencies (only `shell-words`).

## Issues Identified and Resolved in Phase 1

- No cursor position control - fixed with arrow key navigation
- No visual feedback for parsing errors - fixed with real-time preview
- Silent parse error handling - fixed with error blocking on Enter
- Flat state management - fixed with `ArgumentInputState` struct

## Remaining Improvements (Future Phases)

**Phase 2** (enhanced editing):
- Delete key, Ctrl+U/W/K shortcuts
- Dynamic dialog width (75% of screen)
- Horizontal scrolling for long inputs

**Phase 3** (power user):
- Command history (up arrow to recall)
- Tab completion for file paths
- Clipboard support (Ctrl+V)

## Sources

- [Command execution spec](../docs/specifications/grove/command-execution.md)
- [Grove TUI source](../grove/src/tui.rs)
