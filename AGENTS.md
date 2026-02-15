# Agent Entrypoint - Graft Project

You are working on the **Graft** project: semantic dependency management for knowledge bases.

This repo contains two components in a shared Rust workspace:
- **Graft** (Rust CLI in `crates/graft-*`, ready for use; Python legacy in `src/graft/`) - semantic dependency manager
- **Grove** (Rust workspace tool in `crates/grove-*`) - workspace management for multi-repo development

## Orientation

| Path | Purpose |
|------|---------|
| `Cargo.toml` | Virtual workspace manifest (all Rust crates) |
| `crates/` | All Rust crates (grove-core, grove-engine, grove-cli, graft-core, graft-engine, graft-cli) |
| `src/graft/` | Python source (legacy, kept during Rust transition) |
| `grove/docs/` | Grove-specific docs ([agent entrypoint](grove/docs/agents.md)) |
| `docs/specifications/` | Canonical specs (architecture, graft format, grove specs, decision ADRs) |
| `docs/` | Implementation documentation (architecture overview, guides, ADRs) |
| `notes/` | Time-bounded exploration notes ([index](notes/index.md)) |
| `.graft/` | Dependencies managed via `graft resolve` |
| `knowledge-base.yaml` | KB structure declaration |

## Verification commands

Always run before committing:

```bash
# Graft (Rust) - primary implementation
cargo fmt --check                # Format check
cargo clippy -- -D warnings      # Lint
cargo test                       # All Rust tests (49 tests: 42 unit, 7 integration)
cargo run -p graft-cli -- status # Smoke test

# Graft (Python) - legacy, maintained for compatibility
uv run pytest                    # 405 tests, ~46% coverage
uv run mypy src/                 # Strict mode type checking
uv run ruff check src/ tests/   # Linting
```

## Architectural principles

The Python codebase follows patterns from [python-starter](.graft/python-starter/):

- **Protocol-based DI**: Services accept `typing.Protocol` interfaces, not concrete types. See `src/graft/protocols/` for all protocol definitions.
- **Frozen dataclasses**: All domain models in `src/graft/domain/` are `@dataclass(frozen=True)` — immutable by design.
- **Functional service layer**: Business logic lives in pure functions in `src/graft/services/`, not in classes. Functions take a protocol-typed context parameter.
- **Fakes, not mocks**: Tests use in-memory fakes (`tests/fakes/`) that implement protocols. No unittest.mock.

The Rust codebase (Grove and Graft) follows patterns from [rust-starter](.graft/rust-starter/):

- **Library-first architecture**: Core logic in library crates, thin binary wrappers.
- **Trait-based boundaries**: Rust equivalent of Protocol-based DI.
- **Error handling as values**: `thiserror` for library errors, `anyhow` for binary errors.
- **Newtype pattern**: Domain identity types wrap primitives for type safety.

Graft Rust implementation status:
- ✅ All core operations implemented (`status`, `resolve`, `fetch`, `sync`, `apply`, `upgrade`, `validate`, `changes`, `show`, `add`, `remove`, `run`)
- ✅ State queries (Stage 1) implemented
- ✅ Output parity with Python CLI verified (see `notes/2026-02-15-rust-rewrite/parity-verification.md`)
- ✅ 49 tests passing (42 unit, 7 integration)
- Ready for production use

## Workflow: Plan -> Patch -> Verify

From the [agent workflow playbook](.graft/meta-knowledge-base/docs/playbooks/agent-workflow.md):

1. **Plan**: State intent, identify files to touch, check specs in `docs/specifications/`
2. **Patch**: Make minimal changes that achieve the goal
3. **Verify**: Run the verification commands above

## Authority rules

When sources disagree, follow this precedence:

1. **Source code** (`src/`, `crates/`) — canonical for how things actually work
2. **Specifications** (`docs/specifications/`) — canonical for what to build
3. **Implementation docs** (`docs/`) — interpretation of specs, may lag behind code
4. **Notes** (`notes/`) — ephemeral exploration, may contain outdated thinking

