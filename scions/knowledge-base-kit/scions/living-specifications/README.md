# Living Specifications

Specification format, principles, and methodology for spec-driven development.

## What This Is

Living specifications are the source of truth for system behavior. They describe what a system does and why, sitting between intent and implementation.

```
SPEC (readable, durable, collaborative)
  ↓ maps to
TESTS (executable, durable, CI-enforced)
  ↓ verified by
CODE (implementation, durable, reviewed)
```

This repository defines **how to write and maintain specifications** for teams adopting spec-driven development.

## What This Standardizes

- **Format**: The specification template structure and frontmatter requirements
- **Principles**: Normative rules governing specification practices
- **Relationships**: How specs relate to tests, acceptance criteria, and code
- **Verification**: Drift detection approaches between specs and implementations
- **Comprehension**: Making the specification system legible to all collaborators

## Core Reference

- [Format](docs/format.md) — the specification template
- [Principles](docs/principles.md) — rules governing specification practices
- [Relationships](docs/relationships.md) — how specs relate to tests and code
- [Verification](docs/verification.md) — drift detection approaches
- [Comprehension](docs/comprehension.md) — making the system legible

## Guides

- [Writing specs](docs/guides/writing-specs.md) — how to write or update a specification
- [Ensemble with specs](docs/guides/ensemble-with-specs.md) — using specs in mob programming
- [Reviewing against specs](docs/guides/reviewing-against-specs.md) — reviewing PRs with specs as source of truth

## Patterns Borrowed

| Pattern | What We Took | What We Left |
|---------|-------------|--------------|
| ADRs | Context → Decision → Consequences; status lifecycle | Immutability (specs evolve) |
| RFCs | Propose → discuss → decide; Alternatives Considered | Heavyweight process |
| BDD/Gherkin | Given/When/Then as thinking structure | Step definitions; Cucumber |
| Google Design Docs | Non-goals; living document expectation | Google Docs (not version-controlled) |
| Executable Specs | Structured-enough-to-check assertions | Framework lock-in |
| Living Documentation | Specs for what code can't express; computed views | Assumption code is always readable |

Guiding principle: steal structures, not tooling.

## Using This in Your Repository

Graft it ([graft](https://github.com/danielnaab/graft)): a pinned copy of this repo's export
(README, CHANGELOG, `docs/`) in your `scions/living-specifications/`, then link into that copy from
your agent instructions (`AGENTS.md` or `CLAUDE.md`):

```nix
inputs.living-specifications = { url = "github:danielnaab/living-specifications/v1.0.2"; flake = false; };
```

Or link to it by URL (https://github.com/danielnaab/living-specifications/tree/v1.0.2/docs) with
nothing to pin. Grafting gives agents a local copy and an upgrade path; a link is enough for people.

## Architecture

```
living-specifications (this repo: spec-writing methodology)
        ↓ implemented by
knowledge-base-integrity (enforcement tooling: its backlink scanner checks `# spec:` annotations)
        ↓
your-project (applies the methodology)
```

This repo owns the spec methodology (format, annotations, relationships). It is itself maintained as a
knowledge base following [meta-knowledge-base](https://github.com/danielnaab/meta-knowledge-base)'s
conventions, which it grafts into `scions/` for its own upkeep only; nothing from that copy is exported,
so the methodology doesn't depend on it.

## Sources

- ADR pattern: Michael Nygard, "Documenting Architecture Decisions" (2011)
- BDD: Dan North, "Introducing BDD" (2006)
- Living Documentation: Cyrille Martraire (2019)
- C4 Model: Simon Brown

## Navigation

- [Docs](docs/)
