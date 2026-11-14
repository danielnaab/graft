# Session: Examples and dogfooding

**Date**: 2026-01-26
**Status**: working

## Summary

Applied meta-KB patterns to itself, making it a reference implementation.

## Changes made

### Repository cleanup
- Removed `starter-kits/` - premature templates before patterns were proven
- Removed `examples/minimal/` and `examples/graft-project/` - replaced with strategy-specific examples

### New strategy examples
Created 4 examples demonstrating each organizational strategy:
- `examples/team-handbook/` - Handbook strategy (curated onboarding)
- `examples/research-notes/` - Atomic notes + maps (research/exploration)
- `examples/meal-planner/` - Source-first synthesis (canonical data)
- `examples/api-catalog/` - Metadata + views (queryable items)

### AGENTS.md convention
- Changed from embedded agent instructions in README to dedicated `AGENTS.md` files
- `examples/meal-planner/AGENTS.md` demonstrates the pattern
- Root `AGENTS.md` added to meta-KB itself

### Dogfooding improvements
- Renamed `explorations/` to `notes/` (temporal layers policy)
- Added lifecycle status to all policies and playbooks
- Added concrete examples to abstract policies
- Updated `knowledge-base.yaml` with agent entrypoint and notes path
- Fixed `playbooks/README.md` to list all 8 playbooks

## Sources

- [Temporal layers policy](../docs/policies/temporal-layers.md)
- [Lifecycle policy](../docs/policies/lifecycle.md)
- [Examples README](../examples/README.md)
