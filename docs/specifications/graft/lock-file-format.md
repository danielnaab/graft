---
title: "Lock File Format Specification"
date: 2026-01-31
status: draft
version: 3.0
supersedes: 2.0
---

# Lock File Format Specification

## Overview

The `graft.lock` file tracks the exact state of consumed **direct dependencies**. It records:
- Which dependencies are being used
- What version (git ref) has been consumed
- Resolved commit hash for integrity
- When each dependency was last updated

This file should be committed to version control to ensure reproducible dependency states across environments.

**Key Change from v2:** Lock file no longer tracks transitive dependencies. Only direct dependencies declared in `graft.yaml` are recorded. This aligns with the flat-only dependency model introduced in Decision 0007.

## File Location

```
project-root/
  .gitmodules         ← Git submodule tracking (required)
  graft.yaml          ← Consumer's configuration
  graft.lock          ← This file (generated and updated by Graft)
  .graft/             ← Dependencies (submodules)
  src/
  README.md
```

## Purpose

The lock file serves several purposes:

1. **State tracking**: Records direct dependencies and their consumed versions
2. **Reproducibility**: Enables identical dependency states across machines
3. **Integrity**: Stores commit hashes to detect tampering
4. **History**: Can be tracked in git to see dependency evolution
5. **Atomicity**: Updated only when upgrade fully succeeds

## Schema

### Top-Level Structure

```yaml
apiVersion: graft/v0

# Direct dependencies only
dependencies:
  <dep-name>:
    source: string           # Git URL or path
    ref: string              # Consumed git ref (tag, branch, commit)
    commit: string           # Resolved commit hash (SHA-1)
    consumed_at: datetime    # ISO 8601 timestamp
```

### API Version

**Field**: `apiVersion` (required)

**Type**: `string`

**Description**: Identifies the lock file format version.

**Current value**: `graft/v0`

**Example**:
```yaml
apiVersion: graft/v0
```

**Note**: Currently in initial development phase. Format may evolve. Future versions will use `graft/v1`, `graft/v2`, etc. when the specification stabilizes.

## Section: dependencies

Maps dependency names to their current state.

### Ordering Convention

**Specification**: Dependencies SHOULD be ordered alphabetically by name.

**Rationale**:
- **Consistent output** - All implementations generate lock files in the same order
- **Meaningful git diffs** - Changes to dependencies are clear in diffs
- **Easy to scan** - Users can quickly find a specific dependency

**Flexibility**: Parsers MUST accept dependencies in any order to allow:
- Hand-editing when necessary
- Backward compatibility
- Tool flexibility

**Example**:
```yaml
dependencies:
  coding-standards: {...}
  meta-kb: {...}
  templates-kb: {...}
```

**Implementation guideline**: "Be strict in what you generate, liberal in what you accept" (Robustness Principle)

### Fields

#### source (required)
**Type**: `string`

**Description**: Git URL or path to dependency repository. Must match the source in graft.yaml.

**Formats**:
- SSH: `git@github.com:user/repo.git`
- HTTPS: `https://github.com/user/repo.git`
- Local path: `../local-repo`

**Example**:
```yaml
source: "git@github.com:org/meta-kb.git"
```

#### ref (required)
**Type**: `string`

**Description**: The git ref that has been consumed. This is the version the consumer has integrated.

**Values**: Any valid git ref (commit hash, tag, branch)

**Semantics**: The consumer has applied all changes up to and including this ref.

**Example**:
```yaml
ref: "v1.5.0"         # Semver tag
ref: "abc123def456"   # Commit hash
ref: "release-2026-01"  # Date-based tag
```

#### commit (required)
**Type**: `string`

**Description**: The full commit hash that `ref` resolves to. Used for integrity verification.

**Format**: 40-character SHA-1 hash

**Purpose**:
- Detect if ref has been moved (e.g., branch advanced)
- Verify dependency hasn't been tampered with
- Enable exact reproduction

**Example**:
```yaml
commit: "abc123def456789012345678901234567890abcd"
```

#### consumed_at (required)
**Type**: `string` (ISO 8601 datetime)

**Description**: Timestamp when this version was last consumed/upgraded.

**Format**: `YYYY-MM-DDTHH:MM:SS[.mmmmmm][+HH:MM]`

**Example**:
```yaml
consumed_at: "2026-01-31T10:30:00Z"
consumed_at: "2026-01-31T10:30:00.123456+00:00"
```

## Complete Example

### Simple Project (single direct dependency)

```yaml
apiVersion: graft/v0

dependencies:
  meta-knowledge-base:
    source: "git@github.com:org/meta-kb.git"
    ref: "v2.0.0"
    commit: "abc123def456789012345678901234567890abcd"
    consumed_at: "2026-01-31T10:30:00Z"
```

