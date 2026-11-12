---
status: stable
---
# UI views as derived artifacts

User interfaces (UIs) are **not a separate knowledge organization strategy**.
They are **derived views** layered on top of an existing strategy.

This document explains when UI views are appropriate and how they should
co-evolve with a knowledge base without undermining its integrity.

---

## Core principle

> A UI is not the knowledge base.
> A UI is a view *over* the knowledge base.

The system of record remains:
- git
- markdown
- data files
- declared canonical sources

UIs may read from, generate, or edit those artifacts — but must not replace them.

---

## When UI views are justified

Introduce a UI only when there is **persistent, observed pain**, such as:

- **Navigation pain**
  - readers struggle to find relevant material
  - index pages are growing unwieldy

- **Query pain**
  - users repeatedly ask “show me all X related to Y”
  - filtering or cross-cutting views are hard to maintain manually

- **Synthesis pain**
  - overview documents are expensive to keep up to date
  - summaries are frequently stale

- **Contributor pain**
  - people don’t know where to add or update information
  - changes are avoided because edits feel risky

If none of these are present, prefer improving markdown navigation first
(see the [Linking policy](../policies/linking.md)).

---

## UI evolution ladder

UI capability should grow incrementally.

### 1. Markdown-only navigation
- better links
- maps of content
- curated indexes

This is the baseline and often sufficient.

---

### 2. Generated static views
- tables or indexes generated from metadata
- rendered summaries
- committed artifacts

Still fully reviewable in git.

---

### 3. Read-only interactive views
- search
- filters
- multiple synchronized perspectives

No direct editing of source files.

---

### 4. Assisted editing views
- form-based edits that write markdown or data files
- validation against policies before writing

Editing remains transparent and reviewable.

---

### 5. Agent-integrated UIs
- UI as a front-end to agent workflows
- humans approve changes before commit

This is the highest-risk tier and should be adopted cautiously.

---

## Non-negotiable invariants

Regardless of UI sophistication:

- git remains the system of record
- canonical sources remain authoritative
- lifecycle and provenance rules still apply
- UIs must not introduce hidden state

If a UI cannot uphold these invariants, it should not be introduced.

---

## Relationship to organization strategies

UI views can extend **any** strategy:

- handbook → navigable documentation portals
- atomic notes → graph or index views
- source-first synthesis → traceable summaries
- metadata + views → richer, queryable interfaces

UI choices should follow strategy decisions, not replace them.

---

## Related guidance

- [Playbook: evolve a knowledge base](../playbooks/evolve-kb.md)
- [Playbook: evolve a UI](../playbooks/evolve-ui.md)
- [Linking policy](../policies/linking.md)
- [Meta knowledge base](../../README.md)
