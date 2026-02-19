---
status: working
purpose: "Design session: unified process management for graft-as-library consumption by grove"
---

# Unified Process Management Design

## Context

Now that both graft and grove are Rust crates in the same workspace, grove can consume graft-engine as a library rather than shelling out to the graft binary. This session explores what that integration should look like, particularly for long-running and externally-launched processes.

## Current State

Grove shells out to the graft binary (or `uv run python -m graft`) to execute commands, and separately implements its own `sh -c` subprocess management for state queries. Graft-engine also has its own `sh -c` execution that blocks until completion with no streaming. There are effectively three independent subprocess execution paths with no shared infrastructure.

## Design

Three pieces, layered bottom-up:

### 1. ProcessHandle

A shared abstraction in graft-common that wraps subprocess execution. Replaces all three current execution paths with a single model that provides:

- **Streaming output** — events emitted as lines arrive, not buffered until exit
- **Cancellation** — callers can kill the subprocess
- **Log capture** — output is tee'd to a log file so external observers can read it

### 2. ProcessRegistry

A global, trait-backed registry. When a ProcessHandle spawns a subprocess, it registers; on exit or drop, it deregisters.

Key design decisions:

- **Global scope** — the registry is not scoped to a grove workspace. A graft repo may appear in multiple grove workspaces, and processes may be launched outside of grove entirely. The registry records which repo a process belongs to; consumers (grove, graft-cli) filter by whatever scope they care about.
- **Trait-backed** — the interface is clean and abstract. The default implementation uses the filesystem (`~/.cache/graft/processes/` or similar). This can be swapped to a database or network service in the future if distributed processing is needed.
- **Stale entry pruning** — listing active processes checks PID liveness and prunes entries for dead processes that didn't clean up (crashes, kills).

### 3. Unified Execution Model

There is one way to run things. The distinction between a "command" and a "state query" is about what happens with the output after execution, not how the subprocess is managed:

- **Commands**: output is streamed to the user (terminal or TUI)
- **State queries**: output is parsed as JSON and cached by commit hash

Both go through ProcessHandle. Both appear in the registry. Both are observable and cancellable.

## Consumer Patterns

**graft-cli** — calls graft-engine functions directly. For simple commands, uses a convenience wrapper that spawns and waits (synchronous). For `graft run`, streams output to the terminal. `graft ps` lists all registered processes globally or filtered by repo.

**grove-cli** — adds graft-engine as a library dependency. Calls it directly for config parsing, state queries, and command execution. Bridges ProcessHandle events into the TUI's event stream. Filters the global registry by the repos in its workspace to show relevant running processes. Receives completion events for state queries asynchronously — no blocking.

**External visibility** — a user runs `graft run long-test` in a terminal, then opens grove. Grove sees the running process in the registry, can tail its log file, and shows it in a running processes view.

## What Goes Away

- `find_graft_command()` in grove-cli — no longer searches for a graft binary on PATH
- Grove's independent `sh -c` state query execution — uses graft-engine's execution (with caching for free)
- Graft-engine's blocking `.output()` pattern — replaced by ProcessHandle's streaming model with a synchronous convenience wrapper on top

## Sources

- `crates/grove-cli/src/tui/command_exec.rs` — current grove subprocess spawning
- `crates/grove-cli/src/tui/repo_detail.rs` — current grove state query execution
- `crates/graft-engine/src/command.rs` — current graft command execution
- `crates/graft-engine/src/state.rs` — current graft state query execution
- `crates/graft-common/` — shared layer where ProcessHandle and ProcessRegistry would live
