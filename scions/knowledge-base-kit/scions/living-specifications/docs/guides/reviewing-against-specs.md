---
status: working
last-validated: 2026-01-24
---

# Reviewing PRs Against Specifications

How to review a pull request when specifications are the source of truth.

> **Context**: This guide applies to project repositories using the living specifications methodology. Example paths like `docs/specs/` refer to conventions in project repos.

## Before Reviewing

1. **Find the referenced spec**: Check PR description for spec link (e.g., "Implements `docs/specs/auth.md`")
2. **If no spec referenced**: Check if changed files have backlink annotations (`// spec: ...`)
3. **If no spec exists**: Consider whether this change needs one (behavior changes do; refactors don't)

## Review Checklist

### Does the code match the spec?

- [ ] All behavior statements in scope are implemented
- [ ] Edge cases listed in spec are handled
- [ ] Constraints (performance, security) are met
- [ ] No undocumented behavior added beyond spec

### Does the spec match the intent?

- [ ] If spec was updated in this PR, do the changes make sense?
- [ ] Are Open Questions resolved appropriately?
- [ ] Is the Intent section still accurate?

### Is the spec current after this change?

- [ ] `last-verified` date updated if spec was validated
- [ ] Status reflects reality (draft → working → stable)
- [ ] No stale behavior statements remain

## The Workflow

```
1. PR opened
   - Author references spec in PR description
   - If spec updated, diff shows both spec and code changes

2. Reviewer checks alignment
   - Spec behavior → implementation (does code do what spec says?)
   - Implementation → spec (does code do anything spec doesn't mention?)

3. Feedback
   - "Spec says X but code does Y" — clear, actionable
   - "Code does Z but spec doesn't mention it" — needs spec update or code removal

4. Approval
   - Both spec and code reviewed together
   - No separate "spec approval" ceremony needed
```

## When to Flag

- **Spec-impacting change without spec update**: "This PR changes login behavior but `docs/specs/auth.md` still describes the old flow"
- **Undocumented behavior**: "This adds retry logic not mentioned in any spec—should we update the spec?"
- **Constraint violation**: "Spec says max 100ms response time but this adds a blocking call"

## When NOT to Require Spec Updates

- Pure refactors (no behavior change)
- Test-only changes
- Documentation fixes
- Dependency updates (unless they change behavior)
- Build/CI configuration

**Ceremony scales with impact.** Don't be spec-police on trivial changes.

## Backlink Verification

If the project uses backlink annotations, check:
```python
# spec: docs/specs/auth.md
# spec-section: Behavior/Login
def login(credentials):
    ...
```

- Is the backlink accurate? (Does this code actually implement that spec section?)
- Are new behavioral functions annotated with spec references?

## Related

- [Writing specs](writing-specs.md) — the author's perspective (includes Step 9: verify against spec)
- [Verification](../verification.md) — automated drift detection
- [Artifact relationships](../relationships.md) — the three-layer model

## Sources

- Personal experience: Reviewing implementations against specifications
