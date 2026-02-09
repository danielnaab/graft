---
title: "Exploration: Implementation Language and Repository Structure"
date: 2026-02-08
status: working
participants: ["human", "agent"]
tags: [exploration, rust, python, monorepo, architecture, grove, graft]
---

# Exploration: Implementation Language and Repository Structure

## Context

Graft is currently implemented in Python (~8,700 lines, clean architecture, 55 test files, mypy strict, good coverage). Grove exists only as specifications. The specs and exploration notes live in a separate `graft-knowledge` repo, which was created to dogfood graft's dependency update mechanism — but migration commands haven't been exercised yet, so the dogfooding rationale hasn't been validated.

Several questions are tangled together:

1. **Language**: Should Grove be written in Rust? Should graft be rewritten in Rust?
2. **Repository structure**: Should graft, grove, and specs live in one repo or stay separate?
3. **Sequencing**: What order should these decisions happen in?

These questions interact — the language choice affects how much shared code is possible, which affects how much a monorepo helps.

---

## Question 1: Rust vs Python

### The case for Rust

**Performance where it matters.** Grove is an interactive TUI — startup time, render latency, and search speed are user-facing. Rust is naturally fast without optimization effort. Python TUIs can feel sluggish, especially with file watching and search indexing across many repos.

**TUI ecosystem.** ratatui is the most mature terminal UI library in any language. Python's alternatives (textual, urwid, blessed) are usable but less battle-tested for complex interactive applications. ratatui has a large community, active development, and extensive examples.

**Single binary distribution.** `cargo build --release` produces one file. No Python version management, no virtualenvs, no `pip install` ceremony. For a developer tool that should "just work," this matters.

**Learning opportunity.** You've expressed interest in learning Rust. Building a real project is the best way. With Claude Code handling the mechanical parts (borrow checker fights, lifetime annotations, trait implementations), the experience becomes more about understanding Rust's concepts than fighting syntax.

**Claude Code changes the calculus.** The traditional argument against Rust for a solo developer is velocity — Python is faster to iterate in. But with Claude Code writing most of the code, the velocity gap shrinks dramatically. You're directing architecture and design, not hand-writing every function. Rust's compiler catches entire categories of bugs that Python only catches at runtime (or never), which actually speeds up the overall cycle.

### The case for staying with Python

**Known quantity.** You're comfortable with Python. The existing graft codebase is clean, well-tested, and works. Rewriting a working system is risky — you might introduce bugs, lose features, or spend months on parity before making progress.

**The graft codebase is substantial.** ~8,700 lines with 55 test files, clean architecture, protocol-based DI, frozen dataclasses. This isn't a quick rewrite. Even with Claude Code, porting the service layer, domain models, CLI commands, and test suite is weeks of work.

**Python is fine for graft.** Graft itself is a CLI tool that runs git operations and parses YAML. Performance isn't critical — `graft status` taking 200ms vs 20ms doesn't matter. Python's strengths (rapid iteration, readable code, excellent YAML/git libraries) fit graft well.

**Splitting the language is viable.** Graft stays Python. Grove is Rust. They communicate through CLI (`grove` calls `graft status --json`) and shared file formats (graft.yaml, graft.lock). No shared library needed — the specs define the contract.

### The hybrid path: Grove in Rust, graft stays Python

This might be the sweet spot:

- **Graft stays Python** — it works, it's tested, it does its job. Rewriting it doesn't add user value.
- **Grove is Rust** — it's greenfield, performance-sensitive, and the TUI ecosystem is better in Rust.
- **They share nothing at the library level** — they communicate through files and CLI, exactly as the specs describe.
- **Later, if Rust proves comfortable**, graft could be ported. But it's not a prerequisite.

This avoids the biggest risk (rewriting working software) while still getting Rust experience on a project where Rust's strengths matter.

### The full-Rust path: Rewrite everything

If you go monorepo with shared Rust libraries, you'd want both in Rust. The argument:

