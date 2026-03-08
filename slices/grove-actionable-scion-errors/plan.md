---
status: done
created: 2026-03-06
completed: 2026-03-06
completed_note: "Implemented during grove TUI buildout; scion_error_with_hint() adds contextual hints to attach/prune/fuse errors."
depends_on:
  - grove-durable-error-messages
---

# Add actionable hints to scion error messages in grove

## Story

When `:attach test` fails because the scion was created but never started, the
error message is "no active session for scion 'test'" — technically correct but
the user has to piece together that create succeeded, start failed (with a
transient error they may have missed), and therefore attach has nothing to
connect to. The same pattern applies to other scion errors: the messages state
the problem without suggesting next steps.

After this slice, grove appends TUI-specific hints to scion error messages so
users know what to try next.

## Approach

Enhance the error display at the grove caller level — in `cmd_attach` and
other scion handlers in `transcript.rs`. When the engine returns an error,
match on the error text and append a contextual hint. No engine-level changes.

### Hint mapping

| Engine error contains | Grove hint appended |
|----------------------|---------------------|
| `does not exist` | `Create it with :scion create <name>` |
| `no active session` | `Start it with :scion start <name>` |
| `session is still active` (from prune/fuse) | `Stop it first with :scion stop <name>` |

Hints are concatenated into the error string before passing to `show_error()`:
```rust
let hint = format!("{e}. Start it with :scion start {name}");
self.show_error(hint);
```
This produces a single-line error block in the transcript (matching the
durable-error-messages slice format). The hint is part of the error text,
not a separate block.

## Acceptance Criteria

- `:attach <name>` on nonexistent scion shows error + "Create it with :scion create <name>"
- `:attach <name>` on scion without session shows error + "Start it with :scion start <name>"
- `:scion prune <name>` with active session shows error + "Stop it first with :scion stop <name>"
- `:scion fuse <name>` with active session shows error + "Stop it first with :scion stop <name>"
- Engine errors that don't match any hint pattern display as-is (no regression)
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [x] **Add contextual hints to scion error displays in grove**
  - **Delivers** — actionable error messages for scion workflow failures
  - **Done when** — `cmd_attach`, `cmd_scion_prune`, and `cmd_scion_fuse` in
    `transcript.rs` enhance engine error messages with next-step hints based
    on error content matching; hints use grove command syntax (`:scion start`,
    etc.); errors without matching patterns display unchanged; uses
    `show_error()` from durable-error-messages slice
  - **Files** — `crates/grove-cli/src/tui/transcript.rs`
