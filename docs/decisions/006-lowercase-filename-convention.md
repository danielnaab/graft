# ADR 006: Lowercase Filename Convention

**Status**: Accepted
**Date**: 2026-01-04
**Decision Makers**: Development team
**Tags**: documentation, conventions, user-experience

---

## Context

The project initially used ALL_CAPS filenames for documentation files (tasks.md, continue-here.md, cli-reference.md). This created several issues:

1. **Shouting effect** - ALL_CAPS filenames feel aggressive and unfriendly
2. **Inconsistency** - Mix of ALL_CAPS and lowercase created confusion
3. **Deviation from modern standards** - Most modern projects use lowercase-with-hyphens
4. **Poor readability** - UNDER_SCORES harder to scan than hyphens
5. **Not following meta-knowledge-base** - Our upstream conventions use lowercase

### Meta-Knowledge-Base Conventions

Examining `.graft/meta-knowledge-base`, we found:
- All documentation uses lowercase-with-hyphens
- Examples: `atomic-notes.md`, `planning-procedure.md`, `decision-0001-scope.md`
- Only exception: `README.md` (universal standard)

### Industry Practice

Modern projects (Next.js, Tailwind CSS, Vite, Rust book) use:
- `README.md`, `LICENSE` - Universal standards in caps
- `contributing.md`, `changelog.md` - Lowercase for other docs
- Hyphens over underscores for multi-word files

## Decision

**Use lowercase-with-hyphens for all filenames except universal standards.**

### Filename Rules

1. **Universal standards remain capitalized:**
   - `README.md` - Project root introduction
   - `LICENSE` - License file

2. **All other files use lowercase-with-hyphens:**
   - Root: `tasks.md`, `continue-here.md`
   - Docs: `cli-reference.md`, `configuration.md`, `contributing.md`
   - Guides: `user-guide.md`
   - Status: `implementation.md`, `workflow-validation.md`
   - Decisions: `001-decision-name.md` (ADR format)

3. **Use semantic, friendly names:**
   - `contributing.md` instead of `contributing.md`
   - `index.md` instead of `index.md`
   - `architecture.md` instead of `architecture.md`

4. **Date-based notes use ISO 8601:**
   - `notes/2026-01-04-topic-name.md`

### Specific Renames

```
# Root
tasks.md              → tasks.md
continue-here.md      → continue-here.md
PR_DESCRIPTION.md     → pr-description.md

# Documentation
docs/cli-reference.md          → docs/cli-reference.md
docs/configuration.md          → docs/configuration.md
docs/index.md             → docs/index.md
docs/architecture.md      → docs/architecture.md
docs/guides/user-guide.md      → docs/guides/user-guide.md
docs/guides/contributing.md → docs/guides/contributing.md

# Status
status/workflow-validation.md      → status/workflow-validation.md
status/implementation.md  → status/implementation.md
status/gap-analysis.md           → status/gap-analysis.md
status/phase-8.md → status/phase-8.md
```

## Consequences

### Positive

1. **Friendlier** - Files don't shout at users
2. **Consistent** - Single, clear convention throughout project
3. **Readable** - Hyphens easier to scan than underscores
4. **Standard** - Follows modern open source practices
5. **Meta-KB compliant** - Matches upstream conventions
6. **Semantic** - Names describe purpose clearly

### Negative

1. **Breaking change** - All documentation links need updating
2. **Git history** - File renames affect blame/history (mitigated by `git log --follow`)

### Neutral

1. **New contributors** - Must learn convention (documented in this ADR)
2. **Documentation overhead** - Must update this ADR if convention changes

## Implementation

1. Create this ADR to document decision
2. Rename all files according to convention
3. Update all cross-references in documentation
4. Update meta-references (docs/index.md documentation map)
5. Verify all links work
6. Commit with clear message explaining changes

## Compliance

This decision ensures compliance with:
- **Meta-knowledge-base**: Follows example projects in `.graft/meta-knowledge-base/examples/`
- **Style policy**: Professional, friendly, not aggressive
- **Linking policy**: Consistent, predictable file references

## Future Considerations

- When adding new files, follow lowercase-with-hyphens convention
- Use semantic names that describe purpose
- Prefer hyphens over underscores for readability
- Consider renaming if a file's purpose changes significantly

## References

- Meta-knowledge-base examples: `.graft/meta-knowledge-base/examples/`
- Meta-KB style policy: `.graft/meta-knowledge-base/policies/style.md`
- Industry examples: Next.js, Tailwind CSS, Vite, Rust documentation

---

**Supersedes**: No prior ADR
**Related**: ADR-004 (protocol-based dependency injection - similar focus on consistency)
