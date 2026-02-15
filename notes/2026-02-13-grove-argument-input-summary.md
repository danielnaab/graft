---
status: deprecated
date: 2026-02-13
archived-reason: "Phase 1 shipped, stable"
---

# Grove Argument Input - Phase 1 Summary

Implemented cursor navigation, command preview, and parse validation for the argument input dialog.

## What Was Implemented

- **Cursor navigation**: arrow keys, Home/End, insert/delete at position
- **Command preview**: real-time display of parsed command (green=valid, red=error)
- **Parse error blocking**: invalid input prevents execution with clear feedback
- **`ArgumentInputState` struct**: encapsulated state replacing flat fields
- **Unicode-safe editing**: char indices instead of byte indices

## Metrics

| Metric | Before | After |
|--------|--------|-------|
| Test count | 8 | 15 (+7) |
| Editing features | 2 (type, delete) | 8 (type, delete, nav, preview, validate) |
| Dialog size | 60x7 | 70x9 |

## Known Limitations

- No horizontal scrolling (inputs >70 chars wrap awkwardly)
- No selection/copy/paste
- No undo (Backspace is destructive)

## Sources

- [Command execution spec](../docs/specifications/grove/command-execution.md)
- [Grove TUI source](../grove/src/tui.rs)
