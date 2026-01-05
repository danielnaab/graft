---
status: stable
updated: 2026-01-05
---

# Graft User Guide

Complete guide to using Graft for semantic dependency management.

This guide provides step-by-step tutorials, common workflows, troubleshooting tips, and best practices for using Graft effectively.

## About This Guide

> **Authority Note:** This guide interprets canonical specifications from [graft-knowledge](../../../graft-knowledge/) for practical application. When specifications and this guide conflict, specifications are authoritative.

This guide provides practical applications of Graft specifications:

**Canonical Specifications:**
- [Change Model](../../../graft-knowledge/docs/specification/change-model.md) - Semantic change definitions
- [graft.yaml Format](../../../graft-knowledge/docs/specification/graft-yaml-format.md) - Configuration schema
- [Lock File Format](../../../graft-knowledge/docs/specification/lock-file-format.md) - Lock file schema
- [Core Operations](../../../graft-knowledge/docs/specification/core-operations.md) - Command semantics

**Implementation References:**
- CLI commands: `src/graft/cli/commands/*.py`
- Configuration parser: `src/graft/services/config_service.py`
- Examples tested against: Working implementation (Phase 1-8 complete)

---

## Table of Contents

1. [Getting Started](#getting-started)
2. [Core Concepts](#core-concepts)
3. [Common Workflows](#common-workflows)
4. [Troubleshooting](#troubleshooting)
5. [Best Practices](#best-practices)
6. [Advanced Topics](#advanced-topics)

---

## Getting Started

### Your First Graft Project

This tutorial walks you through setting up and using Graft for the first time.

#### Step 1: Install Graft

```bash
# Clone the graft repository
git clone <graft-repository-url>
cd graft

# Install dependencies using uv
uv sync

# Verify installation
uv run python -m graft --help
```

You should see the Graft CLI help text with available commands.

#### Step 2: Create Your First graft.yaml

Create a new directory for your project and add a `graft.yaml` file:

```bash
# Create a new project
mkdir my-project
cd my-project

# Create graft.yaml
cat > graft.yaml <<EOF
apiVersion: graft/v0
deps:
  my-knowledge: "https://github.com/user/knowledge.git#main"
EOF
```

This declares a dependency named `my-knowledge` that tracks the `main` branch of a git repository.

#### Step 3: Clone Dependencies

```bash
uv run python -m graft resolve
```

This command:
- Clones the repository to `.graft/deps/my-knowledge/`
- Makes the repository ready to use but does not yet create a lock file

Expected output:
```
Resolving dependencies...
  ✓ my-knowledge: cloned successfully
```

#### Step 4: Create Initial Lock File

```bash
uv run python -m graft apply my-knowledge --to main
```

This command:
- Resolves `main` to a specific commit hash
- Creates `graft.lock` with the consumed version
- Records the timestamp of consumption

Expected output:
```
Applied my-knowledge@main
Lock file updated: graft.lock
```

The graft.lock file will contain:
```yaml
apiVersion: graft/v0
dependencies:
  my-knowledge:
    source: "https://github.com/user/knowledge.git"
    ref: main
    commit: abc123def456...
    consumedAt: "2026-01-04T12:34:56Z"
```

#### Step 5: Check Status

```bash
uv run python -m graft status
```

Expected output:
```
Dependencies:
  my-knowledge:
    Current: main
    Commit: abc123def456...
    Consumed: 2026-01-04T12:34:56Z
```

You have successfully set up your first Graft project.

---

## Core Concepts

### Dependencies (deps)

Dependencies are git repositories that your project depends on. Each dependency has:

- Name: A unique identifier (e.g., `my-knowledge`)
- URL: The git repository URL
- Ref: The branch, tag, or commit to track (e.g., `main`, `v1.0.0`)

```yaml
deps:
  meta-kb: "https://github.com/org/meta-kb.git#v2.0.0"
  utils: "https://github.com/org/utils.git#main"
```

### Changes

Changes are semantic versioned modifications to dependencies. Each change can define:

- Type: `breaking`, `feature`, or `fix`
- Description: Human-readable explanation
- Migration: Optional command to run during upgrade
- Verification: Optional command to verify the upgrade

```yaml
changes:
  v2.0.0:
    type: breaking
    description: "Renamed getUserData → fetchUserData"
    migration: migrate-v2
    verify: verify-v2
```

### Commands

Reusable shell commands defined in `graft.yaml`:

```yaml
commands:
  migrate-v2:
    run: "npx jscodeshift -t codemods/v2.js src/"
    description: "Rename getUserData to fetchUserData"
    working_dir: "."
```

### Lock File (graft.lock)

The lock file records exactly what version you are consuming:

- Commit hash (not just ref name)
- Timestamp of consumption
- Source repository URL

This ensures reproducible builds and auditable upgrades.

---

## Common Workflows

### Workflow 1: Adding a New Dependency

Add a new knowledge base to your project:

```bash
# 1. Add dependency to graft.yaml
cat >> graft.yaml <<EOF
  new-kb: "https://github.com/org/new-kb.git#v1.0.0"
EOF

# 2. Clone the dependency
uv run python -m graft resolve

# 3. Apply the initial version
uv run python -m graft apply new-kb --to v1.0.0

# 4. Verify it's locked
uv run python -m graft status new-kb
```

The new dependency is cloned and locked at `v1.0.0`.

### Workflow 2: Checking for Updates

See if there are new versions available without upgrading:

```bash
# 1. Fetch latest from remote
uv run python -m graft fetch my-knowledge

# 2. Check status with updates
uv run python -m graft status my-knowledge --check-updates

# 3. List available changes
uv run python -m graft changes my-knowledge

# 4. View details of a specific version
uv run python -m graft show my-knowledge@v2.0.0
```

You can see what is available without modifying your lock file.

### Workflow 3: Upgrading a Dependency

Upgrade to a new version with migration:

```bash
# 1. Preview the upgrade first
uv run python -m graft upgrade my-knowledge --to v2.0.0 --dry-run

# 2. Review the planned operations

# 3. Perform the actual upgrade
uv run python -m graft upgrade my-knowledge --to v2.0.0

# 4. Verify the upgrade succeeded
uv run python -m graft status my-knowledge
```

During upgrade:
1. Snapshot created (graft.lock backed up)
2. Migration command runs
3. Verification command runs
4. Lock file updated
5. If anything fails: Automatic rollback to previous state

Safe atomic upgrade with automatic rollback on failure.

### Workflow 4: Handling Breaking Changes

A dependency has a breaking change with a migration command:

```bash
# 1. Check what the breaking change involves
uv run python -m graft show my-knowledge@v2.0.0

# Output shows:
# Type: breaking
# Description: Renamed getUserData → fetchUserData
# Migration: migrate-v2
#   Command: npx jscodeshift -t codemods/v2.js src/
# Verify: verify-v2
#   Command: npm test && ! grep -r 'getUserData' src/

# 2. Run the upgrade (migration executes automatically)
uv run python -m graft upgrade my-knowledge --to v2.0.0
```

Breaking change migrated and verified automatically.

### Workflow 5: Manual Migration Workflow

Run migration manually instead of during upgrade:

```bash
# 1. Apply version without running migration
uv run python -m graft apply my-knowledge --to v2.0.0

# 2. Run migration command manually
uv run python -m graft my-knowledge:migrate-v2

# 3. Verify the migration
uv run python -m graft my-knowledge:verify-v2

# 4. Confirm final state
uv run python -m graft status my-knowledge
```

You have full control over when migration runs.

### Workflow 6: Validating Configuration

Validate your graft.yaml before committing:

```bash
# 1. Validate everything
uv run python -m graft validate

# 2. Validate only schema
uv run python -m graft validate --schema

# 3. Validate git refs exist
uv run python -m graft validate --refs

# 4. Validate lock file consistency
uv run python -m graft validate --lock
```

Catch configuration errors before they cause problems.

### Workflow 7: Scripting with JSON Output

Automate graft operations in CI/CD:

```bash
# Get status as JSON
STATUS=$(uv run python -m graft status --format json)
echo "$STATUS" | jq '.dependencies."my-knowledge".current_ref'

# List changes as JSON
CHANGES=$(uv run python -m graft changes my-knowledge --format json)
echo "$CHANGES" | jq '.changes[] | select(.type == "breaking")'

# Show change details as JSON
DETAILS=$(uv run python -m graft show my-knowledge@v2.0.0 --format json)
echo "$DETAILS" | jq '.migration.command'
```

Easy integration with automation tools.

---

## Troubleshooting

### Problem: "graft.yaml not found"

Cause: You are not in a directory with a graft.yaml file.

Solution:
```bash
# Check current directory
pwd

# Look for graft.yaml
ls -la graft.yaml

# If missing, create one
cat > graft.yaml <<EOF
apiVersion: graft/v0
deps:
  my-dep: "https://github.com/user/repo.git#main"
EOF
```

### Problem: "Dependency not cloned"

Cause: You have not run `graft resolve` yet.

Solution:
```bash
# Clone all dependencies
uv run python -m graft resolve
```

### Problem: "Ref 'v2.0.0' not found"

Cause: The ref does not exist in the git repository.

Solution:
```bash
# 1. Fetch latest from remote
uv run python -m graft fetch my-knowledge

# 2. Check available tags/branches
cd .graft/deps/my-knowledge
git tag
git branch -r

# 3. Use a valid ref
cd ../../..
uv run python -m graft upgrade my-knowledge --to <valid-ref>
```

### Problem: "Migration command failed"

Cause: The migration command encountered an error.

What happens:
- Graft automatically rolls back to previous state
- Lock file is restored from snapshot
- Error message shows migration output

Solution:
```bash
# 1. Review the error message

# 2. Fix the issue (e.g., missing dependency)
npm install required-package

# 3. Try upgrade again
uv run python -m graft upgrade my-knowledge --to v2.0.0
```

Graft's automatic rollback ensures you are never left in a broken state.

### Problem: "Lock file out of sync"

Cause: The dependency in .graft/deps/ does not match the lock file.

Solution:
```bash
# 1. Validate to see the issue
uv run python -m graft validate --lock

# 2. Resolve dependencies to fix
uv run python -m graft resolve

# 3. Verify consistency
uv run python -m graft validate --lock
```

### Problem: "Cannot access .graft/deps/"

Cause: Permissions issue or directory does not exist.

Solution:
```bash
# Check if directory exists
ls -ld .graft/

# If missing, create it
mkdir -p .graft/deps

# Run resolve to populate
uv run python -m graft resolve
```

---

## Best Practices

### 1. Always Use --dry-run First

Before upgrading, preview what will happen:

```bash
# Preview upgrade
uv run python -m graft upgrade my-knowledge --to v2.0.0 --dry-run

# Review the output, then execute
uv run python -m graft upgrade my-knowledge --to v2.0.0
```

Prevents surprises and lets you review migration commands.

### 2. Commit Lock File to Version Control

Always commit `graft.lock` to your repository:

```bash
git add graft.lock graft.yaml
git commit -m "Update dependencies"
```

Ensures everyone on the team uses the same dependency versions.

### 3. Use Semantic Versioning for Refs

Track semantic version tags, not just branches:

```yaml
# Good - explicit version
deps:
  my-kb: "https://github.com/org/my-kb.git#v1.0.0"

# Less good - moving target
deps:
  my-kb: "https://github.com/org/my-kb.git#main"
```

Semantic versions are stable and predictable.

### 4. Define Changes for All Breaking Changes

Always document breaking changes with migration commands:

```yaml
changes:
  v2.0.0:
    type: breaking
    description: "Renamed getUserData → fetchUserData"
    migration: migrate-v2
    verify: verify-v2
```

Makes upgrades safe and automatic for consumers.

### 5. Validate Before Committing

Run validation before committing changes:

```bash
# Validate everything
uv run python -m graft validate

# If valid, commit
git add graft.yaml graft.lock
git commit -m "Update configuration"
```

Catches errors early before they reach other developers.

### 6. Use JSON Output in CI/CD

In automated scripts, always use JSON output:

```bash
# In CI/CD pipeline
STATUS=$(uv run python -m graft status --format json)
if echo "$STATUS" | jq -e '.dependencies."my-kb"' > /dev/null; then
  echo "Dependency configured correctly"
fi
```

Reliable parsing and error handling in scripts.

### 7. Fetch Before Checking for Updates

Always fetch first to get latest information:

```bash
# Fetch, then check
uv run python -m graft fetch
uv run python -m graft status --check-updates
```

Ensures you see the most up-to-date information.

### 8. Use Commands for Reusable Operations

Define commands in your graft.yaml:

```yaml
commands:
  test:
    run: "pytest tests/"
    description: "Run test suite"

  lint:
    run: "ruff check src/"
    description: "Run linter"
```

Standardizes common operations across the team.

### 9. Document Migration Steps

Include clear descriptions in changes:

```yaml
changes:
  v2.0.0:
    type: breaking
    description: |
      Renamed getUserData → fetchUserData

      Migration involves:
      1. Running codemod to rename function
      2. Running tests to verify
      3. Checking no old API usage remains
    migration: migrate-v2
```

Helps users understand what will happen during upgrade.

### 10. Test Migrations in Isolation

Before upgrading, test migration commands manually:

```bash
# 1. Clone dependency at new version
cd .graft/deps/my-knowledge
git checkout v2.0.0

# 2. Test migration command in your project
cd ../../..
uv run python -m graft my-knowledge:migrate-v2

# 3. If successful, run actual upgrade
uv run python -m graft upgrade my-knowledge --to v2.0.0
```

Catches migration issues before committing to the upgrade.

---

## Advanced Topics

### Custom Migration Workflows

For complex migrations, skip automatic migration and do it manually:

```bash
# 1. Apply version without migration
uv run python -m graft apply my-knowledge --to v2.0.0

# 2. Run custom migration steps
./scripts/prepare-migration.sh
uv run python -m graft my-knowledge:migrate-v2
./scripts/post-migration.sh

# 3. Verify manually
uv run python -m graft my-knowledge:verify-v2
```

### Filtering Changes by Type

View only specific types of changes:

```bash
# Only breaking changes
uv run python -m graft changes my-knowledge --breaking

# Only features
uv run python -m graft changes my-knowledge --type feature

# Only fixes
uv run python -m graft changes my-knowledge --type fix
```

### Using Field Filters

Extract specific information from changes:

```bash
# Get only migration command
uv run python -m graft show my-knowledge@v2.0.0 --field migration

# Get only verification command
uv run python -m graft show my-knowledge@v2.0.0 --field verify

# Get only description
uv run python -m graft show my-knowledge@v2.0.0 --field description
```

### Automating Upgrades in CI/CD

Example GitHub Actions workflow:

```yaml
name: Update Dependencies
on:
  schedule:
    - cron: '0 0 * * 0'  # Weekly

jobs:
  update:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install uv
        run: pip install uv

      - name: Check for updates
        run: |
          uv run python -m graft fetch
          UPDATES=$(uv run python -m graft status --check-updates --format json)
          echo "$UPDATES" | jq .

      - name: Upgrade dependencies
        run: |
          uv run python -m graft upgrade my-knowledge --to v2.0.0

      - name: Create PR
        uses: peter-evans/create-pull-request@v4
        with:
          commit-message: "Update dependencies"
          title: "Update dependencies"
```

### Handling Multiple Dependencies

Upgrade multiple dependencies in sequence:

```bash
# Get all dependencies
DEPS=$(uv run python -m graft status --format json | jq -r '.dependencies | keys[]')

# Upgrade each
for dep in $DEPS; do
  echo "Checking $dep..."
  LATEST=$(git -C .graft/deps/$dep describe --tags --abbrev=0)
  uv run python -m graft upgrade $dep --to $LATEST
done
```

---

## Need Help?

- Documentation: See [README.md](../../README.md) for command reference
- Developer Guide: See [docs/README.md](../README.md) for architecture details
- Issue Tracker: Report bugs and request features on GitHub
- Discussions: Ask questions in GitHub Discussions

---

Last Updated: 2026-01-04
