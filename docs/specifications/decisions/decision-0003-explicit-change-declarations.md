---
title: "Explicit Change Declarations in graft.yaml"
date: 2026-01-01
status: accepted
---

# Explicit Change Declarations in graft.yaml

## Context

Graft needs to map changes (identified by git refs) to migration operations. This mapping must be:
- **Deterministic**: Same input always produces same output
- **Reliable**: Not subject to parsing ambiguities
- **Validatable**: Can check correctness before runtime
- **Discoverable**: Easy to find what operations are available

Two primary approaches:
1. **Parse from markdown** - Extract migration commands from CHANGELOG.md using conventions
2. **Explicit YAML declarations** - Define change→command mapping in graft.yaml

This decision is critical for:
- Reliability of automated upgrades
- Ease of validation and testing
- User confidence in deterministic behavior
- Implementation complexity

## Decision

**Changes and their associated operations (migrations, verifications) will be declared explicitly in graft.yaml, not parsed from markdown.**

The graft.yaml file defines:
- Which git refs have changes
- What type of change each is (optional)
- What migration command to run (optional)
- What verification command to run (optional)

CHANGELOG.md remains valuable for human-readable context and rationale, but graft.yaml is the source of truth for automation.

## Alternatives Considered

### Alternative 1: Parse Migration Commands from Markdown

**Approach**: Define conventions for CHANGELOG.md format, parse migration commands from it

Example:
```markdown
## [2.0.0]

### BREAKING: Renamed getUserData → fetchUserData

**Migration**: `graft meta-kb:migrate-v2`
```

Parser looks for `graft <dep>:<command>` pattern in backticks.

**Pros**:
- Single source of documentation
- Changelog contains everything
- Fewer files to maintain
- Changelog is "executable"

**Cons**:
- **Brittle parsing**: Relies on conventions, format variations break it
- **Not deterministic**: Parser logic could change
- **Hard to validate**: Can't easily check correctness without parsing
- **Ambiguous**: Is every backtick-wrapped command a reference?
- **Error-prone**: Typos in markdown break automation
- **Versioning**: Parser version affects behavior

**Why rejected**: Parsing heuristics are fundamentally unreliable for deterministic automation.

### Alternative 2: Frontmatter in Markdown

**Approach**: Add YAML frontmatter to CHANGELOG.md sections

Example:
```markdown
## [2.0.0]
---
type: breaking
migration: migrate-v2
verify: verify-v2
---

### Renamed getUserData → fetchUserData

Details...
```

**Pros**:
- Structured metadata
- Still in changelog
- Easier to parse than prose

**Cons**:
- Violates Keep a Changelog conventions
- Awkward to read
- Mixes machine and human formats
- Still parsing markdown (section boundaries)
- Tools that display changelogs may not handle frontmatter

**Why rejected**: Awkward hybrid, still requires markdown parsing.

### Alternative 3: Separate changes.yaml

**Approach**: Create dedicated changes.yaml or .graft/changes.yaml file

Example:
```yaml
# changes.yaml
changes:
  v2.0.0:
    type: breaking
    migration: migrate-v2
    verify: verify-v2
  v1.5.0:
    type: feature
```

**Pros**:
- Explicit structured format
- Separate from changelog
- Easy to parse and validate

**Cons**:
- **Another file** to maintain
- Unclear where to look (graft.yaml vs changes.yaml)
- Duplication of version information
- Not obviously discoverable

**Why rejected**: Unnecessary extra file when graft.yaml already exists.

### Alternative 4: Validate Parsed References

**Approach**: Parse markdown, but validate that referenced commands exist

Example:
- Parse `graft meta-kb:migrate-v2` from changelog
- Validate that `migrate-v2` exists in graft.yaml
- Error if not found

**Pros**:
- Catches some errors early
- Still use markdown as source

**Cons**:
- **Still brittle**: Parsing is still fragile
- Validation helps but doesn't solve root problem
- False sense of security
- Parser bugs still cause issues

**Why rejected**: Doesn't address fundamental brittleness of parsing.

## Consequences

### Positive

✅ **Deterministic**: Explicit declarations, no parsing ambiguity
✅ **Validatable**: Can check command references before execution
✅ **Discoverable**: All automation in one well-known file
✅ **Reliable**: Format changes don't break automation
✅ **Versioned**: Changes to mapping are tracked in git
✅ **Testable**: Can validate without parsing heuristics

### Negative

❌ **Duplication**: Version refs appear in both graft.yaml and CHANGELOG.md
❌ **Maintenance**: Two files to keep in sync
❌ **Verbose**: graft.yaml grows with each version