- **Shared domain types.** `GraftConfig`, `LockEntry`, `Change`, `Dependency` — these are the same types in both tools. In Rust, they'd be a shared crate. In Python+Rust, they're duplicated.
- **Shared git operations.** Both tools do git operations. A shared git abstraction (wrapping gitoxide) avoids duplication.
- **One toolchain.** cargo, clippy, rustfmt, one CI pipeline, one release process.
- **Faster graft.** Not critical, but nice. `graft status` on 20 repos would be noticeably snappier.

The counterargument is cost: ~8,700 lines of Python to rewrite, 55 test files to port, risk of regressions, and weeks before you're back at feature parity. With Claude Code this is more feasible than by hand, but it's still real work with real risk.

---

## Question 2: Monorepo vs Multi-repo

### Current structure

```
~/src/graft/              # Python implementation (~8,700 lines)
~/src/graft-knowledge/    # Specs, decisions, notes (~108 markdown files)
```

Grove doesn't exist yet as code.

### Why graft-knowledge was separate

The stated reason: dogfood graft's dependency update mechanism. `graft-knowledge` would depend on `meta-knowledge-base` and `living-specifications` via graft, exercising the upgrade/migration flow.

**Reality check:** Migration commands haven't been used yet. The dependency relationship exists (graft.yaml declares deps, graft.lock tracks them) but the value proposition — automated migrations when upstream changes — hasn't been validated. The dogfooding has been limited to `graft sync` / submodule management, not the full change-tracking flow.

### The case for monorepo

**Specs next to code.** When you change behavior, you update the spec in the same commit. No cross-repo coordination, no "remember to update the spec repo." This is the living-specifications ideal — specs evolve with the code.

**Shared context.** An agent working on graft can read the specs without cloning another repo. A human reviewing a PR can see spec changes alongside code changes.

**Simplified workflow.** One clone, one branch, one PR, one CI pipeline. No submodule headaches, no version skew between repos.

**Grove shares the design space.** Grove's specs reference graft's specs. They share concepts (graft.yaml format, dependency model). Having them in one repo makes cross-references simple relative paths instead of cross-repo links.

**Monorepo doesn't prevent dogfooding.** You can still use graft to track dependencies on `meta-knowledge-base` and `living-specifications` from within a monorepo. The graft.yaml in the monorepo root would declare those external dependencies. The dogfooding target is external deps, not the relationship between graft's own code and specs.

### The case for keeping repos separate

**Separation of concerns.** Specs are authoritative design documents. Code is implementation. Mixing them risks treating specs as "just comments" — updated carelessly or not at all.

**Different audiences.** Someone reading specs doesn't need 8,700 lines of Python in their clone. Someone hacking on graft doesn't need 108 markdown files in their diff.

**Different cadences.** Specs might stabilize while code churns, or vice versa. Separate repos let them version independently.

**The dogfooding argument still holds.** Even if migrations haven't been used yet, the *structure* is in place. When graft matures to where migration commands work, having graft-knowledge as a real consumer is valuable.

### Assessment

The separation-of-concerns argument is real but weak in practice for a solo/small-team project. The "different audiences" concern doesn't apply when there's one developer. The cadence argument cuts both ways — specs that drift from code are worse than specs next to code.

The strongest argument for monorepo: **you want specs and code to evolve together, and cross-repo coordination is friction that discourages that.**

The strongest argument against: **you lose a real graft consumer for dogfooding.** But you can create other consumers (a template repo, a tutorial repo) that are simpler and more focused dogfooding targets.

---

## Question 3: What to call it

If everything merges into one repo, the name matters. Some options:

**`graft`** — The simplest. It's the project name. The repo contains graft (the tool), grove (the workspace tool), and the specs/docs. Python projects and Rust projects coexist fine in one repo with separate directories. This is what most projects do — the repo is named after the project.

**`graft-workspace`** — Emphasizes that this is the development workspace for the graft ecosystem. But it's confusingly close to Grove's "workspace" concept.

**`graft-project`** — Generic. Doesn't add meaning.

**`graft-mono`** — Explicit about being a monorepo. A bit mechanical.

