---
status: stable
---
# Strategy: Metadata + generated views

Add small metadata to artifacts and generate indexes/views automatically.

## When it works well
- lots of artifacts
- queries like "all X related to Y" matter
- navigation is breaking down

## Minimal implementation
- keep metadata tiny (owner, status, tags)
- generate one index view, or maintain a curated index
- avoid schema creep

## Risks
- metadata theater
- brittle schemas

## Recommended safeguards
- [Linking policy](../policies/linking.md)
- [Playbook: evolve a knowledge base](../playbooks/evolve-kb.md)
