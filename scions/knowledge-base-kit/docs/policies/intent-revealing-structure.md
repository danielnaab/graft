---
status: stable
---
# Intent-revealing structure policy

File and directory names should communicate purpose without opening files.

## File naming

Name files to reveal content. Avoid opaque names that require opening to understand.

- `secrets-management.md` - clear purpose
- `decision-0003.md` - reveals nothing (avoid)
- `misc-notes.md` - catch-all name (avoid)

## Directory naming

Name directories to reveal purpose, not sequence or time.

- `integrations/` - clear purpose
- `part-2/` - sequence only (avoid)
- `old/` - temporal marker (avoid)

## Hybrid names

Numbers are acceptable when combined with descriptive content:

- `decision-0001-auth-flow.md` - number + intent (acceptable)
- `decision-0001.md` - number only (avoid)

The test: can someone unfamiliar guess the content from the name?

## When to apply

- Repository grows beyond 5-10 files
- Multiple people or agents navigate the knowledge base
- Repeated "where is X?" questions (3+ per week)

## Trade-offs

Accepts longer names and occasional renames when purpose shifts.
Gains faster navigation, self-documenting structure, clearer handoffs.

Related:
- [Evolve KB playbook](../playbooks/evolve-kb.md) - navigation pain triggers
- [Linking policy](linking.md)
- [Exploration: detailed examples](https://github.com/danielnaab/knowledge-base-kit/blob/main/notes/2026-01-11-intent-revealing-structure.md)
