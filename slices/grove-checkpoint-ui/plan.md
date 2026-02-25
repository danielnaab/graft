---
status: accepted
created: 2026-02-24
depends_on: [workflow-checkpoints]
---

# Prominent checkpoint display and approval overlay in grove

## Story

After `implement-verified` succeeds, `checkpoint.json` appears in grove's Run
State section as a plain JSON entry indistinguishable from other state files.
This slice makes checkpoints visually prominent and actionable: they render with
a `!` prefix in yellow/bold, show the human-readable `.message` instead of raw
JSON, and pressing Enter opens an approval overlay with `[a]pprove / [r]eject`.

## Approach

Three targeted changes to the grove TUI:

1. **Rendering**: `append_run_state_section_mapped()` in `repo_detail.rs` detects
   entries named `checkpoint` with `phase == "awaiting-review"` and renders them
   with a `!` prefix, Yellow/Bold style, and the `.message` field as the summary
   line. All other entries are unaffected.

2. **Overlay**: A new `ApprovalOverlayState` struct holds the checkpoint context.
   `App` gains an `approval_overlay: Option<ApprovalOverlayState>` field.
   `handle_key_approval_overlay()` in `overlays.rs` handles `a` (approve), `r`
   (reject), and `Esc`. `render_approval_overlay()` draws a centered 52×9 modal.
   Enter on a pending checkpoint item in the detail view sets the overlay instead
   of toggling expansion.

3. **Hint bar**: When the cursor is on a pending checkpoint run-state entry, the
   hint bar shows `Enter: review  a: approve  r: reject` instead of the default
   expand/collapse hint.

`approve` and `reject` run as graft commands via the existing `execute_command_with_args`
path; the CommandOutput view shows their output; run-state reloads on exit.

## Acceptance Criteria

- Checkpoint entry renders with `!` prefix, Yellow/Bold style, and `.message` as
  the summary (not raw JSON compact); the `(← ...)` producer annotation is still shown
- Non-checkpoint run-state entries are visually unchanged
- The hint bar shows `Enter: review  a: approve  r: reject` when cursor is on a
  pending checkpoint item
- Enter on a pending checkpoint opens the approval overlay; Enter on any other
  run-state item retains existing expand/collapse behavior
- `a` in the overlay runs `software-factory:approve <slice>`, pushes CommandOutput
  view to show the result, and reloads run-state on return
- `r` in the overlay runs `software-factory:reject <slice>` and reloads run-state
- `Esc` in the overlay dismisses without action
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [ ] **Add ApprovalOverlayState to App; render checkpoint entries distinctly in repo_detail.rs**
  - **Delivers** — pending checkpoints are visually prominent and distinguishable
    from other run-state entries
  - **Done when** — `App` struct gains `approval_overlay: Option<ApprovalOverlayState>`
    initialized to `None`, where `ApprovalOverlayState` holds `{slice: String,
    sequence: String, message: String}`; `append_run_state_section_mapped()` checks
    whether an entry's name is `"checkpoint"` and its JSON `phase` field equals
    `"awaiting-review"`; such entries render the name with a `!` prefix and
    Yellow/Bold style, and use the JSON `message` field (or `"Awaiting review"` as
    fallback) as the collapsed summary instead of the generic JSON compact string;
    all other entries use existing rendering unchanged; a unit test asserts that a
    `checkpoint` entry with `phase: awaiting-review` renders the `!` prefix and
    that a `checkpoint` entry with any other phase renders normally
  - **Files** — `crates/grove-cli/src/tui/mod.rs`,
    `crates/grove-cli/src/tui/repo_detail.rs`,
    `crates/grove-cli/src/tui/tests.rs`

- [ ] **Add approval overlay handlers and rendering in overlays.rs; wire Enter in repo_detail.rs**
  - **Delivers** — pressing Enter on a pending checkpoint opens the approval modal
    and `a`/`r` run the approve/reject commands
  - **Done when** — `render_approval_overlay()` in `overlays.rs` draws a centered
    52×9 modal following the `render_stop_confirmation_dialog` pattern:
    ```
    ┌─ Review Checkpoint ─────────────────────────────────┐
    │                                                      │
    │  implement-verified · slices/<slug>                 │
    │  Step complete. Verify passed.                      │
    │                                                      │
    │  [a] Approve — advance to next step                 │
    │  [r] Reject  — re-implement this step               │
    │  [Esc] Cancel                                       │
    └──────────────────────────────────────────────────────┘
    ```
    `handle_key_approval_overlay()` handles: `a` → calls
    `self.execute_command_with_args("software-factory:approve", &[slice.clone()])`
    and clears `approval_overlay`; `r` → same with `reject`; `Esc` → clears
    `approval_overlay`; in `handle_key_repo_detail()`, the Enter branch for a
    `RunState(idx)` item checks `is_pending_checkpoint(idx)` — if true, populates
    `self.approval_overlay` instead of toggling expand; `render()` calls
    `render_approval_overlay()` when `approval_overlay.is_some()` (drawn on top,
    same layer as stop-confirmation dialog)
  - **Files** — `crates/grove-cli/src/tui/overlays.rs`,
    `crates/grove-cli/src/tui/repo_detail.rs`

- [ ] **Update hint_bar.rs for pending checkpoint items**
  - **Delivers** — the hint bar communicates what `Enter`, `a`, and `r` do when
    the cursor is on a checkpoint requiring review
  - **Done when** — `hint_bar.rs` detects when the focused `DetailItem` is a
    `RunState(idx)` whose entry is a pending checkpoint and renders
    `"Enter: review  a: approve  r: reject"` in place of the default
    expand/collapse hint; all other detail items show existing hints unchanged;
    a unit test (or the existing hint-bar tests) asserts the checkpoint-specific
    hint text appears for a pending checkpoint item
  - **Files** — `crates/grove-cli/src/tui/hint_bar.rs`,
    `crates/grove-cli/src/tui/tests.rs`
