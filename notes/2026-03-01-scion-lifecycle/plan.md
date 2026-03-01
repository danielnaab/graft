---
status: working
purpose: "Implementation plan for scion lifecycle — task tracking for Ralph loop"
---

# Scion Lifecycle Implementation Plan

Implementing parallel workstream management via git worktrees: `graft scion create`,
`list`, `fuse`, `prune` with composable lifecycle hooks. Four vertical slices in
dependency order, each followed by a whole-slice critique pass.

## How to use this plan

Each task is a self-contained unit of work. Read the listed files, implement the
capability, verify, and mark complete. Tasks are ordered for incremental migration —
each task leaves all crates compiling and tests passing.

Key constraints:
- **Spec-first**: where a task includes a spec update, do the spec update first.
- **Test-driven**: write tests alongside the implementation, not as an afterthought.
- **Follow existing patterns**: match the API style in the files you're modifying.
- **Minimal changes**: only modify what the task requires.

## Design references

- `notes/2026-03-01-shoot-lifecycle-design.md` — scion lifecycle design (vocabulary,
  commands, hooks, composition, failure semantics, hook environment)
- `slices/scion-worktree-primitives/plan.md` — slice 1 detailed plan
- `slices/scion-config-create-prune/plan.md` — slice 2 detailed plan
- `slices/scion-list/plan.md` — slice 3 detailed plan
- `slices/scion-hook-composition-and-fuse/plan.md` — slice 4 detailed plan

## Resolved design conflicts

(Record conflicts you discover and how you resolved them here)

## Design decisions made during implementation

(Record decisions here as you make them)

---

## Slice 1: Git Worktree Primitives

Pure library functions in `graft-common/src/git.rs`. No config, no CLI, no engine.

### Task 1.1: `WorktreeInfo` type and `git_worktree_list`
- [x] Add `WorktreeInfo { path, branch, head }` and `git_worktree_list(repo)` to `graft-common/src/git.rs`
- **Spec**: `slices/scion-worktree-primitives/plan.md` (step 1)
- **Code**: `crates/graft-common/src/git.rs`
- **Acceptance**:
  - `WorktreeInfo` struct with `path: PathBuf`, `branch: Option<String>`, `head: String`
  - `git_worktree_list(repo)` runs `git worktree list --porcelain`, parses output
  - Branch is `None` for detached HEAD worktrees
  - Unit test: `git init` + `git worktree add`, verify round-trip
  - `cargo test -p graft-common && cargo clippy -p graft-common -- -D warnings` passes

### Task 1.2: `git_worktree_add`
- [x] Add `git_worktree_add(repo, path, branch)` to `graft-common/src/git.rs`
- **Code**: `crates/graft-common/src/git.rs`
- **Acceptance**:
  - Runs `git worktree add <path> -b <branch>`, returns absolute worktree path
  - Returns `GitError` if worktree path or branch already exists
  - Test: create worktree, confirm it appears in `git_worktree_list`
  - `cargo test -p graft-common && cargo clippy -p graft-common -- -D warnings` passes

### Task 1.3: `git_worktree_remove` and `git_branch_delete`
- [x] Add removal primitives to `graft-common/src/git.rs`
- **Code**: `crates/graft-common/src/git.rs`
- **Acceptance**:
  - `git_worktree_remove(repo, path)` runs `git worktree remove <path> --force`
  - `git_branch_delete(repo, branch)` runs `git branch -D <branch>`
  - Non-existent worktree/branch returns `GitError`
  - Test: create then remove, confirm gone from list
  - `cargo test -p graft-common && cargo clippy -p graft-common -- -D warnings` passes

### Task 1.4: `git_ahead_behind`
- [x] Add `git_ahead_behind(repo, branch, base)` to `graft-common/src/git.rs`
- **Code**: `crates/graft-common/src/git.rs`
- **Acceptance**:
  - Runs `git rev-list --left-right --count <branch>...<base>`, returns `(usize, usize)`
  - Test: create diverging branches, verify counts
  - `cargo test -p graft-common && cargo clippy -p graft-common -- -D warnings` passes

### Task 1.C: Critique slice 1
- [x] Re-read ALL code added/modified in tasks 1.1–1.4. Evaluate against slice plan's acceptance criteria. Check API consistency with existing `git.rs` functions, error handling, test coverage, edge cases. Fix any concrete issues found, commit fixes separately.
- **Read**: `crates/graft-common/src/git.rs`, `slices/scion-worktree-primitives/plan.md`

---

## Slice 2: Scion Config, Create, and Prune

Spec update, domain types, config parsing, engine module, CLI commands.

