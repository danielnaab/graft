---
title: "Change Model Specification"
date: 2026-01-01
status: draft
---

# Change Model Specification

## Overview

A **Change** represents a semantic change in a dependency, identified by a git ref and optionally associated with migration and verification operations.

Changes are the atomic unit that consumers track and apply when upgrading dependencies.

## Data Model

### TypeScript Definition

```typescript
interface Change {
  // Required
  ref: string              // Git ref (commit, tag, branch)

  // Optional metadata
  type?: ChangeType        // Semantic type of change
  description?: string     // Brief human-readable summary
  migration?: string       // Command name for migration
  verify?: string          // Command name for verification

  // Extensible
  [key: string]: any       // Additional custom metadata
}

type ChangeType =
  | "breaking"    // Breaking change (requires migration)
  | "feature"     // New feature (backward compatible)
  | "fix"         // Bug fix
  | "deprecation" // Deprecation notice
  | "security"    // Security fix
  | "performance" // Performance improvement
  | "docs"        // Documentation only
  | "internal"    // Internal change (no consumer impact)
  | string        // Custom types allowed
```

### Python Definition

```python
from dataclasses import dataclass
from typing import Optional, Any

@dataclass
class Change:
    """Represents a semantic change in a dependency."""

    # Required
    ref: str  # Git ref (commit, tag, branch)

    # Optional
    type: Optional[str] = None  # Semantic type
    description: Optional[str] = None  # Brief summary
    migration: Optional[str] = None  # Migration command name
    verify: Optional[str] = None  # Verification command name

    # Extensible metadata
    metadata: dict[str, Any] = None

    def __post_init__(self):
        if self.metadata is None:
            self.metadata = {}
```

## Field Specifications

### ref (required)

**Type**: `string`

**Description**: Git reference identifying this change. Can be any valid git ref.

**Valid values**:
- Commit hash: `abc123def456` or `abc123` (short hash)
- Tag: `v2.0.0`, `release-2026-01`, `r42`
- Branch: `main`, `stable`, `feature-auth`

**Constraints**:
- Must be a valid git ref
- Must exist in the dependency's repository
- Should be immutable (commits and tags preferred over branches)

**Examples**:
```yaml
ref: "v2.0.0"           # Semver tag
ref: "abc123def456"     # Full commit hash
ref: "abc123"           # Short commit hash
ref: "release-2026-01"  # Date-based tag
ref: "main"             # Branch (not recommended for stable releases)
```

### type (optional)

**Type**: `string`

**Description**: Semantic type of the change. Helps consumers understand the impact.

**Standard values**:
- `breaking`: Breaking change, requires consumer action
- `feature`: New feature, backward compatible
- `fix`: Bug fix
- `deprecation`: Deprecation notice
- `security`: Security fix
- `performance`: Performance improvement
- `docs`: Documentation only
- `internal`: Internal change, no consumer impact

**Custom values**: Projects may define their own types.

**Usage**:
- If type is `breaking`, migration is typically required
- Consumers can filter changes by type: `graft changes --breaking`
- Used for impact analysis

**Examples**:
```yaml
type: breaking
type: feature
type: security
type: custom-type  # Custom types allowed
```

### description (optional)

**Type**: `string`

**Description**: Brief human-readable summary of the change.

**Format**: Single line, 50-100 characters recommended.

**Purpose**:
- Quick overview in list views
- Used in `graft status` and `graft changes` output
- Complements detailed CHANGELOG.md content

**Examples**:
```yaml
description: "Renamed getUserData → fetchUserData"
description: "Added response caching"
description: "Fixed race condition in auth flow"
```

### migration (optional)

**Type**: `string`

**Description**: Name of the command to run for migration. References a command defined in the `commands` section of graft.yaml.

**Format**: Command name (no prefix or special characters)

**Execution**: When `graft upgrade` runs, if `migration` is specified, the referenced command is executed.

**Validation**:
- Must reference a command that exists in `commands` section
- Command must be defined in the same graft.yaml

**Examples**:
```yaml
migration: migrate-v2
migration: migrate-auth
migration: fix-abc
```

**Omission**: If no migration is specified, no automated migration runs. The upgrade may still require manual steps documented in CHANGELOG.md.

### verify (optional)

**Type**: `string`

**Description**: Name of the command to run for verification. References a command defined in the `commands` section of graft.yaml.

**Format**: Command name (no prefix or special characters)

**Execution**: When `graft upgrade` runs, after migration completes, if `verify` is specified, the referenced command is executed.

**Purpose**:
- Validate that migration succeeded
- Run tests
- Check for deprecated patterns
- Ensure correctness

**Validation**:
- Must reference a command that exists in `commands` section
- Command must be defined in the same graft.yaml

**Examples**:
```yaml
verify: verify-v2
verify: run-tests
verify: check-no-deprecated
```

**Omission**: If no verification is specified, upgrade succeeds after migration (or immediately if no migration).

### metadata (extensible)

**Type**: `object` (key-value pairs)

**Description**: Additional custom metadata. Projects can add any fields they need.

**Purpose**: Extensibility for project-specific needs without modifying the core schema.

