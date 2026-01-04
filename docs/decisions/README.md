# Architectural Decision Records (ADRs)

This directory contains Architectural Decision Records (ADRs) documenting key design decisions in the graft project.

## What are ADRs?

ADRs are documents that capture important architectural decisions along with their context and consequences. They help:

- **Preserve Context**: Why was this decision made?
- **Enable Onboarding**: Help new contributors understand the design
- **Prevent Revisiting**: Document why alternatives were rejected
- **Track Evolution**: See how the architecture evolved over time

## Format

Each ADR follows this structure:

```markdown
# ADR NNN: Title

**Status**: Accepted | Rejected | Superseded | Deprecated
**Date**: YYYY-MM-DD
**Deciders**: Who made this decision
**Context**: When this decision was made

## Context
The situation requiring a decision...

## Decision
What we decided to do...

## Consequences
Positive and negative outcomes...

## Alternatives Considered
What other options we evaluated...

## References
Related docs, code, discussions...
```

## Index

### Core Architecture

- **[ADR 004: Protocol-Based Dependency Injection](004-protocol-based-dependency-injection.md)**
  - Why we use Python Protocols instead of class inheritance
  - Status: Accepted
  - Impact: All abstractions (Git, FileSystem, etc.)

- **[ADR 005: Functional Service Layer](005-functional-service-layer.md)**
  - Why services are pure functions instead of classes
  - Status: Accepted
  - Impact: All service modules

### CLI Design

- **[ADR 001: Require Explicit Ref in Upgrade](001-require-explicit-ref-in-upgrade.md)**
  - Why `graft upgrade` requires `--to <ref>` flag
  - Status: Accepted
  - Deviation from spec: Spec suggested optional flag

### Snapshot & Rollback

- **[ADR 002: Filesystem Snapshots for Rollback](002-filesystem-snapshots-for-rollback.md)**
  - Why we use filesystem copies instead of git
  - Status: Accepted
  - Impact: Rollback implementation

- **[ADR 003: Snapshot Only Lock File](003-snapshot-only-lock-file.md)**
  - Why we snapshot only graft.lock, not full workspace
  - Status: Accepted
  - Impact: What gets restored on rollback

## Decision Log

| ADR | Title | Status | Date | Impact |
|-----|-------|--------|------|--------|
| 001 | Require Explicit Ref in Upgrade | Accepted | 2026-01-04 | CLI design |
| 002 | Filesystem Snapshots for Rollback | Accepted | 2026-01-04 | Snapshot impl |
| 003 | Snapshot Only Lock File | Accepted | 2026-01-04 | Rollback scope |
| 004 | Protocol-Based Dependency Injection | Accepted | 2026-01-04 | Core arch |
| 005 | Functional Service Layer | Accepted | 2026-01-04 | Service design |

## Guidelines for New ADRs

### When to Create an ADR

Create an ADR when making a decision that:
- Affects the overall architecture
- Is hard to reverse later
- Has significant trade-offs
- Deviates from common patterns
- Might be questioned in the future

### When NOT to Create an ADR

Don't create ADRs for:
- Implementation details (use code comments)
- Temporary decisions (use TODOs)
- Obvious choices (no trade-offs)
- Team process decisions (use team docs)

### Numbering

- Use sequential numbers: 001, 002, 003, etc.
- Never reuse numbers (even if an ADR is rejected)
- Numbers indicate chronological order, not importance

### Status Values

- **Accepted**: Decision is final and implemented
- **Proposed**: Decision is under consideration
- **Rejected**: Decision was considered but not chosen
- **Superseded**: Replaced by a newer ADR (reference it)
- **Deprecated**: No longer relevant but kept for history

## Template

```markdown
# ADR NNN: [Title]

**Status**: [Proposed | Accepted | Rejected | Superseded | Deprecated]
**Date**: YYYY-MM-DD
**Deciders**: [Who was involved]
**Context**: [When/why this decision was needed]

## Context

[Describe the situation, problem, or need that triggered this decision]

## Decision

[Describe what you decided to do]

## Consequences

### Positive
- [Good outcome 1]
- [Good outcome 2]

### Negative
- [Trade-off 1]
- [Trade-off 2]

## Alternatives Considered

### Alternative 1: [Name]
**Pros**: ...
**Cons**: ...
**Rejected**: [Why]

### Alternative 2: [Name]
**Pros**: ...
**Cons**: ...
**Rejected**: [Why]

## Related Decisions

- [Related ADR 1]
- [Related ADR 2]

## References

- [Code implementation]
- [Specification]
- [External resources]
```

## Further Reading

- [Architectural Decision Records by Michael Nygard](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions)
- [ADR GitHub Organization](https://adr.github.io/)
- [When to Write an ADR](https://engineering.atspotify.com/2020/04/when-should-i-write-an-architecture-decision-record/)

---

Last Updated: 2026-01-04