### Task 2.1: Update spec — add `scions:` section to `graft-yaml-format.md`
- [x] Document the `scions:` top-level key with all four hook points
- **Spec**: `slices/scion-config-create-prune/plan.md` (step 1)
- **Code**: `docs/specifications/graft/graft-yaml-format.md`
- **Acceptance**:
  - `scions:` documented as optional top-level key
  - Each hook point (`on_create`, `pre_fuse`, `post_fuse`, `on_prune`) accepts string or list of strings
  - Command names resolve relative to the defining graft.yaml's scope
  - Examples included showing single command and list of commands
  - Consistent with existing spec style and terminology

### Task 2.2: `ScionHooks` type, `GraftConfig` field, and parser
- [x] Add `ScionHooks` to `domain.rs`, wire into `GraftConfig`, parse in `config.rs`
- **Spec**: `slices/scion-config-create-prune/plan.md` (steps 2–3)
- **Code**: `crates/graft-engine/src/domain.rs`, `crates/graft-engine/src/config.rs`
- **Acceptance**:
  - `ScionHooks { on_create, pre_fuse, post_fuse, on_prune }` — each `Option<Vec<String>>`
  - Derives: `Serialize, Deserialize, Clone, Debug, PartialEq, Eq`
  - Single string normalized to one-element vec during parsing
  - `GraftConfig` gains `scion_hooks: Option<ScionHooks>` field
  - `parse_graft_yaml` parses `scions:` YAML key
  - Tests: missing section, single command, list of commands, empty section
  - Existing config tests still pass
  - `cargo test -p graft-engine && cargo clippy -p graft-engine -- -D warnings` passes

### Task 2.3: Cross-validate hook command names
- [x] Add scion hook validation to `GraftConfig::validate()`
- **Code**: `crates/graft-engine/src/domain.rs`
- **Acceptance**:
  - `validate()` checks every command name in `scion_hooks` exists in `self.commands`
  - Returns `ConfigValidation` error with field path on mismatch
  - Test: invalid hook name rejected, valid name passes, no hooks passes
  - `cargo test -p graft-engine && cargo clippy -p graft-engine -- -D warnings` passes

### Task 2.4: `From<GitError> for GraftError` error bridging
- [x] Add error conversion impl to `graft-engine/src/error.rs`
- **Code**: `crates/graft-engine/src/error.rs`
- **Acceptance**:
  - `impl From<graft_common::GitError> for GraftError` maps to `GraftError::Git(String)`
  - Scion engine functions can use `?` on git primitive results
  - `cargo test -p graft-engine && cargo clippy -p graft-engine -- -D warnings` passes

### Task 2.5: `scion_create` and `scion_prune` engine functions
- [x] New `graft-engine/src/scion.rs` module with create and prune operations
- **Spec**: `slices/scion-config-create-prune/plan.md` (step 4)
- **Code**: `crates/graft-engine/src/scion.rs`, `crates/graft-engine/src/lib.rs`
- **Acceptance**:
  - `scion_create(repo_path, name)` builds `.worktrees/<name>` + `feature/<name>`,
    calls `git_worktree_add`, returns worktree path
  - `scion_prune(repo_path, name)` calls `git_worktree_remove` then `git_branch_delete`
  - Module declared in `lib.rs` with pub use for `scion_create`, `scion_prune`
  - Integration test: create-then-prune round-trip in tempdir
  - `cargo test -p graft-engine && cargo clippy -p graft-engine -- -D warnings` passes

