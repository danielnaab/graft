---
title: "Exploration: Binary Architecture and Composition Strategy"
date: 2026-02-08
status: working
participants: ["human", "agent"]
tags: [exploration, architecture, composition, cli, grove, graft, monorepo]
related: ["2026-02-08-implementation-language-and-repo-structure.md"]
---

# Exploration: Binary Architecture and Composition Strategy

## Context

Given a monorepo direction, the question surfaces: should graft and grove be one binary or two? And beyond that, what's the right strategy for composing the various interfaces these tools might grow — CLI, TUI, daemon, MCP server, REST API, protocol buffers?

The concern is real: overengineering kills projects. But the opposite — underengineering the boundaries — creates painful rewrites later. The goal is to find the architectural strategy that keeps things simple now while not closing doors.

---

## What are graft and grove, really?

Before asking "same binary?" it's worth asking what these things actually do at the conceptual level.

**Graft** answers: "What are my dependencies, what's changed, and how do I upgrade?"
- Reads/writes graft.yaml, graft.lock
- Runs git operations (fetch, submodule sync)
- Executes migration commands
- Reports status

**Grove** answers: "What needs my attention across all my repos?"
- Reads workspace.yaml
- Aggregates git status across repos
- Runs status scripts
- Routes captures
- Searches across repos
- Displays all of this (TUI, CLI)

They share some concerns (git operations, reading graft.yaml) but solve different problems. Graft is about **dependency management**. Grove is about **workspace awareness**.

The relationship is: Grove *consumes* graft data. Grove reads graft.yaml to show dependency status. Grove might call graft to run upgrades. But graft doesn't know or care about Grove.

This is an asymmetric dependency, not a peer relationship.

---

## Strategy 1: Separate binaries, shared libraries

```
graft (binary)     grove (binary)
   │                   │
   └──────┬────────────┘
          │
   shared library crate(s)
   (git ops, yaml parsing, domain types)
```

**How it works:** Two independent binaries that happen to share library code. `grove` links against the same git and config-parsing libraries that `graft` uses. They're built from the same repo, same Cargo workspace, separate `[[bin]]` targets.

**Example from the ecosystem:** ripgrep and grep are separate tools. They share nothing. But within a project like BurntSushi's, `regex` and `regex-automata` are separate crates in a workspace, consumed by multiple binaries.

**Pros:**
- Clean separation. Each binary does one thing.
- Composable. Users can install just graft if they don't want grove.
- Standard Unix model. Small, sharp tools.
- Independent lifecycles. Graft can be stable while grove is experimental.
- Shared code without forced coupling.

**Cons:**
- Two things to install, two things to version, two things in PATH.
- Cross-tool workflows require shelling out (`grove` calls `graft status --json`). Adds latency and error surface.
- Shared library changes can break both tools simultaneously.
- Distribution complexity — do you ship two binaries? One archive with both?

**The composability question:** Very composable. Other tools can call either binary independently. Scripts can use graft without grove.

---

## Strategy 2: Single binary, subcommands

```
graft status              # dependency management
graft upgrade meta-kb
graft grove               # workspace management (or just `graft ui`)
graft grove capture "..."
graft serve               # daemon/API
graft mcp                 # MCP server
```

**How it works:** One binary, everything is a subcommand. Like `git` — one binary that does fetch, commit, log, bisect, and dozens of other things through subcommands.

**Example from the ecosystem:** `git` itself. `docker` (run, build, compose, etc.). `kubectl`. These are single binaries with enormous subcommand trees.

**Pros:**
- One thing to install. `curl | sh` and you're done.
- Shared process — grove's TUI can call graft operations in-process (no subprocess overhead, no JSON parsing).
- Single version. No compatibility matrix between graft v1.3 and grove v1.2.
- Simpler distribution. One binary, one release, one CI artifact.

**Cons:**
- Monolithic. The binary grows. Someone who just wants `graft status` gets the TUI, search indexing, and everything else.
- Naming tension. Is it `graft grove` or `graft workspace` or `graft ui`? The subcommand hierarchy matters and it's hard to rename later.
- Feature creep pressure. When everything's one binary, the temptation to add "just one more subcommand" is strong.
- Testing surface. Changes to grove's TUI code could theoretically break graft's CLI (through shared dependencies, build issues, etc.).

**The composability question:** Less composable externally (other tools call `graft grove status --json` — verbose), but more composable internally (grove calls graft functions directly, no serialization boundary).

