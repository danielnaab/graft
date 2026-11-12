---
status: stable
---
# Strategy: Atomic notes + maps of content

Many small notes (one idea per note) connected by index pages ("maps of content").

## When it works well
- research and exploration
- long-lived conceptual work

## Risks
- navigation debt
- unclear authority ("which note is true?")

## Minimal implementation
- a `notes/` directory
- a map-of-content page (e.g., `notes/index.md`)
- lifecycle markers (see [Lifecycle policy](../policies/lifecycle.md))

## Recommended safeguards
- keep stable guidance in curated docs once it settles
- use [Linking policy](../policies/linking.md) to keep navigation readable

Next:
- [Playbook: evolve a knowledge base](../playbooks/evolve-kb.md)
