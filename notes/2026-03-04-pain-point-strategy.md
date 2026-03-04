---
status: done
purpose: "Strategic analysis of dogfooding pain points and optimal resolution path"
---

# Pain Point Resolution Strategy (2026-03-04)

## Pain Points (from dogfooding 2026-03-02)

1. No output preview for running commands in transcript log
2. `new-slice` slug extraction fragile
3. `scion start` fails — no `scions.start` config
4. No path from `scion create` to working in the scion

## Key Insight

Pain points 2-4 share a root cause: software-factory scripts are too complex to
work in a scion context, and the root graft.yaml has no scion config. Pain point
1 is independent grove UX work.

## Options Considered

### A. Thin scions.start command (skip software-factory refactor)

Add a simple `worker` command to root graft.yaml with `stdin: literal` prompt
telling claude to read the plan. Fixes pain points 3+4 in ~10 lines of config.
Fix new-slice to take slug as arg (pain point 2 in 10 min). Defer the full
software-factory refactor.

**Pro:** 30 minutes to unblock scion lifecycle. No engine changes.
**Con:** Two paths to implement (`:run implement` and `:scion start`). May drift.

### B. Full software-factory simplification first

Replace ~700 lines of bash prompt-construction scripts with `stdin: literal`
declarations. Then wire scions.start to the simplified implement command.

**Pro:** Single clean path. Eliminates technical debt.
**Con:** 3-4 day investment before scion lifecycle works.

### C. Targeted surgical fixes + defer rest

Fix new-slice slug (arg instead of extraction), add thin scions.start, then
invest in output preview as the real UX slice. Defer checkpoint UI, self-review,
conditional steps, step timeouts until demand appears.

**Pro:** Fast, focused, YAGNI-compliant.
**Con:** Leaves bash complexity in place.

## Decision

Option B — full software-factory simplification. The bash scripts are unnecessary
complexity that will keep causing friction. Eliminate them, then consider next
steps from a clean foundation.

## Pending Slices (deferred until demand)

- Grove checkpoint UI (plan exists, no code)
- Self-review / reflexion pattern (plan exists, no code)
- Conditional sequence steps (plan exists, no code)
- Step-level timeouts (plan exists, no code)
- Output preview in grove log (not yet designed)
