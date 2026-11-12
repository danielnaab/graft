---
status: stable
---
# Playbook: evolve a knowledge base

Safe evolution without over-architecting.

## Evidence-driven principle

Wait for clear, repeated pain before adding complexity.

Require:
- 3+ concrete examples of same pain
- Evidence of cost (time wasted, errors made)
- Understanding of minimal intervention

## Pain triggers

**Navigation pain** (can't find information):
- Threshold: 3+ "where is X?" in one week
- Minimal: add index or contents section
- Next: use intent-revealing structure (see [policy](../policies/intent-revealing-structure.md))

**Authority pain** (documents conflict):
- Threshold: any conflict between canonical sources
- Minimal: mark authority in frontmatter
- Next: add provenance to operational guidance

**Maintenance pain** (docs drift from reality):
- Threshold: weekly updates needed
- Minimal: add provenance links to sources
- Next: move closer to source or generate

**Handoff pain** (context loss between sessions):
- Threshold: every handoff costly (>30min)
- Minimal: add continue-here.md or tasks.md
- Next: add status layer with archive triggers

**Discovery pain** (can't search effectively):
- Threshold: daily grep for same patterns
- Minimal: add tags to frontmatter
- Next: add metadata and generate indexes

## Evolution moves (low to high cost)

1. Entrypoints (<1hr, easy to reverse)
2. Lifecycle markers (1-2hr, easy to reverse)
3. Provenance on claims (2-4hr, easy to reverse)
4. Navigation index (2-3hr, easy to reverse)
5. Temporal layers (3-5hr, moderate to reverse)
6. KB dependencies (4-6hr, moderate to reverse)
7. Metadata + generated views (8-16hr, harder to reverse)

## Decision framework

1. Name the pain (from list above)
2. Count occurrences (ensure real, not one-off)
3. Try minimal intervention
4. Observe impact
5. Iterate if pain persists

## Anti-patterns

Avoid:
- Premature infrastructure
- Complex hierarchies with few documents
- Elaborate metadata schemas with no queries
- Building UIs before understanding domain
- "We might need this later" thinking

The best knowledge base is the simplest one that solves actual problems.

Related:
- [Choose a strategy](choose-strategy.md)
- [Temporal layers policy](../policies/temporal-layers.md)
- [Generated content policy](../policies/generated-content.md)
