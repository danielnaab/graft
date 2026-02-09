---
title: "Graft Architecture"
status: draft
date: 2026-01-01
---

# Graft Architecture

## Overview

Graft is a **semantic change tracking and dependency management system** built on git primitives. It provides:

1. **Change tracking**: Track semantic changes (features, breaking changes, fixes) independently from code commits
2. **Automated migrations**: Run codemods and migration scripts during upgrades
3. **Command execution**: Execute tasks defined by dependencies
4. **Lock file state**: Track consumed versions for reproducibility

**Core insight**: If git tracks code changes, Graft tracks **semantic changes** - the evolution of APIs, features, and contracts that consumers need to integrate.

## Core Concepts

### Changes

A **Change** is the atomic unit of semantic evolution, identified by a git ref and optionally associated with migration operations.

```yaml
# In dependency's graft.yaml
changes:
  v2.0.0:                    # Git ref (tag, branch, or commit)
    type: breaking           # Semantic type
    migration: migrate-v2    # Command to run
    verify: verify-v2        # Verification command
```

Key properties:
- **Git-native**: Changes identified by any git ref (commit, tag, branch)
- **No semver requirement**: Works with any versioning strategy
- **Explicit declarations**: Changes defined in graft.yaml, not parsed
- **Optional automation**: Migration and verification are optional

See: [Change Model Specification](graft/change-model.md)

### Commands

Dependencies define **commands** that consumers can execute:

```yaml
# In dependency's graft.yaml
commands:
  migrate-v2:
    run: "npx jscodeshift -t codemods/v2.js src/"
    description: "Rename getUserData → fetchUserData"
```

Commands can be:
- Migration scripts (codemods, shell scripts, Python, etc.)
- Verification scripts (tests, linters, checks)
- Utility commands (changelog display, etc.)

Consumers execute: `graft <dep>:<command>`

### Lock File

The **lock file** (`graft.lock`) tracks consumed state:

```yaml
dependencies:
  meta-kb:
    source: "git@github.com:org/meta-kb.git"
    ref: "v1.5.0"              # Consumed version
    commit: "abc123..."         # Resolved commit hash
    consumed_at: "2026-01-01T10:30:00Z"
```

Key properties:
- **Single version**: No split between installed vs. consumed
- **Atomic updates**: Updated only when upgrade fully succeeds
- **Integrity**: Stores commit hash for verification
- **Reproducibility**: Committed to git for consistency

See: [Lock File Format Specification](graft/lock-file-format.md)

## Data Model

### Primitives

**1. Change**
```typescript
interface Change {
  ref: string           // Git ref - required
  type?: string         // Optional: "breaking", "feature", "fix"
  migration?: string    // Optional: command name
  verify?: string       // Optional: verification command
  [key: string]: any    // Extensible metadata
}
```

**2. Dependency State**
```typescript
interface Dependency {
  source: string        // Git URL or path
  ref: string          // Consumed git ref
  commit: string       // Resolved commit hash
  consumed_at: string  // ISO 8601 timestamp
}
```

### Configuration Files

**graft.yaml** (in dependency repository):
- Change definitions
- Command definitions
- Metadata

**graft.lock** (in consumer repository):
- Dependency states
- Consumed versions
- Commit hashes

See: [graft.yaml Format Specification](graft/graft-yaml-format.md)

## Core Operations

### Query Operations (Read-only)

- `graft status`: Show current dependency states
- `graft fetch`: Update cache of upstream changes
- `graft changes <dep>`: List available changes
- `graft show <dep>@<ref>`: Show change details

### Mutation Operations

- `graft upgrade <dep>`: Atomic upgrade with migration and verification
- `graft apply <dep>`: Update lock file without migration
- `graft validate`: Validate configuration and lock file

### Command Execution

- `graft <dep>:<command>`: Execute dependency command

See: [Core Operations Specification](graft/core-operations.md)

## Atomic Upgrade Flow

```
graft upgrade meta-kb --to v2.0.0
    ↓
Create snapshot (for rollback)
    ↓
Update files to v2.0.0
    ↓
Run migration command (if defined)
    ↓
Run verification command (if defined)
    ↓
Update graft.lock
    ↓
Success ✓ (or rollback on failure ✗)
```

Upgrades are **all-or-nothing**. No intermediate states like "applied but not verified".

See: [Decision 0004: Atomic Upgrades](decisions/decision-0004-atomic-upgrades.md)

## Design Principles

### 1. Git-Native
- Use git refs (commits, tags, branches) as identity
- No opinions on versioning strategy (semver optional)
- Leverage git for integrity and history

See: [Decision 0002: Git Refs Over Semver](decisions/decision-0002-git-refs-over-semver.md)

### 2. Explicit Over Implicit
- Changes declared in structured YAML, not parsed from markdown
- Migration commands explicitly referenced
- Deterministic, validatable, not brittle

See: [Decision 0003: Explicit Change Declarations](decisions/decision-0003-explicit-change-declarations.md)

### 3. Minimal Primitives
- Only two core primitives: Change and Dependency
- Everything else is metadata
- Extensible through custom fields

### 4. Separation of Concerns
- **graft.yaml**: Automation (for machines)
- **CHANGELOG.md**: Context and rationale (for humans)
- Both valuable, different purposes

### 5. Atomic Operations
- Upgrades succeed or fail as a unit
- No partial states
- Simple mental model