---

## Strategy 3: Single binary, multicall

```
graft status              # when invoked as "graft"
grove                     # when invoked as "grove" (symlink to same binary)
grove capture "..."
```

**How it works:** One compiled binary, but it checks `argv[0]` to decide which personality to present. Install creates symlinks: `grove -> graft`. Like BusyBox — one binary, many names.

**Example from the ecosystem:** BusyBox (ls, cat, sh are all the same binary). `bat` ships as both `bat` and `batcat` on some systems.

**Pros:**
- Best of both: one binary to distribute, clean separate namespaces for users.
- `graft` and `grove` feel like separate tools but share everything internally.
- In-process calls between graft and grove functionality (no subprocess overhead).
- Single install, single version, but two clear identities.

**Cons:**
- Slightly clever. Multicall binaries surprise people who don't expect them.
- Symlink management adds installation complexity.
- Same monolithic binary concern as Strategy 2.
- Every platform handles symlinks differently (Windows is painful).

**The composability question:** Externally composable (clean `graft` and `grove` commands). Internally composable (shared library calls). The symlink mechanism is the only awkward part.

---

## Strategy 4: Library-first, binaries are thin shells

```
graft-core (library)        # domain types, config parsing
graft-engine (library)      # dependency operations
grove-engine (library)      # workspace operations

graft (binary)              # thin CLI shell over graft-engine
grove (binary)              # thin CLI/TUI shell over grove-engine
graft-server (binary)       # daemon exposing both engines via API
graft-mcp (binary)          # MCP server
```

**How it works:** The real work lives in libraries. Binaries are just thin wrappers that parse args, call library functions, and format output. Any new interface (REST API, MCP, protocol buffers) is just another thin wrapper.

**Example from the ecosystem:** This is how most well-architected Rust projects work. `ripgrep` has a `grep` library crate and an `rg` binary crate. The library does the work; the binary does I/O.

**Pros:**
- Maximum flexibility. Any interface is just a new thin binary.
- Testable. Library code is tested without CLI/TUI concerns.
- Embeddable. Other Rust programs can link against graft-engine or grove-engine.
- Clean architecture. The Grove spec already describes this (engine-UI separation).
- Doesn't force a decision on binary count. You can start with separate binaries and merge later, or vice versa.

**Cons:**
- More crates to manage (but Cargo workspaces handle this well).
- API design pressure. Library APIs need to be thoughtful — they're consumed by multiple binaries.
- Doesn't answer the "how many binaries" question — it defers it.

**The composability question:** This is the strategy that maximizes composability. Everything else is a decision about how to package the libraries.

---

## The interfaces question

You mentioned several possible interfaces. Let's think about which actually matter and when:

**CLI** — Needed from day one. Both graft and grove need scriptable, machine-readable interfaces. This exists for graft already.

**TUI** — Grove's primary interface. This is Slice 1. Without it, Grove doesn't exist.

**MCP server** — Increasingly important for agent integration. An MCP server wrapping grove-engine lets Claude/Cursor/etc. query workspace state, run status checks, capture notes. This is probably Slice 8-ish — valuable but not urgent.

**Daemon** — Only needed if real-time features matter (file watching, live status updates, push notifications to TUI). An open question in the specs. Could be deferred indefinitely — "rebuild on launch" might be good enough.

**REST API** — Only needed for the web UI. That's explicitly "future" in the architecture. And when it comes, it's just a thin wrapper around the engine library.

**Protocol buffers** — Only needed for cross-language high-performance IPC. This is the kind of thing you'd add if grove-engine is Rust and a mobile app needs to call it efficiently. Very far future, if ever.

**Assessment:** For the next 6-12 months, you need CLI and TUI. Everything else is theoretical. The architectural strategy should make CLI and TUI clean and natural, and not preclude the others. That's it.

---

## The plugin question

You mentioned plugins. Let's examine whether they're needed.

