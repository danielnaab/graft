---
status: stable
---
# Linking policy

Links exist to improve navigation and trust, especially when browsing in Forgejo/GitHub.

## Navigation links (required)
If a reader should click something, use a real Markdown link.

Correct:
- [Meta knowledge base](../../README.md)

Incorrect:
- `../../README.md`

Write link text that says where the link goes ("the [lifecycle policy](lifecycle.md)"), not "here".

## Provenance links
In a `## Sources` section, use real links whenever possible.

## Exported files
A repository that others graft exports some of its files. An exported file links only to exported files:
a link into an excluded path (`scions/`, `specs/`, `tests/`) works in your repository and breaks in every
consumer's copy. Link to such a file by URL, at the tag you pin rather than `main`, and update the tag
when you upgrade. Files you don't export (AGENTS.md, specs) may link into your `scions/`.

## Backticks are for literal text
- paths and globs being discussed as text (`docs/**`)
- commands, flags, keys, environment variables and identifiers the reader types or sees
- code snippets

Related:
- [Provenance policy](provenance.md)
- [Style policy](style.md)
- [Managing KB dependencies](../playbooks/manage-kb-dependencies.md#provide-one)