**Examples**:
```yaml
# In TypeScript/Python, stored as metadata dict
metadata:
  author: "jane@example.com"
  jira_ticket: "PROJ-123"
  review_url: "https://github.com/org/repo/pull/42"
  breaking_apis: ["getUserData", "setUserData"]
```

## Source: graft.yaml

Changes are defined in the dependency's `graft.yaml` file:

```yaml
# Dependency's graft.yaml

changes:
  v2.0.0:
    type: breaking
    description: "Renamed getUserData → fetchUserData"
    migration: migrate-v2
    verify: verify-v2

  v1.5.0:
    type: feature
    description: "Added caching support"

  abc123:
    type: fix
    description: "Fixed race condition"
    migration: migrate-abc

commands:
  migrate-v2:
    run: "npx jscodeshift -t codemods/v2.js"

  verify-v2:
    run: "npm test"

  migrate-abc:
    run: "./scripts/fix-abc.sh"
```

## Parsing

```python
def load_changes(graft_yaml_path: str) -> list[Change]:
    """Load changes from graft.yaml."""
    with open(graft_yaml_path) as f:
        config = yaml.safe_load(f)

    changes = []
    for ref, data in config.get('changes', {}).items():
        change = Change(
            ref=ref,
            type=data.get('type'),
            description=data.get('description'),
            migration=data.get('migration'),
            verify=data.get('verify'),
            metadata={k: v for k, v in data.items()
                     if k not in ('type', 'description', 'migration', 'verify')}
        )
        changes.append(change)

    return changes
```

## Validation

### Required Validation

```python
def validate_change(change: Change, git_refs: set[str], commands: set[str]) -> list[str]:
    """Validate a change. Returns list of errors (empty if valid)."""
    errors = []

    # 1. Ref must exist in git
    if change.ref not in git_refs:
        errors.append(f"Ref '{change.ref}' does not exist in git repository")

    # 2. Migration command must exist (if specified)
    if change.migration and change.migration not in commands:
        errors.append(f"Migration command '{change.migration}' not defined in commands section")

    # 3. Verify command must exist (if specified)
    if change.verify and change.verify not in commands:
        errors.append(f"Verify command '{change.verify}' not defined in commands section")

    return errors
```

### Optional Validation

Projects may add additional validation:
- Type must be from standard list
- Description must be under N characters
- Breaking changes must have migration
- Security changes must be documented in detail

## Ordering

Changes are ordered by their declaration order in graft.yaml:

```yaml
changes:
  v1.0.0:    # Applied first
    migration: migrate-v1

  v2.0.0:    # Applied second
    migration: migrate-v2

  v3.0.0:    # Applied third
    migration: migrate-v3
```

When upgrading from v1.0.0 to v3.0.0, migrations run in this order.

**Fallback**: If order isn't clear from the file, use git log chronological order.

## Examples

### Minimal Change (No Automation)

```yaml
changes:
  v1.5.0:
    type: feature
```

Just tracks that v1.5.0 exists and is a feature. No migration or verification.

### Full Automation

```yaml
changes:
  v2.0.0:
    type: breaking
    description: "Renamed getUserData → fetchUserData"
    migration: migrate-v2
    verify: verify-v2

commands:
  migrate-v2:
    run: "npx jscodeshift -t codemods/v2.js"
  verify-v2:
    run: "npm test"
```

Fully automated: runs migration and verification during upgrade.

### Custom Metadata

```yaml
changes:
  v3.0.0:
    type: breaking
    description: "Major refactor"
    migration: migrate-v3
    jira_ticket: "PROJ-456"
    author: "jane@example.com"
    estimated_duration: "30 minutes"
```

Additional fields stored in metadata.

### Commit-Based

```yaml
changes:
  abc123def:
    type: fix
    description: "Fixed auth race condition"
    migration: fix-auth
```

Uses commit hash instead of tag.

## Querying Changes

### Get all changes for a dependency

```python
changes = load_changes('path/to/dep/graft.yaml')
```

### Filter by type

```python
breaking_changes = [c for c in changes if c.type == 'breaking']
```

### Find changes between refs

```python
def changes_between(changes: list[Change], from_ref: str, to_ref: str) -> list[Change]:
    """Get changes between two refs (based on declaration order)."""
    start = next(i for i, c in enumerate(changes) if c.ref == from_ref)
    end = next(i for i, c in enumerate(changes) if c.ref == to_ref)
    return changes[start+1:end+1]
```

### Check if migration is needed

```python
def needs_migration(change: Change) -> bool:
    return change.migration is not None
```

## Relation to CHANGELOG.md

Changes in graft.yaml provide **automation metadata**.

CHANGELOG.md provides **human-readable context**:
- Detailed rationale
- Impact analysis
- Manual migration steps
- Examples
- Breaking change explanations

Both are valuable for different purposes:
- **Machines** read graft.yaml
- **Humans** read CHANGELOG.md

## Related

- [Specification: graft.yaml Format](./graft-yaml-format.md)
- [Specification: Core Operations](./core-operations.md)
- [Decision 0002: Git Refs Over Semver](../decisions/decision-0002-git-refs-over-semver.md)
- [Decision 0003: Explicit Change Declarations](../decisions/decision-0003-explicit-change-declarations.md)