### Project with Multiple Dependencies

```yaml
apiVersion: graft/v0

dependencies:
  coding-standards:
    source: "https://github.com/org/standards.git"
    ref: "v1.5.0"
    commit: "def456abc123789012345678901234567890abcd"
    consumed_at: "2026-01-31T09:15:00Z"

  meta-kb:
    source: "git@github.com:org/meta-kb.git"
    ref: "v2.0.0"
    commit: "abc123def456789012345678901234567890abcd"
    consumed_at: "2026-01-31T10:30:00Z"

  templates-kb:
    source: "https://github.com/org/templates.git"
    ref: "v1.0.0"
    commit: "789abc456def012345678901234567890abcdef12"
    consumed_at: "2026-01-30T14:20:00Z"
```

**Note:** All dependencies are direct (declared in `graft.yaml`). Alphabetically ordered for consistency.

## Lifecycle

### Creation

Generated when first dependency is added:

```bash
$ graft add meta-kb --source git@github.com:org/meta-kb.git --ref v1.0.0

Created graft.lock:
  meta-kb@v1.0.0
```

### Updates

Updated when dependency is upgraded:

```bash
$ graft upgrade meta-kb --to v2.0.0

Updated graft.lock:
  meta-kb: v1.0.0 → v2.0.0
```

**Important**: Lock file is only updated when upgrade fully succeeds (atomic operation).

### Manual Editing

Generally not recommended. Use `graft` commands instead.

If manual editing is necessary:
- Ensure YAML is valid
- Update all fields together (ref + commit + consumed_at)
- Run `graft validate` to check consistency

## Comparison to Other Lock Files

### Similar to package-lock.json (npm)

```json
{
  "dependencies": {
    "package-name": {
      "version": "1.0.0",
      "resolved": "https://...",
      "integrity": "sha512-..."
    }
  }
}
```

**Similarities**:
- Tracks exact versions
- Committed to version control
- Enables reproducibility

**Differences**:
- Graft uses git refs, not npm versions
- Graft tracks consumption state, not just installation
- Graft uses commit hash for integrity, not content hash

### Similar to Cargo.lock (Rust)

```toml
[[package]]
name = "package-name"
version = "1.0.0"
source = "registry+https://..."
checksum = "abc123..."
```

**Similarities**:
- Declarative format
- Checksum for integrity
- Version pinning

**Differences**:
- Graft uses git refs directly
- Graft designed for knowledge/code dependencies, not just code libraries

## Validation

### Lock File Validation

```python
def validate_lock_file(lock: dict) -> list[str]:
    """Validate lock file structure and content."""
    errors = []

    # Check apiVersion
    if 'apiVersion' not in lock:
        errors.append("Missing 'apiVersion' field")
    elif not lock['apiVersion'].startswith('graft/'):
        errors.append(f"Unsupported lock file apiVersion: {lock['apiVersion']}")

    # Check dependencies
    if 'dependencies' not in lock:
        errors.append("Missing 'dependencies' section")
        return errors

    for dep_name, dep_data in lock['dependencies'].items():
        # Required fields
        for field in ['source', 'ref', 'commit', 'consumed_at']:
            if field not in dep_data:
                errors.append(f"Dependency '{dep_name}': missing '{field}'")

        # Validate commit hash format
        if 'commit' in dep_data:
            commit = dep_data['commit']
            if not re.match(r'^[0-9a-f]{40}$', commit):
                errors.append(f"Dependency '{dep_name}': invalid commit hash '{commit}'")

        # Validate timestamp format
        if 'consumed_at' in dep_data:
            try:
                datetime.fromisoformat(dep_data['consumed_at'].replace('Z', '+00:00'))
            except ValueError:
                errors.append(f"Dependency '{dep_name}': invalid timestamp format")

    return errors
```

### Dependency Graph Validation

**Note:** In v3, there is no dependency graph to validate since only direct dependencies are tracked. This section is retained for v2 compatibility.

If migrating from v2 to v3, the `direct`, `requires`, and `required_by` fields can be safely ignored.

### Integrity Verification

```python
def verify_integrity(lock: dict, dep_name: str, repo_path: str) -> bool:
    """Verify that ref resolves to expected commit."""
    dep_data = lock['dependencies'][dep_name]
    expected_commit = dep_data['commit']

    # Resolve ref to commit
    result = subprocess.run(
        ['git', 'rev-parse', dep_data['ref']],
        cwd=repo_path,
        capture_output=True,
        text=True
    )

    actual_commit = result.stdout.strip()

    if actual_commit != expected_commit:
        print(f"Warning: {dep_name} ref '{dep_data['ref']}' has moved!")
        print(f"  Expected: {expected_commit}")
        print(f"  Actual:   {actual_commit}")
        return False

    return True
```

