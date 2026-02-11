# Agent Entrypoint - Graft Project

You are working on the **Graft** project: semantic dependency management for knowledge bases.

This repo contains two components:
- **Graft** (Python CLI) - `src/graft/` - the dependency manager
- **Grove** (Rust workspace tool) - `grove/` - workspace management for multi-repo development

## Orientation

| Path | Purpose |
|------|---------|
| `src/graft/` | Python source (domain/services/protocols/adapters/cli) |
| `grove/` | Rust workspace tool (submodule, has its own [agent entrypoint](grove/docs/agents.md)) |
| `docs/specifications/` | Canonical specs (architecture, graft format, grove specs, decision ADRs) |
| `docs/` | Implementation documentation (architecture overview, guides, ADRs) |
| `notes/` | Time-bounded exploration notes ([index](notes/index.md)) |
| `.graft/` | Dependencies managed via `graft resolve` |
| `knowledge-base.yaml` | KB structure declaration |

## Verification commands

Always run before committing:

```bash
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

The Rust codebase (Grove) follows patterns from [rust-starter](.graft/rust-starter/):

- **Library-first architecture**: Core logic in library crates, thin binary wrappers.
- **Trait-based boundaries**: Rust equivalent of Protocol-based DI.
- **Error handling as values**: `thiserror` for library errors, `anyhow` for binary errors.
- **Newtype pattern**: Domain identity types wrap primitives for type safety.

## Workflow: Plan -> Patch -> Verify

From the [agent workflow playbook](.graft/meta-knowledge-base/docs/playbooks/agent-workflow.md):

1. **Plan**: State intent, identify files to touch, check specs in `docs/specifications/`
2. **Patch**: Make minimal changes that achieve the goal
3. **Verify**: Run the verification commands above

## Authority rules

When sources disagree, follow this precedence:

1. **Source code** (`src/`, `grove/`) — canonical for how things actually work
2. **Specifications** (`docs/specifications/`) — canonical for what to build
3. **Implementation docs** (`docs/`) — interpretation of specs, may lag behind code
4. **Notes** (`notes/`) — ephemeral exploration, may contain outdated thinking

When docs and code disagree, **code is canonical** for implementation details. When code and specs disagree, **specs are canonical** for intended behavior (the code has a bug).

## Write boundaries

You may write to:
- `src/**` - Source code
- `tests/**` - Test code
- `docs/**` - Implementation documentation
- `notes/**` - Time-bounded development notes

Never write to:
- `docs/specifications/**` - Canonical specs (requires explicit spec-change workflow)
- `secrets/**`, `config/prod/**` - Sensitive configuration

## Working with Grove

Grove is a Rust workspace tool in the `grove/` submodule. It has its own:
- Agent entrypoint: [`grove/docs/agents.md`](grove/docs/agents.md)
- Specifications: [`docs/specifications/grove/`](docs/specifications/grove/)

When working on Grove, read its agent entrypoint first. The root project's Rust patterns (from `rust-starter`) apply.

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
