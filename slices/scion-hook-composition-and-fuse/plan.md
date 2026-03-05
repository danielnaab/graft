---
status: done
created: 2026-03-01
depends_on:
  - scion-config-create-prune
---

# Scion hook composition and fuse command

## Story

Scion create and prune (from the prior slice) perform only git operations. The
lifecycle design specifies composable hooks at each lifecycle point — dependencies'
hooks run first, then the project's, with fail-fast semantics and event-specific
rollback. This slice adds the hook resolution and execution engine, retrofits hooks
into create and prune, and implements `graft scion fuse` which is inherently
hook-dependent (pre_fuse gate, post_fuse notification).

## Approach

Four areas of work:

1. **Hook resolution** — `resolve_hook_chain(event, config, dependencies)` walks
   dependencies in declaration order, collects their `scions.<event>` hooks qualified
   to the dependency's namespace, then appends the project's own hooks. Returns an
   ordered `Vec<ResolvedHook>` where each entry carries the command name, namespace,
   and working directory. Working directory is event-dependent per the design doc:
   worktree for `on_create`, `pre_fuse`, `on_prune`; project root for `post_fuse`.

2. **Hook execution** — `execute_hook_chain(chain, scion_env)` runs hooks
   sequentially using the existing `execute_command` / `execute_command_by_name`
   infrastructure from `graft-engine/src/command.rs` (not raw `ProcessConfig` — hooks
   are command names that resolve to `commands:` entries with their own `run`, `env`,
   `working_dir`, etc.). Each hook receives `GRAFT_SCION_NAME`,
   `GRAFT_SCION_BRANCH`, and `GRAFT_SCION_WORKTREE` as additional env vars. On
   failure, returns which hook failed and which hooks already completed.

3. **Retrofit create/prune** — `scion_create` gains: run `on_create` chain after
   worktree add; on hook failure, remove worktree (rollback) and propagate error.
   `scion_prune` gains: run `on_prune` chain before worktree removal; on hook
   failure, leave worktree intact and propagate error.

4. **Fuse command** — new `scion_fuse(repo, name)` engine function and
   `graft scion fuse <name>` CLI command. Sequence: merge feature branch to temp ref,
   run `pre_fuse` chain (rollback: discard temp ref), fast-forward main, run
   `post_fuse` chain (no rollback — main already moved), remove worktree + branch.

Two new git helpers support fuse: `git_merge_to_ref` (merge into a temporary ref
without moving HEAD) and `git_fast_forward` (advance a branch to a commit). Merge
conflicts during fuse are detected and reported as errors — no automatic resolution.

## Acceptance Criteria

- `resolve_hook_chain` collects hooks from dependencies then project in declaration order
- `execute_hook_chain` runs hooks sequentially with scion env vars
- Hook failure stops the chain and reports which hook failed and which completed
- `scion_create` runs `on_create` hooks; rolls back worktree on hook failure
- `scion_prune` runs `on_prune` hooks; preserves worktree on hook failure
- `scion_fuse` performs: temp-ref merge, `pre_fuse` gate, fast-forward main,
  `post_fuse`, worktree + branch cleanup
- `pre_fuse` failure discards the temp ref and exits non-zero
- `post_fuse` failure leaves the scion intact (worktree not removed) and exits
  non-zero; re-running fuse detects "already merged" and retries from `post_fuse`
- `graft scion fuse <name>` CLI command exists with `--help`
- `git_merge_to_ref(repo, source, target, ref_name)` merges without moving HEAD
- `git_fast_forward(repo, branch, commit)` advances a branch ref
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [x] **Add `git_merge_to_ref` and `git_fast_forward` to `graft-common/src/git.rs`**
  - **Delivers** — git plumbing for non-destructive merge and branch advancement
  - **Done when** — `git_merge_to_ref(repo, source, target, ref_name)` creates a
    merge commit of `source` into `target` stored at `ref_name` without touching
    HEAD or the working tree; uses `git merge-tree --write-tree <target> <source>`
    (available since git 2.38) to compute the tree, checks exit code for conflicts
    (returns `GitError` with conflict details if merge fails), then `git commit-tree`
    to create the merge commit, then `git update-ref` to store it at `ref_name`;
    `git_fast_forward(repo, branch, commit)` runs `git update-ref
    refs/heads/<branch> <commit>`; tests verify: clean merge produces reachable
    commit, conflicting merge returns error, branch ref advances after fast-forward
  - **Files** — `crates/graft-common/src/git.rs`

