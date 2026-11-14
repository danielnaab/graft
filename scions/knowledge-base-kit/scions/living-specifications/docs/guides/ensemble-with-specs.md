---
status: working
last-validated: 2026-01-24
---

# Ensemble Sessions with Specifications

How to use specifications effectively in ensemble (mob) programming sessions.

## Before the Session

1. **Identify the spec**: Find or draft the spec for the feature being implemented
2. **AI can help**: Ask AI to draft a spec from the ticket/story if none exists
3. **Share ahead**: Post spec link in the session invite so participants can pre-read

## Starting the Session (First 10 Minutes)

1. **Open the spec** — display it where everyone can see
2. **Read through together** — ensure shared understanding
3. **Resolve Open Questions** — group consensus, right there, in real-time
4. **Update the spec** with any resolutions
5. **Agree on scope** — which behaviors are we implementing this session?

This prevents the anti-pattern of coding before alignment, which leads to verbal disagreements mid-session.

## During the Session

### Navigator-as-Scribe Pattern

Assign one navigator to update the spec as decisions are made:

- "We decided to return 404 instead of 403" → update Behavior section
- "Skipping rate limiting for v1" → add to Open Questions or Non-goals
- "Edge case: two active sessions?" → add to Edge Cases

**Why this works**:
- Navigators have cognitive bandwidth (not typing code)
- Decisions are verbal and immediate—easy to capture in the moment
- The spec becomes the group's shared memory
- Disagreements surface when written down ("wait, that's not what I meant")

### AI-Assisted Capture

The navigator can use AI to help:
- "Update the spec to reflect that decision"
- "Add this edge case to the behavior section"
- "What Open Questions do we still have?"

## After the Session

The spec should already reflect what was decided and built. No separate "go back and document" step needed.

1. **Verify spec is current** — quick scan for anything missed
2. **Commit spec changes** — in the same branch as code
3. **Update status** if appropriate (draft → working → stable)

## Bridging to Async

Specs are the **continuity artifact** between collaboration modes:

```
Ensemble session → updates spec → async developer picks up next increment
Async developer → updates spec → next ensemble reviews
```

If async developers will continue the work:
- Ensure Open Questions section reflects what's still unresolved
- Ensure Behavior section covers what was implemented vs. what remains
- Consider increment tags (`[v0.1]`, `[v0.2]`) for scoping

## AI Agent Handoff

When ensemble designs and AI implements between sessions:

1. **Ensemble writes/updates spec** — collaborative design, high-bandwidth
2. **AI implements against spec** — fast execution between sessions
3. **Next ensemble reviews** — AI's PR checked against the spec
4. **Spec updated** if implementation revealed gaps

The spec is the **interface between human design and AI execution**.

## What Ensemble Doesn't Need

- Formal sign-off (consensus is immediate and verbal)
- Async review periods (whole team is present)
- Status fields during session (everyone knows)
- Detailed spec commit history during session (session IS the review)

These ceremonies solve async coordination problems. Ensemble doesn't have those problems during the session—but the spec must be current *after* for async consumers.

## Related

- [Writing specs](writing-specs.md) — general spec authoring
- [Reviewing against specs](reviewing-against-specs.md) — the reviewer's perspective
- [Specification format](../format.md) — the template
- [Principles](../principles.md) — #8 (async-first, synchronous-to-resolve)

## Sources

- Personal experience: ensemble programming with spec-first approach
- Navigator-as-Scribe and mob-specific patterns from observation