### 6. Composability
- Commands can call other commands
- Migrations can chain
- Flexible workflows

## Architecture Diagrams

### System Context

```
┌─────────────┐
│  Upstream   │  Publishes changes via graft.yaml
│ (Dependency)│  Defines migration commands
└──────┬──────┘
       │
       │ git fetch
       ↓
┌─────────────┐
│    Graft    │  Queries changes
│   (Tool)    │  Executes migrations
└──────┬──────┘  Updates lock file
       │
       ↓
┌─────────────┐
│  Consumer   │  Tracks consumed versions
│  (Project)  │  Runs migrations
└─────────────┘  Integrates changes
```

### File Structure

```
Dependency Repository:
  graft.yaml       ← Changes, commands, metadata
  CHANGELOG.md     ← Human-readable context
  codemods/        ← Migration implementations
  scripts/
  src/

Consumer Repository:
  graft.yaml       ← Consumer's dependencies
  graft.lock       ← Consumed versions (generated)
  src/
```

### Change Flow

```
1. Dependency publishes v2.0.0
   ├─ Updates graft.yaml (adds change entry)
   ├─ Updates CHANGELOG.md (detailed rationale)
   └─ Commits codemod (codemods/v2.js)

2. Consumer discovers change
   $ graft fetch meta-kb
   $ graft changes meta-kb
   [Shows v2.0.0 available]

3. Consumer upgrades
   $ graft upgrade meta-kb --to v2.0.0
   ├─ Runs migration: migrate-v2
   ├─ Runs verification: verify-v2
   └─ Updates graft.lock: ref → v2.0.0

4. State is tracked in git
   $ git diff graft.lock
   -  ref: "v1.5.0"
   +  ref: "v2.0.0"
```

## Key Decisions

The architecture is based on several key decisions:

### Core Architecture (v1.0)

1. **[Decision 0001: Initial Scope](decisions/decision-0001-initial-scope.md)** - Define Graft as task runner + dependency manager
2. **[Decision 0002: Git Refs Over Semver](decisions/decision-0002-git-refs-over-semver.md)** - Use git refs, don't require semver
3. **[Decision 0003: Explicit Change Declarations](decisions/decision-0003-explicit-change-declarations.md)** - Changes in YAML, not parsed
4. **[Decision 0004: Atomic Upgrades](decisions/decision-0004-atomic-upgrades.md)** - All-or-nothing upgrades, no partial states

### Specification Enhancements (2026-01-05)

5. **[Decision 0005: No Partial Resolution](decisions/decision-0005-no-partial-resolution.md)** - Maintain atomicity and reproducibility *(superseded by Decision 0007)*

Additional enhancements:
- Lock file ordering conventions (see [Lock File Format spec](graft/lock-file-format.md#ordering-convention))
- Validation operations specification (see [Core Operations spec](graft/core-operations.md#validation-operations))

### Dependency Model (2026-01-31)

6. **[Decision 0007: Flat-Only Dependency Model](decisions/decision-0007-flat-only-dependencies.md)** - Direct dependencies only, no transitive resolution

This decision simplifies the dependency model by treating dependencies as "influences" rather than "components". Consumers explicitly declare all dependencies they use, enabling simpler tooling and clearer ownership.

See [all decisions](decisions/) for complete list and rationale.

## Implementation Specifications

Detailed specifications for implementation:

- **[Change Model](graft/change-model.md)** - Data model for changes
- **[graft.yaml Format](graft/graft-yaml-format.md)** - Configuration file format
- **[Lock File Format](graft/lock-file-format.md)** - State tracking format
- **[Core Operations](graft/core-operations.md)** - Operation semantics and behavior

## Example Workflow

### Dependency Publishes Change

```yaml
# meta-kb/graft.yaml
changes:
  v2.0.0:
    type: breaking
    description: "Renamed getUserData → fetchUserData"
    migration: migrate-v2
    verify: verify-v2

commands:
  migrate-v2:
    run: "npx jscodeshift -t codemods/rename-getUserData.js"
  verify-v2:
    run: "npm test"
```

### Consumer Upgrades

```bash
# Check status
$ graft status
meta-kb: v1.5.0 (v2.0.0 available)

# View changes
$ graft changes meta-kb
v2.0.0 (breaking): Renamed getUserData → fetchUserData
  Migration: migrate-v2 (automatic)

# Upgrade
$ graft upgrade meta-kb --to v2.0.0
Running migration...
✓ Modified 15 files
Running verification...
✓ All tests passed
✓ Upgraded to v2.0.0
```

## Open Questions

- Caching strategy for fetched dependencies
- ~~Transitive dependency handling~~ ✅ Resolved (v2.0 - flat layout with extended lock file)
- Multi-version upgrade paths (v1 → v2 → v3)
- Parallel command execution
- Cross-platform compatibility (Windows, macOS, Linux)
- Workspace/monorepo support (deferred - see Decision 0006)

## Related

- [Brainstorming: Upgrade Mechanisms](../../notes/2026-01-01-upgrade-mechanisms.md)
- [Use Cases](use-cases.md) (to be created)

## References

- Git internals: https://git-scm.com/book/en/v2/Git-Internals
- Semantic Versioning: https://semver.org/ (optional, not required)
- Keep a Changelog: https://keepachangelog.com/
- Codemods: https://github.com/facebook/jscodeshift
