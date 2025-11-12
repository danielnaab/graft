# 5. Abstractions define implementations, not vice versa

Date: 2025-11-12

## Status

Accepted

## Context

As systems grow, they face a common degradation pattern: concrete implementations become the de facto specification, while abstract concepts drift into vague aspirations. This happens when:

1. **Documentation lags behind code** — Developers update implementations but not conceptual docs
2. **No clear source of truth** — Is the architecture doc or the actual code authoritative?
3. **Reverse dependencies creep in** — Abstract docs start referencing specific implementation details
4. **Onboarding is unclear** — New contributors don't know whether to read concepts or code first
5. **Refactoring becomes risky** — Changing implementations feels like it might violate unstated principles

The result: the system becomes harder to understand, maintain, and evolve. The gap between "what we say the system is" and "what the system actually is" grows wider.

For Graft specifically, this risk is acute because:
- We're building a tool for managing derived artifacts
- We need clear separation between concepts (what is a graft?) and implementation (how do we execute it?)
- We may rewrite implementations (even in other languages) as we learn
- AI agents will work on this codebase and need clear hierarchies
- We're dogfooding: using Graft to document Graft

## Decision

We will enforce a strict **dependency rule** across both information architecture and code organization:

**Abstractions define the system. Implementations depend on abstractions. Never the reverse.**

### In Information Architecture

**Abstract, authoritative documents:**
- `docs/concepts.md` — Core domain model (what is an artifact, material, derivation, provenance?)
- `docs/philosophy-of-design.md` — Design principles and values
- `docs/adr/` — Architecture decisions
- `docs/use-cases.md` — Problem space and patterns

**Concrete, derived documents:**
- `docs/architecture.md` — Current implementation structure (depends on concepts + ADRs)
- `docs/cli-reference.md` — Specific commands (depends on concepts)
- `docs/implementation-strategy.md` — How to build (depends on architecture + philosophy)
- `docs/testing-strategy.md` — How to test (depends on ADR 0004)

**Dependency flow:**
```
Concepts + Philosophy + ADRs (abstract, stable)
          ↓
Architecture + Use Cases (bridge layer)
          ↓
Implementation + CLI Reference (concrete, changeable)
```

If `architecture.md` changes, `concepts.md` should rarely need to change. If `concepts.md` changes, everything below must be reconsidered.

### In Code Organization

**Abstract layer (domain):**
- `src/graft/domain/` — Pure domain entities, no external dependencies
- Defines WHAT the system is (artifacts, materials, derivations, policy)
- Immutable, frozen dataclasses
- No framework dependencies

**Interface layer (adapters):**
- `src/graft/adapters/` — Protocol definitions for external systems
- Depends on domain entities
- Defines contracts, not implementations

**Implementation layer (services + CLI):**
- `src/graft/services/` — Use case orchestration
- `src/graft/cli.py` — Command handlers
- Depend on domain + adapters
- Can be rewritten without changing domain

**Dependency flow:**
```
Domain (abstract, stable)
   ↓
Adapters (protocols)
   ↓
Services + CLI (concrete, changeable)
```

### Using Graft to Enforce This

We will use **Graft itself** to manage Graft's documentation:

1. **ADRs and concept docs are materials** for architecture.md, implementation-strategy.md
2. **When abstractions change**, derived docs are regenerated (or flagged as dirty)
3. **Provenance tracks dependencies** — We can trace implementation decisions to architectural rationale
4. **Graft dogfooding** proves the patterns work

This creates a **self-documenting, self-evolving system** where:
- Changing an ADR triggers updates to dependent docs
- We discover what Graft needs by using it
- Our own documentation is a reference implementation of Graft patterns

### The Rule in Practice

**When adding a feature:**
1. First: Is this a new concept? Update `concepts.md`
2. Second: Does this require an architectural decision? Write an ADR
3. Third: Implement in appropriate layer (domain → adapters → services → CLI)
4. Fourth: Update derived docs (architecture, CLI reference, etc.)
5. Fifth: Use Graft to detect what else needs updating

**When refactoring:**
1. Ask: Are we changing abstractions or implementations?
2. If abstractions: Update concept docs, expect cascade
3. If implementations only: Concrete docs change, abstractions stable

**When onboarding:**
1. Read: Concepts → Philosophy → ADRs
2. Then: Architecture → Use Cases
3. Finally: Implementation details

## Consequences

**Positive:**

- **Clear source of truth** — Abstractions are authoritative; implementations must conform
- **Sustainable growth** — New features fit into existing conceptual model or extend it explicitly
- **Refactoring freedom** — Can rewrite implementations (even in other languages) without breaking concepts
- **Better onboarding** — Clear hierarchy: concepts first, details later
- **Dogfooding proves patterns** — Using Graft for Graft validates our approach
- **Provenance of decisions** — Trace implementation to rationale via dependency graph
- **AI-agent friendly** — Clear structure helps agents understand and modify correctly
- **Multiple implementations possible** — Could have Python + Rust implementations sharing same domain model

**Negative:**

- **Requires discipline** — Must think "where does this belong?" for every change
- **Can feel like overhead** — Small changes might require touching multiple layers
- **Abstractions can be wrong** — Getting domain model right takes iterations
- **Documentation debt** — If we don't use Graft to manage docs, dependencies drift

**Neutral:**

- **Some duplication** — Same concept appears in abstract docs and concrete code (but with clear dependency direction)
- **Graft dogfooding is meta** — Using Graft to manage Graft documentation requires Graft to be working
- **Abstractions evolve slowly** — This is by design; stability enables confidence

## Implementation Notes

### Information Architecture

Create graft artifacts for key documents:
```yaml
# docs-architecture/graft.yaml
graft: architecture-doc
inputs:
  materials:
    - { path: "../concepts.md", rev: HEAD }
    - { path: "../philosophy-of-design.md", rev: HEAD }
    - { path: "../adr/*.md", rev: HEAD }
derivations:
  - id: architecture
    # Template or manual derivation
    outputs:
      - { path: "./architecture.md" }
```

When concept docs change → architecture.md becomes dirty → we propagate updates.

### Code Organization

Enforce via:
- Mypy (ensure no circular imports)
- Architectural tests (verify domain has no external dependencies)
- Code review (check dependency direction)
- Clear README in each module explaining its level

### Evolution

- Concepts evolve through ADRs (explicit decisions)
- Implementations evolve through PRs (reviewed changes)
- Graft usage reveals what we need next
- Documentation stays current via Graft workflows

## Rationale

**Why this matters more for Graft than typical projects:**

1. **We're building a provenance tool** — We must practice what we preach
2. **AI agents will contribute** — Clear structure is essential for agent understanding
3. **Potential language rewrite** — Keeping abstractions separate enables this
4. **Dogfooding is the test** — If Graft can't manage its own docs, how can users trust it?
5. **Long-term evolution** — This project needs to grow sustainably for years

**The meta-benefit:**

Using Graft to manage Graft creates a virtuous cycle:
- We discover missing features (because we need them)
- We validate patterns (by using them ourselves)
- We ensure documentation quality (we rely on it)
- We build confidence (eating our own dog food)

This ADR is itself an abstraction that implementations will depend on. When we add a feature, we'll ask: "Does this follow ADR-0005?" If not, we either change the implementation or update the ADR.

**This is the discipline that enables sustainable growth.**
