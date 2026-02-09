# Architectural Decision Records

This directory contains Architectural Decision Records (ADRs) documenting key design decisions for Graft.

## Purpose

ADRs provide:
- **Context** - What situation led to the decision
- **Decision** - What was decided
- **Rationale** - Why this decision was made
- **Alternatives** - What other options were considered
- **Consequences** - What are the impacts (positive and negative)

## Format

Each ADR follows a standard structure:
- Title: "Decision NNNN: Brief Title"
- Date: When the decision was made
- Status: accepted | rejected | superseded | deprecated
- Context, Decision, Rationale, Alternatives, Consequences, Related

## Decisions

### Core Architecture

- **[Decision 0001](./decision-0001-initial-scope.md)**: Initial Scope
  - Defines Graft as task runner + dependency manager for knowledge bases

- **[Decision 0002](./decision-0002-git-refs-over-semver.md)**: Git Refs Over Semver
  - Use git refs as identity, don't require semantic versioning

- **[Decision 0003](./decision-0003-explicit-change-declarations.md)**: Explicit Change Declarations
  - Changes defined in structured YAML, not parsed from markdown

- **[Decision 0004](./decision-0004-atomic-upgrades.md)**: Atomic Upgrades
  - All-or-nothing upgrade operations with rollback capability

### Specification Enhancements (2026-01-05)

- **[Decision 0005](./decision-0005-no-partial-resolution.md)**: No Partial Dependency Resolution ~~(superseded)~~
  - Explicitly reject partial resolution (violates atomicity & reproducibility)
  - **Superseded by Decision 0007** - transitive deps no longer exist in flat-only model

- **[Decision 0006](./decision-0006-dependency-update-events.md)**: Dependency Update Event Strategy
  - Use org-wide push events with polling fallback for update notification
  - Zero upstream configuration—leverage existing graft.yaml declarations

### Dependency Model (2026-01-31)

- **[Decision 0007](./decision-0007-flat-only-dependencies.md)**: Flat-Only Dependency Model
  - Adopt flat-only resolution (no transitive dependencies)
  - Uses git submodules as required cloning layer
  - Simplifies lock file format
  - See [detailed analysis](../../../notes/2026-01-31-flat-only-dependency-analysis.md)

**Note**: Lock file ordering conventions are specified inline in the [Lock File Format specification](../graft/lock-file-format.md#ordering-convention) rather than as a separate ADR, as they represent a formatting convention rather than an architectural decision.

## Status Legend

- **accepted** - Decision is active and should be followed
- **rejected** - Decision was considered but not adopted (documents rationale)
- **superseded** - Decision replaced by a newer decision (references replacement)
- **deprecated** - Decision is being phased out (documents timeline)

## Creating New ADRs

When making significant architectural decisions:

1. Create new file: `decision-NNNN-brief-title.md`
2. Use next sequential number
3. Follow the standard format (see existing ADRs)
4. Link from relevant specifications
5. Update this README

## Related

- [Notes](../../../notes/) - Working notes, brainstorming, and design analysis
- [Specifications](../graft/) - Formal specifications implementing these decisions
- [Architecture Overview](../architecture.md) - High-level architecture

## References

- **ADR concept**: https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions
- **ADR templates**: https://github.com/joelparkerhenderson/architecture-decision-record
