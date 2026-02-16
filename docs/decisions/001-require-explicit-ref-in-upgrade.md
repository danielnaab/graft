---
status: accepted
date: 2026-01-04
---

# ADR 001: Require Explicit Ref in Upgrade Command

**Deciders**: Implementation team
**Context**: Phase 8 CLI implementation

## Context

The specification suggests that `graft upgrade <dep>` without `--to` should default to "latest" version. However, this presents several design challenges:

1. **Ambiguity**: What is "latest"?
   - Latest commit on the default branch?
   - Latest semantic version tag?
   - Latest change in graft.yaml?

2. **Safety**: Automatic upgrades without user confirmation can be dangerous
   - Breaking changes might be applied unintentionally
   - User might not be aware of what version they're getting

3. **Predictability**: Commands should be explicit about their effects
   - Users should know exactly what will happen
   - Prevents "works on my machine" scenarios

## Decision

We require the `--to <ref>` flag for all `graft upgrade` commands.

```bash
# Required
graft upgrade my-dep --to v2.0.0

# Not supported (will error)
graft upgrade my-dep
```

## Consequences

### Positive

- **Explicit Intent**: Users must consciously choose which version to upgrade to
- **No Surprises**: Command behavior is always predictable
- **Better Error Messages**: Can provide helpful suggestions when flag is missing
- **Safer Defaults**: Prevents accidental breaking changes

### Negative

- **Slightly More Verbose**: Users must always specify `--to`
- **Deviates from Spec**: Original specification suggested optional `--to`

### Mitigation

- Provide clear error message when `--to` is missing
- Error message suggests: "Use 'graft changes <dep>' to see available versions"
- `graft changes` command makes it easy to discover what to upgrade to

## Alternatives Considered

1. **Default to latest tag**: Would require parsing semantic versions, fragile
2. **Default to HEAD of branch**: Could be unstable, breaks predictability
3. **Default to latest change**: Unclear what "latest" means in this context
4. **Interactive prompt**: Would break scripting, adds complexity

## Related Decisions

- Uses `graft changes` to help users discover available versions
- Complements `graft status --check-updates` for update awareness

## References

- Specification: `core-operations.md` lines 336-425
- Implementation: `src/graft/cli/commands/upgrade.py`
- Discussion: Session 4 dogfooding (workflow-validation.md)
