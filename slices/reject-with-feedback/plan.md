---
status: draft
created: 2026-02-26
depends_on: [workflow-checkpoints, grove-checkpoint-ui]
---

# Capture human feedback on checkpoint rejection and inject into next session

## Story

When a human rejects a checkpoint, Claude has no idea why. The next `implement-verified`
run starts with no memory of the rejection reason — only the raw verify.json failures
(which passed, since the checkpoint is only written on verify success). The human must
manually explain the issues, or Claude repeats the same mistakes. This slice makes
`reject` capture a feedback message that is stored in `checkpoint.json` and
automatically injected into the next Claude session via `resume.sh`.

## Approach

Modify `reject.sh` to accept an optional positional `feedback` argument. When provided,
it is stored alongside `phase: "rejected"` as a `feedback` field in `checkpoint.json`.

Modify `resume.sh` to check `checkpoint.json` for a `feedback` field when building the
resume prompt — injecting it before the verify failure summary:

```
The human rejected the previous implementation with this feedback:

<feedback>

Please address this feedback in your next changes.
```

Update `graft.yaml` to add an optional `feedback` string arg to the `reject` command.

Update the grove approval overlay: the `r` key opens a text input prompt (using the
existing `argument_input` overlay pattern) where the human types feedback before the
reject command executes. Pressing Esc at the input skips feedback and rejects without
it (backward compatible).

## Acceptance Criteria

- `graft run software-factory:reject "feedback text"` writes
  `{phase: "rejected", feedback: "feedback text"}` to `checkpoint.json`
- `graft run software-factory:reject` (no feedback) writes `{phase: "rejected"}` with
  no `feedback` field — backward compatible
- `resume.sh` injects `feedback` from `checkpoint.json` when present, before any verify
  failure context
- The feedback injection is skipped when `checkpoint.json` has no `feedback` field, or
  when `checkpoint.json` does not have `phase: "rejected"`
- Grove's `r` key in the approval overlay opens a text input prompt before executing
  reject; submitting calls `reject` with the feedback text; Esc calls `reject` without
  feedback
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [ ] **Modify `reject.sh` and `resume.sh` to support feedback capture and injection**
  - **Delivers** — CLI-level feedback capture and injection into Claude sessions
  - **Done when** — `reject.sh` accepts an optional positional arg; when provided, merges
    it into `checkpoint.json` via `jq '. + {phase:"rejected", feedback:$fb}'` using
    `.tmp` + rename; when absent, writes `{phase:"rejected"}` as before; `resume.sh`
    reads `checkpoint.json`, checks for `feedback` field when `phase == "rejected"`, and
    prepends the formatted feedback section to the resume prompt before any verify/diagnose
    context; `graft.yaml` adds optional `feedback` string arg to `reject` command;
    manual test: reject with feedback, inspect `checkpoint.json`, then run
    `resume slices/<slug>` and confirm Claude receives the feedback in its prompt
  - **Files** — `.graft/software-factory/scripts/reject.sh`,
    `.graft/software-factory/scripts/resume.sh`,
    `.graft/software-factory/graft.yaml`

- [ ] **Add feedback text input to grove approval overlay**
  - **Delivers** — grove users can type rejection feedback before the reject command runs
  - **Done when** — `handle_key_approval_overlay()` in `overlays.rs`: pressing `r` sets
    `self.argument_input = Some(ArgumentInputState { prompt: "Rejection feedback (Esc
    to skip):", ... })` WITHOUT clearing `self.approval_overlay` — the overlay stays
    `Some` while argument_input is active so it remains visible underneath; the
    `argument_input` guard at the top of `handle_key()` fires BEFORE the
    `approval_overlay` guard, so keystrokes go to the text input while it is active;
    when the argument input completes (Enter), calls
    `execute_command_with_args("software-factory:reject", vec![feedback_text])`, then
    sets `self.approval_overlay = None` and `state_loaded = false`; when the argument
    input is dismissed (Esc), calls `execute_command_with_args("software-factory:reject",
    vec![])`, then sets `self.approval_overlay = None`; follows the existing
    `argument_input` guard pattern in `handle_key()` in `mod.rs`; a unit test asserts
    the overlay stays `Some` while argument_input is set, and is cleared on completion
  - **Files** — `crates/grove-cli/src/tui/overlays.rs`,
    `crates/grove-cli/src/tui/mod.rs`