- [x] **Add `ResolvedHook` type and `resolve_hook_chain` to `graft-engine/src/scion.rs`**
  - **Delivers** — hook resolution across dependency scopes with event-aware
    working directories
  - **Done when** — `ResolvedHook { command_name, namespace, working_dir }` struct
    exists; `resolve_hook_chain(event, config, dep_configs, scion_worktree,
    project_root)` iterates dependencies in declaration order, collects
    `scions.<event>` hooks qualified to the dependency namespace, then appends the
    project's hooks unqualified; sets `working_dir` to `scion_worktree` for
    `on_create`, `pre_fuse`, `on_prune` and `project_root` for `post_fuse`;
    returns `Vec<ResolvedHook>`; unit tests cover: no hooks defined, project-only
    hooks, dependency-only hooks, mixed, and correct working_dir per event
  - **Files** — `crates/graft-engine/src/scion.rs`

- [x] **Add `execute_hook_chain` to `graft-engine/src/scion.rs`**
  - **Delivers** — sequential hook execution with fail-fast semantics
  - **Done when** — `execute_hook_chain(chain, scion_env)` resolves each hook's
    command name via the existing `execute_command_by_name` infrastructure (from
    `graft-engine/src/command.rs`), injecting `GRAFT_SCION_NAME`,
    `GRAFT_SCION_BRANCH`, `GRAFT_SCION_WORKTREE` as additional env vars and
    overriding the working directory from `ResolvedHook.working_dir`; returns
    `Ok(completed)` on full success or `Err { failed_hook, completed_hooks, error }`
    on failure; tests verify: all succeed, middle hook fails (first completed,
    third not attempted)
  - **Files** — `crates/graft-engine/src/scion.rs`

- [x] **Retrofit hooks into `scion_create` and `scion_prune`**
  - **Delivers** — lifecycle hooks on create and prune with rollback
  - **Done when** — `scion_create` resolves and executes `on_create` chain after
    `git_worktree_add`; on hook failure, calls `git_worktree_remove` +
    `git_branch_delete` (rollback) then returns the hook error; `scion_prune`
    resolves and executes `on_prune` chain before `git_worktree_remove`; on hook
    failure, leaves worktree intact and returns the hook error; integration tests
    verify rollback behavior with a failing hook script
  - **Files** — `crates/graft-engine/src/scion.rs`

- [x] **Implement `scion_fuse` in `graft-engine/src/scion.rs`**
  - **Delivers** — merge-to-main with hook gates and cleanup
  - **Done when** — `scion_fuse(repo, name, config, dep_configs)` performs:
    (1) `git_merge_to_ref` to merge `feature/<name>` into `main` at a temp ref;
    (2) resolve and execute `pre_fuse` chain — on failure, delete temp ref and
    return error; (3) `git_fast_forward` main to the merge commit; (4) resolve and
    execute `post_fuse` chain — on failure, leave scion intact (don't remove
    worktree) and return error; (5) `git_worktree_remove` + `git_branch_delete`;
    already-merged detection: if merge is a no-op (main already contains the
    branch), skip to step 4; integration test covers full fuse lifecycle
  - **Files** — `crates/graft-engine/src/scion.rs`

- [x] **Add `graft scion fuse <name>` CLI subcommand**
  - **Delivers** — user-facing merge command with lifecycle hooks
  - **Done when** — `graft scion fuse <name>` calls `scion_fuse`, prints merge
    result and cleanup confirmation on success; prints hook failure details
    (which hook, what completed) on error; exits non-zero on any failure; `--help`
    shows usage
  - **Files** — `crates/graft-cli/src/main.rs`