When docs and code disagree, **code is canonical** for implementation details. When code and specs disagree, **specs are canonical** for intended behavior (the code has a bug).

## Write boundaries

You may write to:
- `src/**` - Python source code
- `crates/**` - Rust source code
- `tests/**` - Python test code
- `docs/**` - Implementation documentation
- `notes/**` - Time-bounded development notes

Never write to:
- `docs/specifications/**` - Canonical specs (requires explicit spec-change workflow)
- `secrets/**`, `config/prod/**` - Sensitive configuration

## Working with Rust crates

All Rust code lives in `crates/` under a virtual workspace rooted at `Cargo.toml`:
- **grove-core**, **grove-engine**, **grove-cli** — Grove workspace manager
- **graft-core**, **graft-engine**, **graft-cli** — Graft in Rust (rewrite in progress)

Grove agent entrypoint: [`grove/docs/agents.md`](grove/docs/agents.md)
Grove specifications: [`docs/specifications/grove/`](docs/specifications/grove/)

When working on Rust crates, follow patterns from [rust-starter](.graft/rust-starter/).

## .graft dependencies

Declared in [`graft.yaml`](graft.yaml), cloned to `.graft/` via `graft resolve`:

| Dependency | Purpose | Entrypoint |
|-----------|---------|------------|
| [meta-knowledge-base](.graft/meta-knowledge-base/) | KB organization framework, policies, playbooks | [AGENTS.md](.graft/meta-knowledge-base/AGENTS.md) |
| [python-starter](.graft/python-starter/) | Python clean architecture patterns and template | [knowledge-base.yaml](.graft/python-starter/knowledge-base.yaml) |
| [rust-starter](.graft/rust-starter/) | Rust architecture patterns and template | [knowledge-base.yaml](.graft/rust-starter/knowledge-base.yaml) |
| [living-specifications](.graft/living-specifications/) | Living spec methodology: format, principles, writing guide | [knowledge-base.yaml](.graft/living-specifications/knowledge-base.yaml) |

## Meta-KB policies

The [meta-knowledge-base](.graft/meta-knowledge-base/) defines these policies. This project follows all of them:

| Policy | Action rule |
|--------|-------------|
| [Authority](.graft/meta-knowledge-base/docs/policies/authority.md) | Distinguish canonical sources from interpretation; follow precedence above |
| [Provenance](.graft/meta-knowledge-base/docs/policies/provenance.md) | Ground factual claims in sources; link to evidence |
| [Lifecycle](.graft/meta-knowledge-base/docs/policies/lifecycle.md) | Mark document status (draft/working/stable/deprecated); living docs must reflect current state |
| [Write Boundaries](.graft/meta-knowledge-base/docs/policies/writes.md) | Only modify allowed paths; respect role boundaries |
| [Temporal Layers](.graft/meta-knowledge-base/docs/policies/temporal-layers.md) | Specs are stable, docs interpret, notes are ephemeral |
| [Intent-Revealing Structure](.graft/meta-knowledge-base/docs/policies/intent-revealing-structure.md) | File organization should reveal purpose; names should be self-documenting |
| [Linking](.graft/meta-knowledge-base/docs/policies/linking.md) | Link to sources rather than duplicating; maintain link health |
| [Style](.graft/meta-knowledge-base/docs/policies/style.md) | Plain language, no emojis, professional tone |
| [Generated Content](.graft/meta-knowledge-base/docs/policies/generated-content.md) | Mark AI-generated content; track provenance of generated artifacts |

## Key documentation

- **Specifications**: [Architecture](docs/specifications/architecture.md), [Graft specs](docs/specifications/graft/), [Grove specs](docs/specifications/grove/), [Decision ADRs](docs/specifications/decisions/)
- **Implementation**: [Architecture overview](docs/README.md), [Contributing guide](docs/guides/contributing.md), [Implementation ADRs](docs/decisions/)
- **Session context**: [continue-here.md](continue-here.md), [tasks.md](tasks.md)
- **Notes**: [Index](notes/index.md)
