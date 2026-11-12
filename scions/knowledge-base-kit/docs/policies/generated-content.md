---
status: stable
---
# Generated content policy

Default: handwrite documentation. Generate only when pain is clear.

## Pain triggers

Generate when you have:
- **Synchronization pain**: same info in multiple places, frequent out-of-sync errors
- **Scale pain**: manual indexing burdensome (50+ docs), formatting errors in large sets
- **Consistency pain**: repeated structure with variations, human formatting errors

Threshold: weekly sync burden or errors from staleness.

## Generation levels

**Level 1: Generated indexes**
- Generate ToC, navigation from existing content
- Low risk, easy maintenance
- Example: ADR index table from frontmatter

**Level 2: Extracted summaries**
- Extract from code (signatures, types, docstrings)
- Render in documentation context
- Example: CLI reference from decorators

**Level 3: Fully generated**
- Entire documents from structured sources
- Human edits in separate overlays
- Example: API docs from OpenAPI spec

## Mark generated content

Always mark what's generated:

```markdown
<!-- AUTO-GENERATED from scripts/gen.py -->
<!-- Last generated: 2026-01-05 -->
[generated content]
<!-- END AUTO-GENERATED -->
```

Or in frontmatter:
```yaml
generated:
  script: scripts/gen.py
  last_run: 2026-01-05
```

## Separate concerns

- Generated: structure, data, signatures
- Handwritten: concepts, examples, guidance

## Version control

- Commit generated docs (for diffs and history)
- Include generation script in repo
- Document how to regenerate
- CI validates generated docs are current

Related:
- [Authority policy](authority.md)
- [Lifecycle policy](lifecycle.md)
