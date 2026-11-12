---
status: stable
---
# Lifecycle policy

Recommended statuses:
- draft
- working
- stable
- deprecated

A knowledge base may declare its own list (`rules.lifecycle.statuses` in `knowledge-base.yaml`). Whatever it
declares, `living` is always valid: it marks a document kept current rather than finished, such as an index, a
status page, or a log of decisions that still hold, edited when one is replaced (see
[Temporal layers](temporal-layers.md#living-documents)). A dated decision record written once is historical, and
carries no status.

Raw sources and historical records are not revised, so they carry no status; their path marks
them (see [Temporal layers](temporal-layers.md)).

## What stable means
Stable knowledge should:
- be reviewed
- align with canonical sources
- include provenance where required

## Example frontmatter

```markdown
---
status: working
---
# Document Title
```

Related:
- [Authority policy](authority.md)
- [Provenance policy](provenance.md)