**What would a plugin do?** Possible plugin points:
- Custom status check types (but status scripts already handle this — any executable)
- Custom capture processors (but templates + shell scripts handle this)
- Custom search backends (but this is an implementation detail)
- Custom UI widgets (but ratatui doesn't have a plugin model)
- New graft operations (but graft's command system already delegates to shell scripts)

**The pattern:** In every case, the existing design already has an extension point — and it's shell scripts/executables, not in-process plugins. This is the Unix model: extend through composition, not through plugin APIs.

**Plugin systems are expensive.** They require:
- A stable API contract (you can't change internal types without breaking plugins)
- A loading mechanism (dynamic libraries? WASM? subprocess protocol?)
- Documentation and tooling for plugin authors
- Version compatibility management

**When plugins make sense:** Large ecosystems with many contributors who need to extend behavior without modifying core code. Think VS Code, webpack, Terraform.

**When they don't:** Small projects where the maintainer can add features directly, and extension through external executables covers the use cases.

**Assessment:** A plugin system would be overengineering. The shell script extension points (status scripts, graft commands) already provide composability without API commitments. If a plugin system ever becomes needed, it can be added later — and by then you'll know exactly what plugin points are needed, rather than guessing now.

---

## What about the daemon concern?

The daemon question keeps surfacing. Let's address it directly.

**Why you might want a daemon:**
- File watching (detect changes without polling)
- Persistent search index (no rebuild on launch)
- Multiple clients (TUI + MCP server + web UI sharing state)
- Background status checks (pre-compute so TUI launch is instant)

**Why you might not:**
- Complexity. Daemon lifecycle management (start, stop, restart, crash recovery, pid files, socket management) is significant engineering.
- State bugs. Daemon state can diverge from reality (file deleted but daemon doesn't know).
- The "departure board" model. You launch grove, glance, leave. If launch takes 300ms to rebuild state, that's fine. You don't live in it.

**The middle ground:** Start without a daemon. If performance demands it later, add one. The library-first architecture makes this a packaging decision, not an architectural one — grove-engine doesn't care whether it's invoked once per command or lives in a long-running process.

---

## Recommendation

**Strategy 4 (library-first) for architecture, Strategy 1 (separate binaries) for packaging.**

Given the decision to rewrite graft in Rust, the structure becomes clearer:

```
graft/  (monorepo)
  crates/
    graft-core/          # Shared types: GraftConfig, LockEntry, Dependency, Change
    graft-git/           # Git operations wrapping gitoxide (status, fetch, submodule)
    graft-engine/        # Dependency operations (what Python graft does today)
    grove-engine/        # Workspace operations (status, capture, search)
  bins/
    graft/               # CLI wrapping graft-engine
    grove/               # CLI + TUI wrapping grove-engine
  docs/                  # Specifications (from graft-knowledge)
  notes/                 # Explorations (from graft-knowledge)
  tests/                 # Integration tests
  Cargo.toml             # Workspace config
```

**Why library-first:** It's what the Grove architecture spec already describes (engine-UI separation). It makes every future interface (MCP, REST, daemon) a thin wrapper. It's testable. It's the natural Cargo workspace structure. And critically, it lets graft and grove share domain types and git operations without duplication.

**Why separate binaries:** Clean separation and composability. `graft` does dependency management. `grove` does workspace awareness. Other tools can call either independently. You can always merge them later (multicall or subcommand) if distribution convenience demands it — going from two binaries to one is easy, going from one to two is hard.

**Why shared crates matter now:** With both tools in Rust, `graft-core` types (GraftConfig, LockEntry, Dependency) are used by both. No Python/Rust serialization boundary. `graft-git` wrapping gitoxide is used by both. Write once, use everywhere.

**Why no plugins:** Shell scripts and external executables already cover extension points. A plugin API is a commitment you don't need to make yet.

**Why no daemon:** Start without one. The library architecture means adding one later is a packaging decision, not a rewrite.

**The principle:** Make the libraries right. The binaries are just packaging. Packaging can change cheaply; library boundaries are expensive to move.

---

## Open Questions

1. Should graft-core types be shared between Python graft and Rust grove during the transition period? (Probably not — just read the same YAML files independently.)
2. When grove-engine needs to invoke graft operations, should it call the library (if both are Rust) or shell out to the `graft` CLI? (Shell out during Python era; direct calls after Rust port.)
3. Is there a third tool lurking? (e.g., a `graft-index` or `graft-search` that neither graft nor grove fully owns?) Probably not yet — but the library-first architecture handles it if so.

---

## Sources

- [Implementation Language and Repo Structure](./2026-02-08-implementation-language-and-repo-structure.md) — Companion exploration
- [Grove Architecture](../docs/specifications/grove/architecture.md) — Engine-UI separation, three-layer model
- [Grove Vertical Slices](./2026-02-06-grove-vertical-slices.md) — Implementation priorities
