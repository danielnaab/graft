---
date: 2026-02-09
status: working
purpose: "Brainstorm patterns and structure for rust-starter guidance repo"
related:
  - ../.graft/python-starter/docs/README.md
  - ../.graft/rust-starter/
  - 2026-02-08-implementation-language-and-repo-structure.md
  - 2026-02-08-binary-architecture-and-composition.md
---

# Rust Starter Exploration

## Goal

Create a `rust-starter` repo — a graft dependency that documents idiomatic Rust patterns for our codebases (graft, grove, shared crates). Modeled after `python-starter` but adapted to Rust idioms and our specific architectural needs.

**Key constraint:** This is a *guidance repo*, not a copier template. Rust doesn't have a mature template generation story like Python's copier. The value is in documented patterns with inline code examples, ADRs explaining choices, and reference material that agents and humans consult when building.

## What python-starter Gets Right

Reviewing python-starter, the strongest aspects to carry forward:

1. **Code is canonical** — docs interpret and reference code, never replace it
2. **ADRs for every major tool choice** — not just "use X" but "why X over Y"
3. **Layered architecture** — domain → services → adapters ← cli
4. **Practical guides** — step-by-step "add a new service function" recipes
5. **Testing philosophy** — fakes over mocks, integration tests with real adapters
6. **Audience-aware docs** — agents.md for AI, guides for humans

## Rust vs Python: What Changes

### Same Concepts, Different Idioms

| Python Pattern | Rust Equivalent | Notes |
|---|---|---|
| `typing.Protocol` | `trait` | Both structural contracts; traits are more powerful (associated types, default impls) |
| Frozen dataclass | `struct` (owned fields) | Rust structs are immutable by default |
| Value objects | Newtype wrappers | `struct RepoId(String)` — zero-cost type safety |
| Domain exceptions | Error enums + `Result<T, E>` | No exceptions; errors are values |
| `ServiceContext` dataclass | Context struct or direct params | Same DI pattern, different syntax |
| Pure service functions | Free functions or `impl` blocks | Same idea; Rust makes purity natural |
| Fakes (class implementing Protocol) | Fakes (struct implementing trait) | Same pattern, traits instead of protocols |
| pytest fixtures | Builder/factory functions | No fixture framework; just functions |
| `uv` | `cargo` | Built-in, no alternative needed |
| `ruff` | `clippy` + `rustfmt` | Two tools, but both built-in |
| Typer (type-hint CLI) | clap derive (macro-based CLI) | Very similar DX |

### Genuinely New Patterns

Things Rust needs that Python doesn't:

1. **Ownership & borrowing** — When to take `&self` vs `self` vs `&mut self`. When to clone vs borrow. API design around ownership transfer.

2. **Crate architecture** — Workspace layout, when to split crates, pub API surface design. Python has packages but the boundaries are softer.

3. **Error handling strategy** — `thiserror` vs `anyhow`, when each applies, error conversion chains. This is a first-class architectural concern in Rust unlike Python's exception hierarchy.

4. **Feature flags** — Conditional compilation. No Python equivalent. When to use, naming conventions.

5. **Lifetime annotations** — When they appear, how to minimize them, when to just clone.

6. **Module visibility** — `pub`, `pub(crate)`, `pub(super)` — much more granular than Python's `_` convention.

## Core Opinions for rust-starter

### 1. Library-First Architecture

All business logic lives in library crates. Binaries are thin wrappers.

```
my-project/
  crates/
    my-core/        # Domain types, traits, error types
    my-engine/      # Business logic, orchestration
  src/              # Binary entry point (thin)
  Cargo.toml        # Workspace root
```

**Why:** Testable without spinning up CLI. Reusable across binaries. Matches our graft/grove shared-library plan.

### 2. Trait-Based Abstraction at Boundaries