### Task 2.6: `graft scion create/prune` CLI commands
- [x] Add scion subcommand group to `graft-cli/src/main.rs`
- **Spec**: `slices/scion-config-create-prune/plan.md` (step 5)
- **Code**: `crates/graft-cli/src/main.rs`
- **Acceptance**:
  - `Scion` variant in `Commands` enum with `ScionCommands` sub-enum
  - `graft scion create <name>` calls `scion_create`, prints worktree path
  - `graft scion prune <name>` calls `scion_prune`, prints confirmation
  - Non-zero exit and descriptive error on failure
  - `--help` shows scion subcommand group
  - `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

### Task 2.C: Critique slice 2
- [x] Re-read ALL code added/modified in tasks 2.1–2.6. Evaluate: spec consistency with design doc, ScionHooks type design, parsing correctness, validation completeness, naming convention placement, error bridging, CLI ergonomics. Fix any concrete issues found.
- **Read**: `crates/graft-engine/src/domain.rs`, `crates/graft-engine/src/config.rs`,
  `crates/graft-engine/src/error.rs`, `crates/graft-engine/src/scion.rs`,
  `crates/graft-cli/src/main.rs`, `docs/specifications/graft/graft-yaml-format.md`,
  `slices/scion-config-create-prune/plan.md`

---

## Slice 3: Scion List

Git helpers for activity/dirty state, engine enumeration, CLI with table and JSON.

### Task 3.1: `git_last_commit_time` and `git_is_dirty`
- [x] Add git helpers for commit timestamps and dirty state
- **Spec**: `slices/scion-list/plan.md` (step 1)
- **Code**: `crates/graft-common/src/git.rs`
- **Acceptance**:
  - `git_last_commit_time(repo, branch)` runs `git log -1 --format=%ct <branch>`,
    returns Unix timestamp as `i64`; `GitError` if no commits
  - `git_is_dirty(worktree_path)` runs `git -C <path> status --porcelain`,
    returns `true` if non-empty
  - Unit tests for both
  - `cargo test -p graft-common && cargo clippy -p graft-common -- -D warnings` passes

### Task 3.2: `ScionInfo` and `scion_list` engine function
- [x] Add structured scion enumeration to `graft-engine/src/scion.rs`
- **Spec**: `slices/scion-list/plan.md` (step 2)
- **Code**: `crates/graft-engine/src/scion.rs`
- **Acceptance**:
  - `ScionInfo { name, branch, worktree_path, ahead, behind, last_commit_time, dirty }`
    with `Serialize` derive
  - `scion_list(repo_path)` calls `git_worktree_list`, filters to `.worktrees/` entries,
    extracts name from path, gathers metrics via git helpers
  - Integration test: create two scions, commit in one, verify list output
  - `cargo test -p graft-engine && cargo clippy -p graft-engine -- -D warnings` passes

### Task 3.3: `graft scion list` CLI command
- [x] Add list subcommand with text table and `--format json`
- **Spec**: `slices/scion-list/plan.md` (step 3)
- **Code**: `crates/graft-cli/src/main.rs`
- **Acceptance**:
  - `graft scion list` prints formatted table: name, ahead/behind, relative time, dirty
  - `--format json` outputs `serde_json::to_string_pretty` of `Vec<ScionInfo>`
  - Empty list: "No scions" (text) or `[]` (JSON)
  - Relative time formatting: "2m ago", "1h ago", "3d ago"
  - `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

### Task 3.C: Critique slice 3
- [x] Re-read ALL code added/modified in tasks 3.1–3.3. Evaluate: git helper correctness, ScionInfo completeness, filtering logic, table formatting, JSON output, edge cases (no scions, brand-new scion with no commits). Fix any concrete issues found.
- **Read**: `crates/graft-common/src/git.rs`, `crates/graft-engine/src/scion.rs`,
  `crates/graft-cli/src/main.rs`, `slices/scion-list/plan.md`

---

## Slice 4: Hook Composition and Fuse

Hook resolution, execution, create/prune retrofit, fuse command with merge workflow.

### Task 4.1: `git_merge_to_ref` and `git_fast_forward`
- [ ] Add merge plumbing and branch advancement to `graft-common/src/git.rs`
- **Spec**: `slices/scion-hook-composition-and-fuse/plan.md` (step 1)
- **Code**: `crates/graft-common/src/git.rs`
- **Acceptance**:
  - `git_merge_to_ref(repo, source, target, ref_name)` uses
    `git merge-tree --write-tree <target> <source>` to compute tree; checks exit code
    for conflicts (returns `GitError` with details); creates merge commit via
    `git commit-tree`; stores at `ref_name` via `git update-ref`
  - `git_fast_forward(repo, branch, commit)` runs
    `git update-ref refs/heads/<branch> <commit>`
  - Tests: clean merge produces reachable commit, conflicting merge returns error,
    fast-forward advances branch
  - `cargo test -p graft-common && cargo clippy -p graft-common -- -D warnings` passes

### Task 4.2: `ResolvedHook` and `resolve_hook_chain`
- [ ] Hook resolution across dependency scopes with event-aware working directories
- **Spec**: `slices/scion-hook-composition-and-fuse/plan.md` (step 2),
  `notes/2026-03-01-shoot-lifecycle-design.md` (resolution algorithm, hook environment)
- **Code**: `crates/graft-engine/src/scion.rs`
- **Acceptance**:
  - `ResolvedHook { command_name, namespace, working_dir }` struct
  - `resolve_hook_chain(event, config, dep_configs, scion_worktree, project_root)`
    iterates deps in declaration order, collects hooks qualified to namespace,
    appends project hooks unqualified
  - Working dir: worktree for `on_create`/`pre_fuse`/`on_prune`, project root for `post_fuse`
  - Unit tests: no hooks, project-only, dep-only, mixed, correct working_dir per event
  - `cargo test -p graft-engine && cargo clippy -p graft-engine -- -D warnings` passes

