# 1. Record architecture decisions

Date: 2025-11-10

## Status

Accepted

## Context

We need to record the architectural decisions made on this project so that:

1. Future contributors understand the reasoning behind design choices
2. We can track the evolution of the architecture over time
3. We avoid revisiting settled decisions without good reason
4. New team members can quickly understand the "why" behind the codebase structure

Architecture Decision Records (ADRs) provide a lightweight, version-controlled way to document significant architectural decisions. Each ADR describes a decision, its context, and its consequences.

## Decision

We will use Architecture Decision Records, as described by Michael Nygard in his article "Documenting Architecture Decisions" (http://thinkrelevance.com/blog/2011/11/15/documenting-architecture-decisions).

An architecture decision record is a short text file in a format similar to an Alexandrian pattern. Each record describes a set of forces and a single decision in response to those forces.

ADRs will be stored in `docs/adr/` and numbered sequentially. The format is:

```markdown
# [number]. [Title]

Date: YYYY-MM-DD

## Status

[Proposed | Accepted | Deprecated | Superseded by ADR-XXXX]

## Context

[What is the issue that we're seeing that is motivating this decision or change?]

## Decision

[What is the change that we're proposing and/or doing?]

## Consequences

[What becomes easier or more difficult to do because of this change?]
```

## Consequences

**Positive:**
- Architectural decisions are explicitly documented and version-controlled
- Context is preserved for future reference
- Onboarding new contributors becomes easier
- Decisions can be referenced in code reviews and discussions

**Negative:**
- Requires discipline to maintain
- Adds a small overhead to architectural changes
- May accumulate outdated records (mitigated by status field)

**Neutral:**
- ADRs are immutable once accepted; new decisions supersede old ones rather than editing them
- Superseded ADRs remain in the repository for historical context
