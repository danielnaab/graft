---
status: working
last-validated: 2026-01-24
---

# Writing Specifications

How to write a new specification or update an existing one.

> **Context**: This guide is for use in project repositories that adopt the living specifications methodology. The paths below (e.g., `docs/specs/`) are conventions for project repos.

## When to Write a Spec

| Change Type | Spec Impact | Action |
|-------------|------------|--------|
| New feature | New spec needed | Write spec first, or combined with implementation |
| Behavior change | Must update | Update in same PR as code change |
| Bug fix (spec wrong) | Correction | Update spec to match intended behavior |
| Bug fix (code wrong) | None | Spec was already correct |
| Refactor | None | No behavior change, spec unchanged |

**Ceremony scales with impact.** Trivial changes need no spec update; behavior changes do.

## Steps

> **These steps are a thinking guide, not a strict sequence.** In practice, you'll iterate — writing Intent, jumping to Behavior, refining Non-goals, returning to add Open Questions. Sources inform your design from the start, even though they appear late in this list.

### 0. Orient

Before writing, gather context:
- Read the [specification format](../format.md) template
- Check for related specs that your new spec might reference or overlap with
- Identify the sources (requirements, principles, prior decisions) that will inform the design

### 1. Create the File

Create a new spec in your project's `docs/specs/` directory:

```bash
mkdir -p docs/specs
touch docs/specs/[feature-name].md
```

Set frontmatter:
```yaml
---
status: draft
last-verified: [today]
owners: [your-name]
---
```

### 2. Write the Intent

Answer: why does this exist? What problem does it solve? One paragraph is usually enough.

### 3. Define Non-goals

Explicitly state what's out of scope. This prevents scope creep and clarifies boundaries for reviewers and implementers.

### 4. Specify Behavior

Use Given/When/Then structure for each scenario, wrapped in gherkin code blocks:

```markdown
### Login

\`\`\`gherkin
Given valid credentials
When the user submits the login form
Then they receive a session token and are redirected to dashboard
\`\`\`

### Edge Cases

- Empty email: returns 400 with "email required" message
- Expired account: returns 403 with reactivation link
```

Gherkin code blocks provide better visual separation and GitHub syntax highlighting. Edge cases can remain as bullet points when they're simple one-liners.

### 5. Add Constraints

Non-functional requirements that might otherwise get lost:
- Performance bounds
- Security requirements
- Compatibility needs

### 6. Mark Open Questions

For anything unresolved, use checkboxes:
```markdown
## Open Questions
- [ ] Should rate limiting apply per-user or per-IP?
- [ ] What's the session token expiry? (team decision needed)
```

These explicitly invite collaboration without blocking progress.

### 7. Record Decisions

As you make design choices, capture rationale inline:
```markdown
## Decisions
- [Date]: [Decision and brief rationale]
```

This prevents "why is it like this?" archaeology later.

### 8. Cite Sources and Link Related

Include provenance for behavior decisions:
```markdown
## Sources
- [OWASP Auth Guidelines](URL) - Session management patterns
- Personal experience: Project X - Rate limiting approach
```

Link to related specs, docs, or principles:
```markdown
## Related
- [Other Spec](other-spec.md) — shared constraint
- [Principles](../principles.md) — relevant principle numbers
```

### 9. Verify Against Implementation

After implementing (or if spec and implementation are developed together), re-read each behavior statement and edge case against the actual code:

- Does the output structure match what the code produces?
- Do edge case descriptions match actual error paths?
- Are constraint claims (performance, dependencies) still accurate?

This step catches drift introduced during implementation — the most common source of spec inaccuracy is "wrote the spec, then built something slightly different."

## Updating an Existing Spec

1. Find the spec (via backlink in code, or in your project's `docs/specs/`)
2. Update behavior statements to reflect the change
3. Update `last-verified` date in frontmatter
4. Include spec update in the same PR as code change
5. If status was `working` or `stable`, keep it (the update is part of the implementation)

## Key Reframe

**Spec review IS design review.** It's the same discussion that would happen in Slack or meetings, but captured durably. Not extra work—better-preserved work.

## Anti-patterns

- **Spec-police**: Blocking PRs for trivial missing spec updates
- **Spec-after-the-fact**: Writing specs only to document what's already built
- **Spec-as-ticket**: Treating specs as one-time artifacts that get "done"
- **Approval-ceremony**: Requiring layers beyond normal PR review

## Related

- [Specification format](../format.md) — the template reference
- [Principles](../principles.md) — normative rules
- [Reviewing against specs](reviewing-against-specs.md) — the reviewer's perspective

## Sources

- [Specification format](../format.md) — template this playbook walks through