**Recommendation: just `graft`.** The repo is the project. It contains everything related to graft — the tool, the workspace tool, the specs, the docs. This is the overwhelmingly common convention (React, Rust, Go, Kubernetes — all named after the project, all monorepos with multiple components).

The current `graft` repo would absorb `graft-knowledge`'s content. The directory structure might look like:

```
graft/
  docs/                    # From graft-knowledge
    specifications/
    decisions/
    architecture.md
  notes/                   # From graft-knowledge
  src/
    graft/                 # Python graft implementation (current)
    grove/                 # Rust grove implementation (future)
  tests/                   # Python tests (current)
  pyproject.toml           # Python project config
  Cargo.toml               # Rust workspace config (when grove starts)
  graft.yaml               # Self-referential? Or just for external deps
  CHANGELOG.md
  README.md
```

Or if you go full Rust:

```
graft/
  docs/
  notes/
  crates/
    graft-core/            # Shared domain types
    graft-cli/             # Graft CLI
    grove-engine/          # Grove workspace engine
    grove-tui/             # Grove TUI
  graft.yaml
  Cargo.toml               # Rust workspace
```

---

## Question 4: Sequencing

Whatever you decide, the order matters. Some possible sequences:

### Path A: Incremental (lowest risk)

1. **Keep Python graft, start Grove in Rust** in the existing graft repo or a new `grove` repo
2. **Validate Rust experience** through Grove slices 1-3
3. **If Rust feels good**, merge repos and consider porting graft
4. **If Rust feels bad**, Grove stays Rust (it's already there), graft stays Python, repos stay separate

This is conservative. You learn Rust on greenfield code where there's nothing to break. The graft Python codebase stays stable as your working tool.

### Path B: Monorepo first, then Rust

1. **Merge graft-knowledge into graft** (just docs/notes, no code changes)
2. **Start Grove in Rust** within the monorepo
3. **Later, consider porting graft** if the Rust experience is positive

This gets the organizational benefits quickly without any language risk.

### Path C: Full Rust monorepo (highest risk, highest payoff)

1. **Create new monorepo** with Cargo workspace
2. **Port graft to Rust** (with Claude Code doing the heavy lifting)
3. **Build Grove sharing graft's domain types**
4. **Archive old repos**

This is the most work upfront but results in the cleanest architecture. The risk is spending weeks on parity before adding new value. With Claude Code the timeline is compressed, but testing and edge cases still take time.

### Recommended: Path B

Merge the repos first (low risk, immediate organizational benefit), then start Grove in Rust within the monorepo. This gives you:

- Specs next to code immediately
- Rust learning through greenfield Grove work
- No risk to the working graft tool
- Option to port graft later with full context
- Clean directory structure from day one

---

## The "Claude Code changes everything" angle

This deserves its own section because it's the most novel factor.

**Traditional wisdom:** Use the language you know. Learning a new language while building a real project is slow and produces non-idiomatic code. Stick with Python.

**With Claude Code:** The bottleneck shifts from "can I write this code?" to "can I evaluate whether this code is correct?" You don't need to know Rust's borrow checker rules by heart — Claude Code handles that. What you need is:

1. **Architectural judgment** — You have this. Your Python graft codebase shows clean architecture thinking.
2. **Ability to read and review** — Rust is readable even if you can't write it fluently. The type signatures are documentation.
3. **Understanding of what to ask for** — You know what graft and grove need to do. The specs are written.

**What Claude Code doesn't change:**

- **Debugging production issues.** When something goes wrong at 2am, you need to understand the code well enough to diagnose it. Rust's error messages are good but dense.
- **Ecosystem knowledge.** Knowing which crates are well-maintained, which patterns are idiomatic, which approaches will cause problems later. Claude Code knows current best practices, but you can't verify its judgment without experience.
- **Build system complexity.** Cargo is simpler than Python packaging (no virtualenvs!), but Cargo workspaces, feature flags, and cross-compilation have their own learning curve.

**Net assessment:** Claude Code makes Rust viable for you in a way it wouldn't have been two years ago. The risk is lower than traditional "learn a new language" projects. But it's not zero — you're still the one maintaining the system.

---

## Open Questions

1. Is there value in keeping graft-knowledge as a dogfooding consumer, or is that theoretical value that hasn't materialized?
2. If going monorepo, should the merge happen before or after starting Grove?
3. How much does shared Rust library code between graft and grove actually matter? Could they share only through file formats and CLI, even in the same repo?
4. What's the minimum Rust experience needed before porting graft would be responsible (vs. just building Grove)?

---

## Decision: Full Rust with graft rewrite

After exploring the options, the decision is to go full Rust with the intent to rewrite graft. This changes the calculus significantly:

**What this means:**

1. **Shared domain types from day one.** `GraftConfig`, `LockEntry`, `Change`, `Dependency` — these types will be used by both graft and grove. Write them once in `graft-core`. No duplication, no Python/Rust impedance mismatch.

2. **Shared git operations.** Both tools do git operations (status, fetch, commit, submodule management). A single `graft-git` crate wrapping gitoxide avoids duplication and ensures consistent behavior.

3. **Grove validates the architecture.** Building grove-engine first exercises the library boundaries and design patterns. When you port graft, you're following an established pattern rather than inventing it. Grove becomes the proof-of-concept for the architecture.

4. **No transitional complexity.** You don't maintain Python/Rust interop or two versions of the same types. The Python graft stays as-is until the Rust port is feature-complete, then you switch. No gradual migration pain.

5. **One ecosystem, one toolchain.** cargo, clippy, rustfmt, one CI pipeline, one release process. No Python/Rust split tooling.

**What stays the same from the explorations:**

- **Library-first architecture** — even more important now. `graft-core`, `graft-engine`, `grove-engine` as libraries, binaries as thin wrappers.
- **Separate binaries** — `graft` and `grove` stay separate commands. Clean boundaries, composable.
- **No plugins, no daemon** — these were already not needed. Rust doesn't change that.
- **Monorepo** — graft + grove + specs in one repo. The shared libraries make this even more compelling.

**What changes:**

- **Sequencing.** The recommended path was "start Grove in Rust, keep graft in Python, merge later if Rust feels good." Now it's: "start Grove in Rust, then port graft to Rust, sharing the foundation."
- **Library investment.** You invest more in `graft-core` upfront, knowing both tools will use it. Make those types really good.
- **Testing strategy.** The test suite from Python graft becomes the acceptance criteria for Rust graft. 55 test files define expected behavior.

**Revised structure:**

```
graft/  (monorepo)
  crates/
    graft-core/          # Shared domain types and traits
    graft-git/           # Git operations wrapping gitoxide
    graft-engine/        # Dependency management operations
    grove-engine/        # Workspace awareness operations
  bins/
    graft/               # CLI for graft-engine
    grove/               # CLI + TUI for grove-engine
  docs/                  # Specifications (from graft-knowledge)
  notes/                 # Explorations (from graft-knowledge)
  tests/                 # Integration tests
  Cargo.toml             # Workspace config
```

**Why this is better than the hybrid:**

- No Python/Rust boundary to maintain forever
- Shared code where it matters (domain types, git operations)
- One learning investment (Rust) with two tools to show for it
- Performance benefits for graft too (startup time, parallel operations)
- Simpler long-term maintenance

**The risk:**

Porting 8,700 lines of Python with 55 test files is real work. Even with Claude Code, it's weeks. The mitigation: grove validates the architecture first, and you have the Python version as working reference.

## Sources

- Current graft implementation: `/home/coder/src/graft/` (~8,700 lines Python)
- Current specs: `/home/coder/src/graft-knowledge/` (~108 markdown files)
- [Grove Architecture](../docs/specifications/grove/architecture.md)
- [Grove Vertical Slices](./2026-02-06-grove-vertical-slices.md)
- [Binary Architecture and Composition](./2026-02-08-binary-architecture-and-composition.md)
