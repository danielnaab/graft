---
status: working
---

# Artifact Relationships

How specifications relate to tests, acceptance criteria, and code.

## The Three-Layer Model

```
SPEC (human-readable, durable, collaborative)
  ↓ maps to
TESTS (executable, durable, CI-enforced)
  ↓ verified by
CODE (implementation, durable, reviewed)
```

Connected by backlinks flowing upstream:
- Tests reference specs: `// spec: docs/specs/auth.md`
- Code references specs where non-obvious
- Specs reference nothing downstream (views computed from backlinks)

## Specs vs. Acceptance Criteria

| Aspect | Spec | Acceptance Criteria |
|--------|------|-------------------|
| Lifespan | Life of the feature | Life of the story |
| Granularity | Complete behavior | Incremental slice |
| Location | Repo (durable) | Ticket (ephemeral) |
| Purpose | Source of truth | Planning/scoping tool |

**Specs subsume acceptance criteria.** AC becomes a reference to spec sections:
- "Implement behaviors marked `[v0.2]` in `docs/specs/auth.md`"
- Ticket doesn't restate requirements—it points to the spec
- No durable information stored in ephemeral containers

## Specs vs. Tests

- **Specs** are readable by non-developers, capture intent and context
- **Tests** are executable, catch regressions, enforced by CI
- Together they form a complete verification chain
- Neither replaces the other

## Why Not BDD/Gherkin?

Given/When/Then is valuable as a **thinking structure** in plain markdown. But the Gherkin tooling ecosystem (step definitions, Cucumber/SpecFlow) adds complexity without proportional benefit for most teams:
- Step definitions are their own maintenance burden
- Hard to be both precise-enough-to-execute AND clear-enough-to-read
- Non-developers rarely read feature files in practice

**Our approach**: Structured prose that maps to tests, without trying to *be* tests.

## Related

- [Specification format](format.md) — the template with behavior sections
- [Verification](verification.md) — how drift between layers is detected
- [Principles](principles.md) — #7 (separate what/why/how), #9 (tests complement specs)

## Sources

- Dan North, "Introducing BDD" (2006) — Given/When/Then as thinking structure
- Personal experience: Backlink-based traceability in knowledge bases