### Task 4.3: `execute_hook_chain`
- [ ] Sequential hook execution with fail-fast and scion env vars
- **Spec**: `slices/scion-hook-composition-and-fuse/plan.md` (step 3),
  `notes/2026-03-01-shoot-lifecycle-design.md` (failure semantics)
- **Code**: `crates/graft-engine/src/scion.rs`
- **Acceptance**:
  - Resolves each hook via existing `execute_command_by_name` infrastructure
  - Injects `GRAFT_SCION_NAME`, `GRAFT_SCION_BRANCH`, `GRAFT_SCION_WORKTREE` env vars
  - Overrides working directory from `ResolvedHook.working_dir`
  - Returns `Ok(completed)` or `Err { failed_hook, completed_hooks, error }`
  - Tests: all succeed, middle hook fails (first completed, third not attempted)
  - `cargo test -p graft-engine && cargo clippy -p graft-engine -- -D warnings` passes

### Task 4.4: Retrofit hooks into `scion_create` and `scion_prune`
- [ ] Wire hook execution into create/prune with event-specific rollback
- **Spec**: `slices/scion-hook-composition-and-fuse/plan.md` (step 4),
  `notes/2026-03-01-shoot-lifecycle-design.md` (rollback table)
- **Code**: `crates/graft-engine/src/scion.rs`
- **Acceptance**:
  - `scion_create` now accepts config + dep_configs; runs `on_create` chain after
    `git_worktree_add`; on hook failure, removes worktree + branch (rollback)
  - `scion_prune` now accepts config + dep_configs; runs `on_prune` chain before
    `git_worktree_remove`; on hook failure, leaves worktree intact
  - CLI updated to pass config through
  - Integration tests verify rollback with failing hook script
  - `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

### Task 4.5: `scion_fuse` engine function
- [ ] Merge-to-main with hook gates, already-merged detection, cleanup
- **Spec**: `slices/scion-hook-composition-and-fuse/plan.md` (step 5),
  `notes/2026-03-01-shoot-lifecycle-design.md` (fuse sequence)
- **Code**: `crates/graft-engine/src/scion.rs`
- **Acceptance**:
  - `scion_fuse(repo, name, config, dep_configs)` sequence:
    (1) `git_merge_to_ref` feature→main at temp ref
    (2) `pre_fuse` chain — failure deletes temp ref
    (3) `git_fast_forward` main to merge commit
    (4) `post_fuse` chain — failure leaves scion intact
    (5) `git_worktree_remove` + `git_branch_delete`
  - Already-merged: if merge is no-op, skip to step 4
  - Merge conflicts: return error, no cleanup needed
  - Integration test covers full fuse lifecycle
  - `cargo test -p graft-engine && cargo clippy -p graft-engine -- -D warnings` passes

### Task 4.6: `graft scion fuse <name>` CLI command
- [ ] Add fuse subcommand
- **Spec**: `slices/scion-hook-composition-and-fuse/plan.md` (step 6)
- **Code**: `crates/graft-cli/src/main.rs`
- **Acceptance**:
  - `graft scion fuse <name>` calls `scion_fuse`
  - Prints merge result and cleanup confirmation on success
  - Prints hook failure details (which hook, what completed) on error
  - Non-zero exit on failure; `--help` shows usage
  - `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

### Task 4.C: Critique slice 4
- [ ] Re-read ALL code added/modified in tasks 4.1–4.6. Evaluate: merge plumbing correctness, hook resolution order, env var injection, rollback behavior, fuse state machine, already-merged detection, error reporting quality, CLI output. Fix any concrete issues found.
- **Read**: `crates/graft-common/src/git.rs`, `crates/graft-engine/src/scion.rs`,
  `crates/graft-cli/src/main.rs`, `slices/scion-hook-composition-and-fuse/plan.md`,
  `notes/2026-03-01-shoot-lifecycle-design.md`

---

## Final: Mark slice plans done

### Task F.1: Update slice plan status
- [ ] Set `status: done` in all 4 slice plan frontmatters. Run full verification: `cargo fmt --check && cargo clippy -- -D warnings && cargo test`. Commit.
- **Files**: `slices/scion-worktree-primitives/plan.md`, `slices/scion-config-create-prune/plan.md`,
  `slices/scion-list/plan.md`, `slices/scion-hook-composition-and-fuse/plan.md`
