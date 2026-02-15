---
status: deprecated
date: 2026-02-13
archived-reason: "All critical/high issues resolved"
---

# State Queries Stage 1 - Review

Condensed from detailed critique. All critical and high-priority issues have been resolved.

## Critical Issues (Resolved)

- **Temporal queries broken**: `--commit` only resolved hash, didn't checkout. Fixed with git worktree implementation.
- **Zero e2e tests**: only domain tests existed. Added 17 service-layer unit tests. E2e tests still recommended but not blocking.

## High Priority Issues (Resolved)

- **Missing dirty tree check**: temporal queries now fail fast if working tree dirty
- **Inconsistent timestamps**: all timestamps now UTC-aware
- **No command timeout**: configurable timeout added (default 300s)
- **Silent cache corruption**: now warns to stderr and deletes corrupted files
- **Missing security docs**: `shell=True` trust model documented

## Architecture Assessment

**Strengths**: clean domain model (frozen dataclasses), protocol-based DI, follows existing patterns, appropriate security model

**Remaining gaps**: no cache size management (unbounded growth), no command shorthand (`graft state coverage` as alias for `graft state query coverage`)

## Test Coverage After Improvements

| Layer | Coverage |
|-------|----------|
| Domain | 100% |
| Config parsing | good |
| State service | ~50% (unit tests) |
| CLI commands | low (no e2e) |

## Sources

- [State queries spec](../docs/specifications/graft/state-queries.md)
- [State service](../src/graft/services/state_service.py)
