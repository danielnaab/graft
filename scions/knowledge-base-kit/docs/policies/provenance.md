---
status: stable
---
# Provenance policy

When a claim matters, show where it came from.

## When required
- operational guidance
- factual claims that drive decisions
- syntheses explaining system behavior

## Preferred references
- file + heading anchor
- file + stable symbol name

## Where provenance should appear
Use a dedicated `## Sources` section whenever possible:

```markdown
## Sources
- [Authority policy](policies/authority.md#conflict-resolution)
- [RFC 2119](https://datatracker.ietf.org/doc/html/rfc2119) - requirement level keywords
```

## Structured provenance (optional)
Where tools read provenance, frontmatter may replace the section. Use one or the other, not both.
- `sources` lists what the document derives from. Each entry has a `resource` (URL or path) and,
  when cited, a stable `id`.
- A claim cites a source with a footnote labelled with that `id`, such as `[^rfc2119]`.
- This is [OKF v0.2](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md)
  provenance (§5.1), usable without the rest of OKF.

Related:
- [Authority policy](authority.md)
- [Linking policy](linking.md)