## Submodule Synchronization

### Synchronization Guarantee

**The lock file `commit` field MUST match the submodule's checked-out commit.**

Dependencies are tracked as git submodules in `.gitmodules`. The lock file and submodule state must remain synchronized:

| State | Lock file `commit` | Submodule HEAD | Valid? |
|-------|-------------------|----------------|--------|
| Synced | `abc123...` | `abc123...` | ✓ Yes |
| Mismatch | `abc123...` | `def456...` | ✗ No |
| Missing submodule | `abc123...` | (not initialized) | ✗ No |

### Validation

`graft validate integrity` checks both lock file AND submodule state:

1. For each dependency in lock file:
   - Verify `.graft/<name>/` exists and is a git submodule
   - Run `git rev-parse HEAD` in the submodule
   - Compare to `commit` field in lock file
   - Report any mismatches

2. Check for orphaned submodules:
   - Submodules in `.graft/` not tracked in lock file

### When Synchronization Occurs

| Operation | Updates lock file | Updates submodule |
|-----------|------------------|-------------------|
| `graft add` | ✓ Yes | ✓ Yes (creates) |
| `graft remove` | ✓ Yes (removes) | ✓ Yes (removes) |
| `graft upgrade` | ✓ Yes | ✓ Yes |
| `graft resolve` | Creates if missing | ✓ Yes |
| `graft sync` | No | ✓ Yes (matches lock) |

### Sync After Pull

When pulling changes from teammates who upgraded dependencies:

```bash
# Teammate upgraded a dependency
git pull
# graft.lock and .gitmodules changed

# Sync submodule checkouts to match lock file
graft sync
```

The `graft sync` command updates submodule checkouts to match the lock file state without running migrations (migrations were already run by the upgrader).

## Operations

### Read Lock File

```python
def read_lock_file(path: str = 'graft.lock') -> dict:
    """Read and parse lock file."""
    with open(path) as f:
        return yaml.safe_load(f)
```

### Update Dependency

```python
def update_lock_file(
    dep_name: str,
    ref: str,
    commit: str,
    source: str,
    lock_path: str = 'graft.lock'
):
    """Update lock file after successful upgrade."""
    lock = read_lock_file(lock_path)

    if 'dependencies' not in lock:
        lock['dependencies'] = {}

    lock['dependencies'][dep_name] = {
        'source': source,
        'ref': ref,
        'commit': commit,
        'consumed_at': datetime.now(timezone.utc).isoformat()
    }

    with open(lock_path, 'w') as f:
        yaml.dump(lock, f, default_flow_style=False, sort_keys=False)
```

### Get Current Version

```python
def get_consumed_ref(dep_name: str, lock_path: str = 'graft.lock') -> Optional[str]:
    """Get currently consumed ref for a dependency."""
    lock = read_lock_file(lock_path)
    dep_data = lock.get('dependencies', {}).get(dep_name)
    return dep_data['ref'] if dep_data else None
```

## CLI Integration

```bash
# Show lock file status
$ graft status
Dependencies:
  meta-kb: v1.5.0 (consumed 2026-01-01)
  shared-utils: v2.0.0 (consumed 2025-12-15)

# Check for updates
$ graft status --check-updates
Dependencies:
  meta-kb: v1.5.0 → v2.0.0 available
  shared-utils: v2.0.0 (up to date)

# Validate lock file
$ graft validate --lock
Lock file is valid
All commits verified
No integrity issues
```

## Git Integration

The lock file and submodule references should be committed to git:

```bash
# After upgrade
$ graft upgrade meta-kb --to v2.0.0
Upgraded meta-kb to v2.0.0

$ git status
modified:   .gitmodules
modified:   .graft/meta-kb (new commits)
modified:   graft.lock

$ git add .gitmodules .graft/meta-kb graft.lock
$ git commit -m "Upgrade meta-kb to v2.0.0"
```

This creates a history of dependency evolution:

```bash
$ git log --oneline -- graft.lock
abc123 Upgrade meta-kb to v2.0.0
def456 Upgrade shared-utils to v2.0.0
789abc Initial graft.lock
```

## Edge Cases

### Ref Has Moved (Branch Updated)

```yaml
# Lock file says:
dependencies:
  meta-kb:
    ref: "main"
    commit: "abc123"

# But main has advanced to def456
```

**Detection**:
```bash
$ graft validate --lock
Warning: meta-kb ref 'main' has moved
  Lock file: abc123
  Current:   def456
  Run 'graft upgrade meta-kb' to update
```

