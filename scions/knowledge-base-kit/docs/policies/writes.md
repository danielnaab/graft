---
status: stable
---
# Write boundaries policy

Agents and automation need clear boundaries.

A domain repo may specify allow/deny globs in its `knowledge-base.yaml`:

- `allow`: where an agent may create/edit content
- `deny`: where an agent must never write (secrets, prod configs)

If not specified, agents should be conservative and prefer documentation-only edits.

## Example configuration

```yaml
rules:
  writes:
    allow: ["docs/**", "notes/**"]
    deny: ["secrets/**", ".env*", "*.key"]
```

Related:
- [Meta knowledge base](../../README.md)
