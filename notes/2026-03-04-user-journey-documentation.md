---
status: considered
purpose: "Analysis of whether to add user journey/saga documentation to the spec-driven workflow"
decision: deferred
---

# User Journey Documentation (2026-03-04)

## Context

During dogfooding of the full scion lifecycle (create slice → create scion →
start worker → implement → review → fuse), we found 4 pain points that lived
in the seams between individually-specified features. This raised the question:
should we document end-to-end user journeys as a first-class artifact?

## Current documentation stack

- **Specs** (`docs/specifications/`) — what should happen when X
- **Slices** (`slices/`) — what are we building next (vertical, per-feature)
- **Reference** (`docs/cli-reference.md`, guides) — how does command Y work

The gap is the horizontal view — complete workflows crossing multiple commands,
specs, and user decisions.

## Benefits identified

- Expose integration gaps that vertical slices miss
- Define "done" for the product, not just the feature
- Serve as manual integration test checklists
- Anchor dogfooding sessions and onboarding

## Risks identified

- Maintenance burden: journeys cross many commands, break when any step changes
- Duplication: steps restate what specs and guides already say
- Scope creep: always one more journey to document
- False confidence: written journeys feel tested even when not exercised

## Proposed format (if adopted)

Single file (`docs/grove/journeys.md`), checklist-style, 5-15 lines per journey.
Each journey lists numbered steps (command names, not behavior descriptions),
a "last walked" freshness date, and links to known gaps. No narrative prose.
References specs rather than restating them.

## Decision

Deferred. The pain points log (`notes/2026-03-02-dogfood-pain-points.md`)
captures integration issues ad-hoc for now. If we find ourselves repeatedly
losing track of cross-feature regressions, revisit this with the proposed
lightweight format.