**Resolution**: Run upgrade to update to new commit.

### Deleted Ref

```yaml
# Lock file references:
dependencies:
  meta-kb:
    ref: "feature-branch"
    commit: "abc123"

# But feature-branch was deleted
```

**Detection**:
```bash
$ graft validate --lock
Error: meta-kb ref 'feature-branch' does not exist
  Commit abc123 is still accessible
  Consider updating to a stable ref (tag or main)
```

**Resolution**: Update to a different ref that points to the same commit or newer.

### Source URL Changed

```yaml
# Lock file:
source: "git@github.com:old-org/repo.git"

# graft.yaml now says:
source: "git@github.com:new-org/repo.git"
```

**Detection**:
```bash
$ graft validate
Warning: meta-kb source URL differs between graft.yaml and graft.lock
  Lock: git@github.com:old-org/repo.git
  Config: git@github.com:new-org/repo.git
```

**Resolution**: Update lock file source to match graft.yaml.

## Migration from v2 to v3

### Changes

v3 removes transitive dependency tracking:

**Removed fields:**
- `direct` - All dependencies are now direct
- `requires` - No longer tracked
- `required_by` - No longer tracked

**Preserved fields:**
- `source` - Git URL or path (unchanged)
- `ref` - Consumed git ref (unchanged)
- `commit` - Resolved commit hash (unchanged)
- `consumed_at` - Timestamp (unchanged)

### Migration Steps

**Delete and regenerate:**
```bash
# Delete old lock and dependencies
rm graft.lock
rm -rf .graft/

# Regenerate with v3
graft resolve
```

This is the recommended approach. The lock file will be regenerated with the v3 schema automatically.

**Note:** No migration tool is provided. The delete-and-regenerate approach is simple and sufficient for current needs.

**Example transformation:**

```yaml
# v2 (before)
apiVersion: graft/v0
dependencies:
  meta-kb:
    source: "..."
    ref: "v2.0.0"
    commit: "abc123..."
    consumed_at: "2026-01-31T10:30:00Z"
    direct: true
    requires: ["standards-kb"]
    required_by: []

  standards-kb:
    source: "..."
    ref: "v1.5.0"
    commit: "def456..."
    consumed_at: "2026-01-31T10:30:00Z"
    direct: false
    requires: []
    required_by: ["meta-kb"]
```

```yaml
# v3 (after)
apiVersion: graft/v0
dependencies:
  meta-kb:
    source: "..."
    ref: "v2.0.0"
    commit: "abc123..."
    consumed_at: "2026-01-31T10:30:00Z"
```

**Note:** Transitive dependency `standards-kb` is removed. If needed, add it as a direct dependency in `graft.yaml`.

## Lock File History (Optional Future Enhancement)

Could optionally include history:

```yaml
apiVersion: graft/v0

dependencies:
  meta-kb:
    source: "git@github.com:org/meta-kb.git"
    ref: "v2.0.0"
    commit: "def456..."
    consumed_at: "2026-01-31T10:30:00Z"
    history:
      - ref: "v1.0.0"
        commit: "abc123..."
        consumed_at: "2025-10-01T08:00:00Z"
      - ref: "v1.5.0"
        commit: "bcd234..."
        consumed_at: "2025-12-15T09:00:00Z"
```

**Not currently specified** - use git log on graft.lock instead.

## Related

- [Specification: graft.yaml Format](./graft-yaml-format.md)
- [Specification: Dependency Layout](./dependency-layout.md)
- [Specification: Core Operations](./core-operations.md)
- [Decision 0004: Atomic Upgrades](../decisions/decision-0004-atomic-upgrades.md)
- [Decision 0007: Flat-Only Dependency Model](../decisions/decision-0007-flat-only-dependencies.md)

## References

- YAML Specification: https://yaml.org/spec/
- ISO 8601 (datetime format): https://en.wikipedia.org/wiki/ISO_8601
- Git commit hashing: https://git-scm.com/book/en/v2/Git-Internals-Git-Objects

## Changelog

- **2026-01-31 (v3.0)**: Flat-only dependency model
  - Removed transitive dependency tracking
  - Removed fields: `direct`, `requires`, `required_by`
  - Simplified ordering to alphabetical only
  - Added submodule synchronization requirements
  - Added migration guide from v2 to v3
  - Updated all examples
  - Supersedes v2

- **2026-01-05 (v2.0)**: Extended lock file
  - Added transitive dependency tracking
  - Added fields: `direct`, `requires`, `required_by`
  - Introduced ordering convention (direct first, then transitive)

- **2026-01-01 (v1.0)**: Initial draft
  - Basic lock file structure
  - Core fields: source, ref, commit, consumed_at
