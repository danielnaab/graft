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
   line. All other entries — including checkpoints with `phase: approved` or
   `phase: rejected` — use existing rendering unchanged.

2. **Overlay**: A new `ApprovalOverlayState` struct holds the checkpoint context
   (sequence name, slice, message). `App` gains an
   `approval_overlay: Option<ApprovalOverlayState>` field. Enter on a pending
   checkpoint in the detail view sets the overlay instead of toggling expansion.
   `handle_key_approval_overlay()` in `overlays.rs` handles `a` (approve), `r`
   (reject), and `Esc`. `render_approval_overlay()` draws a centered modal showing
   the sequence/slice context and `[a]`/`[r]`/`[Esc]` keys.

3. **Hint bar**: When the cursor is on a pending checkpoint run-state entry, the
   hint bar shows `Enter: review checkpoint` (not `Enter: expand`). The `a` and `r`
   keys are intentionally NOT shown in the hint bar — they only work inside the
   overlay, and the overlay renders its own key hints. Showing them in the detail
   view hint bar would imply they work there, which they do not.

`approve` and `reject` run as graft commands (no args) via the existing
`execute_command_with_args` path; the CommandOutput view shows their output;
run-state reloads on exit.

## Acceptance Criteria

- Checkpoint entry with `phase: awaiting-review` renders with `!` prefix, Yellow/Bold
  style, and `.message` as the summary (not raw JSON compact); the `(← ...)` producer
  annotation is still shown
- Checkpoints with any other phase (`approved`, `rejected`) render as normal JSON
  entries — visually identical to any other run-state file
- Non-checkpoint run-state entries are visually unchanged
- The hint bar shows `Enter: review checkpoint` when cursor is on a pending
  checkpoint item; all other items show their existing hints
- Enter on a pending checkpoint opens the approval overlay; Enter on any other
  run-state item retains existing expand/collapse behavior
- `a` in the overlay runs `software-factory:approve` (no args), pushes CommandOutput
  view to show the result, clears the overlay, and reloads run-state on return
- `r` in the overlay runs `software-factory:reject` (no args), same flow
- `Esc` in the overlay dismisses without action
- After approve, the checkpoint entry transitions from `!` pending to normal JSON
  showing `phase: approved`
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [ ] **Add ApprovalOverlayState to App; render pending checkpoints distinctly in repo_detail.rs**
  - **Delivers** — pending checkpoints are visually prominent and distinguishable
    from other run-state entries
  - **Done when** — `App` struct gains `approval_overlay: Option<ApprovalOverlayState>`
    initialized to `None`, where `ApprovalOverlayState` holds `{sequence: String,
    slice: String, message: String}` (slice is read from `checkpoint.json`'s `args`
    field); `append_run_state_section_mapped()` checks whether an entry's name is
    `"checkpoint"` and its JSON `phase` field equals `"awaiting-review"`; such
    entries render the name with a `!` prefix and Yellow/Bold style, and use the
    JSON `message` field (or `"Awaiting review"` as fallback) as the collapsed
    summary; all other entries — including `phase: approved` checkpoints — use
    existing rendering unchanged; a unit test asserts the `!` prefix appears for
    `phase: awaiting-review` and that `phase: approved` renders without `!`
  - **Files** — `crates/grove-cli/src/tui/mod.rs`,
    `crates/grove-cli/src/tui/repo_detail.rs`,
    `crates/grove-cli/src/tui/tests.rs`

- [ ] **Add approval overlay handlers and rendering in overlays.rs; wire Enter in repo_detail.rs**
  - **Delivers** — pressing Enter on a pending checkpoint opens the approval modal
    and `a`/`r` run the approve/reject commands
  - **Done when** — `render_approval_overlay()` in `overlays.rs` draws a centered
    modal following the `render_stop_confirmation_dialog` pattern:
    ```
    ┌─ Review Checkpoint ─────────────────────────────────┐
    │                                                      │
    │  implement-verified · slices/<slug>                 │
    │  Sequence complete. Verify passed.                  │
    │                                                      │
    │  [a] Approve — advance to next step                 │
    │  [r] Reject  — re-implement this step               │
    │  [Esc] Cancel                                       │
    └──────────────────────────────────────────────────────┘
    ```
    `handle_key_approval_overlay()` handles: `a` → calls
    `self.execute_command_with_args("software-factory:approve", &[])` and clears
    `approval_overlay`; `r` → calls
    `self.execute_command_with_args("software-factory:reject", &[])` and clears
    `approval_overlay`; `Esc` → clears `approval_overlay`; in
    `handle_key_repo_detail()`, the Enter branch for a `RunState(idx)` item checks
    `is_pending_checkpoint(idx)` — if true, reads the `sequence` and `args.slice`
    fields from the entry's JSON to populate `ApprovalOverlayState`, then sets
    `self.approval_overlay = Some(...)` instead of toggling expand; `render()` calls
    `render_approval_overlay()` when `approval_overlay.is_some()` (drawn on top,
    same layer as stop-confirmation dialog)
  - **Files** — `crates/grove-cli/src/tui/overlays.rs`,
    `crates/grove-cli/src/tui/repo_detail.rs`

- [ ] **Update hint_bar.rs for pending checkpoint items**
  - **Delivers** — the hint bar communicates that Enter opens a review modal for
    pending checkpoints, without suggesting keys that only work inside the overlay
  - **Done when** — `hint_bar.rs` detects when the focused `DetailItem` is a
    `RunState(idx)` whose entry is a pending checkpoint (`name == "checkpoint"` and
    `phase == "awaiting-review"`) and renders `"Enter: review checkpoint"` in place
    of the default expand/collapse hint; the `a` and `r` keys are shown only in the
    overlay, not here; all other detail items show existing hints unchanged; a unit
    test asserts the checkpoint-specific hint renders for a pending checkpoint item
    and the default hint renders for a non-pending checkpoint item
  - **Files** — `crates/grove-cli/src/tui/hint_bar.rs`,
    `crates/grove-cli/src/tui/tests.rs`