### Mitigations

- **Tooling**: Provide `graft validate` to check consistency
- **Generation**: Can generate graft.yaml stubs from changelog
- **Documentation**: Clear conventions about what goes where
- **Minimal required**: Only refs with automation need entries

### Division of Responsibility

**graft.yaml**: Automation source of truth
- Which refs have changes
- What commands to run
- Execution details

**CHANGELOG.md**: Human-readable context
- What changed and why
- Rationale and decision-making
- Detailed migration steps
- Impact analysis

Both are valuable, different purposes.

## Implementation Notes

### graft.yaml Format

```yaml
# Optional: Point to human-readable changelog
metadata:
  changelog: "CHANGELOG.md"

# Explicit change declarations
changes:
  v2.0.0:
    type: breaking
    migration: migrate-v2
    verify: verify-v2

  v1.5.0:
    type: feature
    # No migration needed - no command specified

  abc123:
    type: fix
    migration: migrate-abc

# Commands referenced by changes
commands:
  migrate-v2:
    run: "npx jscodeshift -t codemods/v2.js src/"
    description: "Rename getUserData to fetchUserData"

  verify-v2:
    run: "npm test && ! grep -r 'getUserData' src/"
    description: "Verify v2 migration completed"

  migrate-abc:
    run: "./scripts/fix-abc.sh"
```

### Validation

```bash
# Check consistency
graft validate

# Checks performed:
# 1. All refs in changes exist in git
# 2. All migration commands exist in commands section
# 3. All verify commands exist in commands section
# 4. No circular dependencies
# 5. Refs are in valid order (if ordering matters)
```

### Workflow

```python
def get_migration_command(dep: str, ref: str) -> Optional[str]:
    """Get migration command for a change - no parsing needed."""
    config = load_graft_yaml(dep)
    change = config.get('changes', {}).get(ref)
    if not change:
        return None
    return change.get('migration')

def upgrade(dep: str, to_ref: str):
    """Upgrade - deterministic, no parsing."""
    migration_cmd = get_migration_command(dep, to_ref)
    if migration_cmd:
        execute_command(dep, migration_cmd)

    verify_cmd = get_verification_command(dep, to_ref)
    if verify_cmd:
        execute_command(dep, verify_cmd)

    update_lock_file(dep, to_ref)
```

### Discovery

```bash
# Show available migrations
$ graft inspect meta-kb

Changes with automation:
  v2.0.0 (breaking)
    Migration: migrate-v2
    Verify: verify-v2

  abc123 (fix)
    Migration: migrate-abc

Changes without automation:
  v1.5.0 (feature)
```

## Examples

### Minimal (Feature with No Migration)

```yaml
changes:
  v1.5.0:
    type: feature
```

No migration or verify commands needed.

### Full Automation

```yaml
changes:
  v2.0.0:
    type: breaking
    migration: migrate-v2
    verify: verify-v2

commands:
  migrate-v2:
    run: "npx jscodeshift -t codemods/v2.js"
  verify-v2:
    run: "npm test"
```

### Manual Migration

```yaml
changes:
  v3.0.0:
    type: breaking
    # No migration command = manual steps only
```

User runs `graft show meta-kb@v3.0.0`, reads CHANGELOG.md for manual steps.

### Chained Migrations

```yaml
changes:
  v3.0.0:
    migration: migrate-v3

commands:
  migrate-v3:
    run: |
      graft meta-kb:migrate-v2
      ./additional-v3-steps.sh
```

## Validation Strategy

### Pre-publish Validation

Dependencies can validate their graft.yaml:

```yaml
# In dependency's CI
- name: Validate graft.yaml
  run: graft validate --local
```

Checks:
- YAML is well-formed
- Referenced commands exist
- Refs exist in git
- No dangling references

### Consumer Validation

Consumers can validate before upgrading:

```bash
$ graft validate meta-kb
✓ All changes reference valid commands
✓ All commands are defined
✓ All refs exist in repository
```

## Related

- [Decision 0002: Git Refs Over Semver](./decision-0002-git-refs-over-semver.md)
- [Decision 0004: Atomic Upgrades](./decision-0004-atomic-upgrades.md)
- [Specification: graft.yaml Format](../graft/graft-yaml-format.md)
- [Specification: Change Model](../graft/change-model.md)

## References

- Keep a Changelog: https://keepachangelog.com/
- Common Changelog: https://common-changelog.org/
- YAML Spec: https://yaml.org/spec/
