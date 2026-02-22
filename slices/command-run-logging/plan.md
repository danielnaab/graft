---
status: in-progress
created: 2026-02-22
---

# Persist command run output and surface it in grove

## Story

When you run a command in grove (especially long-running ones like `implement`), the output disappears the moment you close the view. There's no way to review what happened, compare across runs, or debug a failure after the fact. This slice adds persistent run logging and a UI to browse past runs — closing the feedback loop so the plan→implement workflow has an audit trail.

## Approach

The process infrastructure already supports tee'ing output to a log file via `ProcessConfig.log_path` — it just isn't wired up. The plan is to: compute a timestamped log path per run, pass it through grove's command_exec, write a small metadata sidecar (command name, args, exit code, timing), then add a discovery module and UI section so past runs are browsable in repo detail. The log files are plain text (one line per output line); the metadata is JSON.

Storage layout mirrors the state cache: `~/.cache/graft/{workspace-hash}/{repo}/runs/{timestamp}-{command}.log` with a `.meta.json` companion.

## Acceptance Criteria

- Command output from grove is persisted to a log file that survives closing the output view
- A metadata file records command name, args, start/end time, and exit code
- The repo detail view shows a "Recent Runs" section listing past command runs
- Selecting a run in the list opens its log in the CommandOutput view (read-only)
- Runs are scoped per-repo, per-workspace (same cache hierarchy as state)
- `cargo fmt --check && cargo clippy -- -D warnings && cargo test` pass after each step

## Steps

- [x] **Add run log path helpers to graft-common**
  - **Delivers** — functions to compute run log paths and write/read run metadata
  - **Done when** — `get_run_log_dir(workspace, repo) -> PathBuf`, `RunMeta` struct with serde, `write_run_meta()` and `list_runs()` work with unit tests
  - **Files** — `crates/graft-common/src/runs.rs`, `crates/graft-common/src/lib.rs`

- [x] **Wire log_path in grove command_exec**
  - **Delivers** — every command spawned by grove writes output to a timestamped log file
  - **Done when** — `spawn_command` and `spawn_command_assembled` compute a log path from workspace/repo/command name, pass it to `ProcessConfig`, and after completion write a `RunMeta` sidecar; running a command in grove produces files in `~/.cache/graft/.../runs/`
  - **Files** — `crates/grove-cli/src/tui/command_exec.rs`, `crates/grove-cli/src/tui/mod.rs`

- [ ] **Add run discovery module to grove-cli**
  - **Delivers** — a module that scans the runs directory and returns a list of recent runs with metadata
  - **Done when** — `discover_runs(workspace, repo) -> Vec<RunMeta>` returns runs sorted newest-first, handles missing directory gracefully, tested
  - **Files** — `crates/grove-cli/src/runs.rs`, `crates/grove-cli/src/lib.rs`

- [ ] **Add Runs section to repo detail view**
  - **Delivers** — a "Recent Runs" section between State Queries and Commands showing past runs with timestamp, command name, and exit status
  - **Done when** — runs appear in the detail view, cursor can select them, detail_items includes a `Run(usize)` variant, runs are loaded lazily like state queries
  - **Files** — `crates/grove-cli/src/tui/repo_detail.rs`, `crates/grove-cli/src/tui/mod.rs`

- [ ] **Open run log in CommandOutput view**
  - **Delivers** — pressing Enter on a run loads its log file into the CommandOutput view for scrollable review
  - **Done when** — selecting a run opens output view with the log contents, header shows "Run: {command} ({timestamp})", exit code is shown, q returns to repo detail
  - **Files** — `crates/grove-cli/src/tui/repo_detail.rs`, `crates/grove-cli/src/tui/overlays.rs`