Define traits in the crate that *needs* the capability (like Python's ports-in-domain pattern).

```rust
// In my-core/src/lib.rs
pub trait GitRepository {
    fn resolve_ref(&self, reference: &str) -> Result<Oid, GitError>;
    fn read_file(&self, path: &Path, oid: Oid) -> Result<Vec<u8>, GitError>;
}
```

Implementations live in separate crates or modules (adapters).

**Opinion:** Traits define *what*, not *how*. Keep traits small. Prefer multiple small traits over one large one (Interface Segregation).

### 3. Error Handling: thiserror for Libraries, anyhow for Binaries

```rust
// Library error (my-core)
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("config file not found: {path}")]
    NotFound { path: PathBuf },
    #[error("invalid config: {reason}")]
    Invalid { reason: String },
    #[error("IO error reading config")]
    Io(#[from] std::io::Error),
}

// Binary error handling (main.rs)
fn main() -> anyhow::Result<()> {
    let config = load_config(path)
        .context("failed to load project configuration")?;
    Ok(())
}
```

**Why:** Libraries need precise, matchable errors. Binaries need human-readable context chains. Never panic in library code.

### 4. Newtype Pattern for Domain Identity

```rust
/// A dependency name as declared in graft.yaml.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DependencyName(String);

impl DependencyName {
    pub fn new(name: impl Into<String>) -> Result<Self, ValidationError> {
        let name = name.into();
        if name.is_empty() {
            return Err(ValidationError::empty("dependency name"));
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

**Why:** Compiler catches `fn resolve(name: DependencyName)` vs `fn resolve(name: String)` misuse. Zero runtime cost. Self-documenting APIs.

### 5. Module Organization

```
src/
  lib.rs            # pub use re-exports, crate-level docs
  config.rs         # Configuration types
  config/
    parse.rs        # Parsing logic (if config.rs gets large)
  domain.rs         # Domain types
  error.rs          # Error types
```

**Opinions:**
- Prefer flat files over deep nesting. Split into subdirectories only when a file exceeds ~300 lines.
- Use `mod.rs` sparingly (Rust 2018 named modules preferred).
- `pub use` in `lib.rs` to create a clean public API surface.
- `pub(crate)` for internal helpers that shouldn't leak.

### 6. Testing: Trait Fakes, No Mocking Framework

```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct FakeGitRepo {
        files: HashMap<PathBuf, Vec<u8>>,
    }

    impl FakeGitRepo {
        fn with_file(mut self, path: impl Into<PathBuf>, content: &[u8]) -> Self {
            self.files.insert(path.into(), content.to_vec());
            self
        }
    }

    impl GitRepository for FakeGitRepo {
        fn read_file(&self, path: &Path, _oid: Oid) -> Result<Vec<u8>, GitError> {
            self.files.get(path)
                .cloned()
                .ok_or(GitError::FileNotFound { path: path.to_owned() })
        }
    }

    #[test]
    fn resolves_config_from_repo() {
        let repo = FakeGitRepo::default()
            .with_file("graft.yaml", b"name: test\n");

        let config = load_config(&repo, Path::new("graft.yaml")).unwrap();
        assert_eq!(config.name.as_str(), "test");
    }
}
```

**Why:** No `mockall` or `mockito`. Fakes are real implementations of traits with controlled behavior. Builder-style setup (`.with_file()`). Tests read like specifications.

### 7. CLI with clap Derive

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "graft", about = "Semantic dependency management")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Output as JSON for machine consumption
    #[arg(long, global = true)]
    pub json: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Resolve all dependencies
    Resolve,
    /// Show dependency status
    Status {
        /// Show only this dependency
        name: Option<String>,
    },
}
```

**Why:** Type-driven, auto-generates help, compile-time validation of CLI structure.

### 8. Configuration with serde

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GraftConfig {
    pub name: String,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
}
```

**Opinions:**
- `kebab-case` in YAML, `snake_case` in Rust — serde handles the mapping.
- Validate after deserialization (constructor/`TryFrom`), not during.
- Strong types for validated fields (newtypes), raw strings for simple pass-through.

### 9. Workspace Cargo.toml

```toml
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.package]
edition = "2021"
license = "MIT"
rust-version = "1.75"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
thiserror = "2"
anyhow = "1"
clap = { version = "4", features = ["derive"] }
```

**Why:** Shared dependency versions prevent version skew. `resolver = "2"` is required for modern feature resolution. `rust-version` communicates MSRV.

## ADR Topics

Following python-starter's pattern of one ADR per major choice:

1. **Cargo workspace** — Why workspace layout, when to split crates
2. **Library-first architecture** — Why logic in libs, binaries are thin
3. **Trait-based dependency injection** — Why traits at boundaries, no dyn dispatch by default
4. **Error handling strategy** — Why thiserror + anyhow, no panics in libs
5. **clap for CLI** — Why clap derive over alternatives
6. **clippy + rustfmt** — Why both, which lints to enforce
7. **Newtype pattern** — Why wrapped types for domain identity
8. **Testing without mocks** — Why trait fakes over mockall

## Open Questions

1. **Async or not?** — graft/grove are mostly sync (git operations, file I/O). Do we opine on async at all, or leave it out of starter? Leaning toward: document when *not* to use async (most of the time), note tokio as the choice *when needed*.

2. **How much example code?** — python-starter has full working templates. rust-starter is guidance only. Should we include a minimal example crate that compiles and tests? Or keep it purely docs with inline code snippets? Leaning toward: inline snippets in docs, no compilable example (simpler to maintain).

3. **Tracing vs log** — `tracing` is the modern choice but adds complexity. For our use case (CLI tools), is `log` + `env_logger` sufficient? Leaning toward: `tracing` — it's the ecosystem direction and structured logging is valuable.

4. **Dynamic dispatch (`dyn Trait`) vs generics** — When should we use `Box<dyn Trait>` vs `impl Trait` vs `T: Trait`? This needs a clear decision framework. Leaning toward: generics by default, `dyn` only when heterogeneous collections are needed.

## Proposed rust-starter Structure

```
rust-starter/
├── README.md
├── knowledge-base.yaml
├── docs/
│   ├── README.md
│   ├── agents.md
│   ├── architecture/
│   │   └── architecture.md
│   ├── decisions/
│   │   ├── 001-cargo-workspace.md
│   │   ├── 002-library-first.md
│   │   ├── 003-trait-based-di.md
│   │   ├── 004-error-handling.md
│   │   ├── 005-clap-cli.md
│   │   ├── 006-clippy-rustfmt.md
│   │   ├── 007-newtype-pattern.md
│   │   └── 008-testing-without-mocks.md
│   ├── guides/
│   │   ├── getting-started.md
│   │   └── development.md
│   └── reference/
│       └── project-reference.md
```

## Next Steps

1. Create the foundational files in `.graft/rust-starter/`
2. Write knowledge-base.yaml and README.md
3. Write architecture.md with the core patterns
4. Write ADRs for each major decision
5. Write guides for common operations
6. Review against actual grove/graft implementation needs
