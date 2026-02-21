---
title: "Core Operations Specification"
date: 2026-01-01
status: draft
---

# Core Operations Specification

## Overview

This document specifies the core operations that Graft provides for managing dependency changes and upgrades.

Operations are divided into:
- **Query operations**: Read-only, inspect state
- **Mutation operations**: Modify state (lock file, files)

## Query Operations

### graft status

**Purpose**: Show current state of dependencies and available updates.

**Syntax**:
```bash
graft status [<dep-name>] [options]
```

**Options**:
- `--json`: Output as JSON
- `--check-updates`: Fetch latest from upstream and show available updates

**Behavior**:
1. Read graft.lock to get current consumed versions
2. Optionally fetch latest from upstream (if --check-updates)
3. Display current version and available updates for each dependency

**Output** (text):
```
Dependencies:
  meta-kb: v1.5.0 (consumed 2026-01-01)
  shared-utils: v2.0.0 → v2.1.0 available (1 feature)
```

**Output** (JSON):
```json
{
  "dependencies": {
    "meta-kb": {
      "current": "v1.5.0",
      "consumed_at": "2026-01-01T10:30:00Z",
      "available": null
    },
    "shared-utils": {
      "current": "v2.0.0",
      "consumed_at": "2025-12-15T14:20:00Z",
      "available": "v2.1.0",
      "pending_changes": 1
    }
  }
}
```


---

### graft changes

**Purpose**: List changes for a dependency.

**Syntax**:
```bash
graft changes <dep-name> [options]
```

**Options**:
- `--from <ref>`: Start ref (default: current consumed version)
- `--to <ref>`: End ref (default: latest)
- `--since <ref>`: Alias for `--from <ref> --to latest`
- `--type <type>`: Filter by type (breaking, feature, fix, etc.)
- `--breaking`: Show only breaking changes
- `--format <format>`: Output format (text, json)

**Behavior**:
1. Read dependency's graft.yaml
2. Determine ref range (from current consumed version to latest, or as specified)
3. Filter changes in that range
4. Optionally filter by type
5. Display changes

**Output** (text):
```
Changes for meta-kb: v1.5.0 → v2.0.0

v2.0.0 (breaking)
  Renamed getUserData → fetchUserData
  Migration: migrate-v2
  Verify: verify-v2

v1.6.0 (feature)
  Added response caching
  No migration required
```

**Output** (JSON):
```json
{
  "dependency": "meta-kb",
  "from": "v1.5.0",
  "to": "v2.0.0",
  "changes": [
    {
      "ref": "v1.6.0",
      "type": "feature",
      "description": "Added response caching",
      "migration": null,
      "verify": null
    },
    {
      "ref": "v2.0.0",
      "type": "breaking",
      "description": "Renamed getUserData → fetchUserData",
      "migration": "migrate-v2",
      "verify": "verify-v2"
    }
  ]
}
```


---

### graft show

**Purpose**: Show details of a specific change.

**Syntax**:
```bash
graft show <dep-name>@<ref> [options]
```

**Options**:
- `--format <format>`: Output format (text, json)
- `--field <field>`: Show only specific field (migration, verify, etc.)

**Behavior**:
1. Load dependency's graft.yaml
2. Find change for specified ref
3. Display full details

**Output** (text):
```
Change: meta-kb@v2.0.0

Type: breaking
Description: Renamed getUserData → fetchUserData

Migration: migrate-v2
  Command: npx jscodeshift -t codemods/v2.js src/
  Description: Rename getUserData to fetchUserData

Verification: verify-v2
  Command: npm test && ! grep -r 'getUserData' src/
  Description: Verify v2 migration: tests pass and no old API usage

See CHANGELOG.md for full details and rationale.
```

**Output** (JSON):
```json
{
  "ref": "v2.0.0",
  "type": "breaking",
  "description": "Renamed getUserData → fetchUserData",
  "migration": "migrate-v2",
  "verify": "verify-v2",
  "migration_command": {
    "name": "migrate-v2",
    "run": "npx jscodeshift -t codemods/v2.js src/",
    "description": "Rename getUserData to fetchUserData"
  },
  "verify_command": {
    "name": "verify-v2",
    "run": "npm test && ! grep -r 'getUserData' src/",
    "description": "Verify v2 migration: tests pass and no old API usage"
  }
}
```


---

### graft fetch

**Purpose**: Update local cache of dependency's remote state.

**Syntax**:
```bash
graft fetch [<dep-name>]
```

**Behavior**:
1. For each dependency (or specified dependency)
2. Fetch latest from remote repository
3. Update local cache of available refs and changes
4. Do not modify lock file or consumed version

**Output**:
```
Fetching meta-kb...
  ✓ Fetched latest from git@github.com:org/meta-kb.git
  Latest: v2.0.0

Fetching shared-utils...
  ✓ Fetched latest from ../shared-utils
  Latest: v2.1.0
```


---

## Resolution Operations

### graft resolve

**Purpose**: Clone or fetch **direct dependencies** specified in graft.yaml.

**Syntax**:
```bash
graft resolve
```

**Behavior**:
1. Find and parse graft.yaml in current directory
2. For each **direct dependency**:
   - If `.graft/<name>/` doesn't exist: add as git submodule and clone
   - If `.graft/<name>/` exists and is a git submodule: fetch and checkout ref
   - If `.graft/<name>/` exists but isn't a git submodule: error
3. Report resolution status for each dependency
4. Initialize and update git submodules for all dependencies

**Output** (success):
```
Found configuration: /path/to/project/graft.yaml
API Version: graft/v0
Dependencies: 2

Resolving dependencies...

✓ meta-kb: cloned to /path/to/project/.graft/meta-kb
✓ coding-standards: cloned to /path/to/project/.graft/coding-standards

Resolved: 2/2

All dependencies resolved successfully!
```

**Output** (failure):
```
Found configuration: /path/to/project/graft.yaml
API Version: graft/v0
Dependencies: 2

Resolving dependencies...

✓ meta-kb: resolved to /path/to/project/.graft/meta-kb
✗ coding-standards: Authentication failed
  Suggestion: Check SSH key configuration

Resolved: 1/2

Some dependencies failed to resolve.
```

**Exit codes**:
- `0` - All dependencies resolved
- `1` - One or more dependencies failed

**Notes**:
- Only **direct dependencies** are cloned (flat-only model)
- Dependencies are stored in `.graft/<name>/` (flat layout)
- Dependencies are tracked as git submodules in `.gitmodules`
- `.graft/` should NOT be in `.gitignore` (submodules are tracked by git)
- Paths in output are absolute for clarity


---

## Validation Operations

### graft validate

**Purpose**: Validate graft configuration files and dependency integrity.

**Syntax**:
```bash
graft validate [mode] [options]
```

**Modes**:
- `config` - Validate graft.yaml syntax and semantics
- `lock` - Validate graft.lock format and consistency
- `integrity` - Verify .graft/ directory matches lock file
- `all` - Run all validations (default)

**Options**:
- `--json`: Output as JSON
- `--fix`: Attempt to fix issues automatically (where possible)

**Behavior**:

**Mode: config**
1. Parse graft.yaml as YAML
2. Check required fields present
3. Validate git URLs format
4. Check command references are valid

**Mode: lock**
1. Parse graft.lock as YAML
2. Check apiVersion is supported
3. Validate all required fields present
4. Check commit hash format (40-char hex)
5. Validate timestamp format (ISO 8601)

**Mode: integrity**
1. For each dependency in lock file:
   - Check `.graft/<dep-name>/` exists and is a git submodule
   - Verify submodule is registered in `.gitmodules`
   - Run `git rev-parse HEAD` in submodule
   - Compare to commit hash in lock file
   - Report any mismatches
2. Check for orphaned submodules in `.graft/` not tracked in lock file

**Exit codes**:
- `0` - All validations passed
- `1` - Validation failed (invalid configuration)
- `2` - Integrity mismatch (lock vs .graft/)

**Output** (text):
```
✓ Config validation passed
  - graft.yaml is valid YAML
  - All dependencies have valid sources
  - All command references valid

✓ Lock file validation passed
  - graft.lock format is valid (apiVersion: graft/v0)
  - All required fields present
  - All dependencies declared in graft.yaml

✓ Integrity verification passed
  - meta-kb: submodule valid, commit matches (abc123...)
  - standards-kb: submodule valid, commit matches (def456...)

All validations passed ✓
```

**Output** (errors):
```
✗ Config validation failed
  - Line 15: Invalid git URL 'not-a-url'
  - Line 23: Command 'migrate-v3' referenced but not defined

✗ Lock file validation failed
  - Dependency 'meta-kb': missing 'commit' field
  - Dependency 'standards-kb': invalid commit hash 'not-a-hash'

✗ Integrity verification failed
  - templates-kb: Expected abc123..., found def456...
    Run 'graft sync' to update submodule

3 validation failures
```

**Output** (JSON):
```json
{
  "config": {
    "valid": false,
    "errors": [
      {
        "line": 15,
        "message": "Invalid git URL 'not-a-url'"
      }
    ]
  },
  "lock": {
    "valid": true,
    "errors": []
  },
  "integrity": {
    "valid": true,
    "mismatches": []
  },
  "overall": "failed"
}
```

**Validation Requirements**:

The implementation MUST:
- Support all three validation modes (config, lock, integrity)
- Return appropriate exit codes (0=success, 1=validation error, 2=integrity mismatch)
- Provide clear, actionable error messages
- Support both human-readable and JSON output formats

The implementation SHOULD:
- Report multiple errors, not just the first one
- Include line numbers for config errors where possible
- Suggest fixes for common errors

**Use Cases**:

1. **Pre-commit hook**:
```bash
#!/bin/bash
# .git/hooks/pre-commit
graft validate config lock
if [ $? -ne 0 ]; then
  echo "Graft validation failed. Fix errors before committing."
  exit 1
fi
```

2. **CI/CD pipeline**:
```yaml
# .github/workflows/validate.yml
- name: Validate Graft config
  run: graft validate --json
```

3. **Debug integrity issues**:
```bash
# Check if local .graft/ is in sync
graft validate integrity

# If mismatch, re-resolve
graft resolve
```

---

### graft sync

**Purpose**: Sync local `.graft/` directory to match lock file state.

**Syntax**:
```bash
graft sync [<dep-name>]
```

**Use case**: After pulling changes from teammates who upgraded dependencies, sync your local checkouts to match the lock file.

**Behavior**:
1. Read graft.lock
2. For each dependency (or specified dependency):
   - Check if `.graft/<name>/` exists
   - If exists: checkout the commit specified in lock file
   - If doesn't exist: clone and checkout
3. Update git submodules
4. Do NOT run migrations (teammate already ran them)

**Output**:
```
Syncing dependencies to lock file state...

meta-kb: v2.0.0 (abc123...)
  ✓ Checked out to commit abc123...

coding-standards: v1.5.0 (def456...)
  ✓ Already at correct commit

All dependencies synced!
```

**When to use:**
```bash
# Teammate upgraded a dependency and pushed
git pull

# You see graft.lock changed
git diff graft.lock

# Sync your local checkouts
graft sync
```

**Note:** This command does NOT run migrations. Migrations were already run by the person who upgraded and committed the lock file.

---

### graft inspect

**Purpose**: Inspect metadata and dependencies of a graft.

**Syntax**:
```bash
graft inspect <dep-name> [options]
```

**Options**:
- `--deps`: Show graft's own dependencies
- `--commands`: List available commands
- `--changes`: Show recent changes
- `--json`: Output as JSON

**Behavior**:
1. Read dependency's graft.yaml
2. Display requested information

**Output** (default):
```
Inspecting: meta-kb

Metadata:
  Name: meta-knowledge-base
  Description: Shared knowledge base for meta-cognitive patterns
  Version: v2.0.0

Location: /path/to/project/.graft/meta-kb
Source: git@github.com:org/meta-kb.git
Current ref: v2.0.0
```

**Output** (with --deps):
```
Inspecting: meta-kb

Dependencies:
  standards-kb: git@github.com:org/standards.git#v1.5.0
  templates-kb: git@github.com:org/templates.git#v1.0.0

Note: These are meta-kb's dependencies.
To use them, add them to YOUR graft.yaml.
```

**Output** (with --commands):
```
Inspecting: meta-kb

Commands:
  migrate-v2: Rename getUserData → fetchUserData
  verify-v2: Verify v2 migration completed
  changelog: Display changelog
```

**Use cases:**
1. **Discovery**: "What does this graft depend on?"
2. **Planning**: "If I add this graft, what else might I need?"
3. **Debugging**: "What commands are available?"

---

### graft add

**Purpose**: Add a new dependency to the project.

**Syntax**:
```bash
graft add <name> <source>#<ref> [options]
```

**Arguments**:
- `<name>`: Local name for the dependency (used in `.graft/<name>/`)
- `<source>`: Git URL or local path
- `<ref>`: Git ref to consume (tag, branch, or commit)

**Options**:
- `--no-resolve`: Add to graft.yaml only, don't clone
- `--json`: Output as JSON

**Behavior**:
1. Validate source is accessible
2. Validate ref exists in source repository
3. Add dependency to graft.yaml
4. Add as git submodule to `.graft/<name>/`
5. Update `.gitmodules` with submodule entry
6. Checkout specified ref
7. Update graft.lock with resolved commit hash

**Output** (success):
```
Adding dependency: meta-kb

Source: git@github.com:org/meta-kb.git
Ref: v2.0.0

✓ Added submodule to .graft/meta-kb
✓ Checked out v2.0.0 (abc123...)
✓ Updated .gitmodules
✓ Updated graft.yaml
✓ Updated graft.lock

Dependency added successfully!
```

**Output** (failure):
```
Adding dependency: meta-kb

Source: git@github.com:org/meta-kb.git
Ref: v2.0.0

✗ Failed to clone: Authentication failed
  Suggestion: Check SSH key configuration

Dependency not added.
```

**Exit codes**:
- `0` - Dependency added successfully
- `1` - Failed to add (clone failed, ref not found, etc.)

**Examples**:
```bash
# Add from GitHub with SSH
graft add meta-kb git@github.com:org/meta-kb.git#v2.0.0

# Add from HTTPS URL
graft add standards https://github.com/org/standards.git#main

# Add from local path
graft add shared-utils ../shared-utils#v1.0.0

# Add to config only (don't clone yet)
graft add meta-kb git@github.com:org/meta-kb.git#v2.0.0 --no-resolve
```

**Notes**:
- If dependency name already exists, command fails with error
- Source URL is stored in both graft.yaml and graft.lock
- The `#<ref>` syntax follows git URL fragment conventions

---

### graft remove

**Purpose**: Remove a dependency from the project.

**Syntax**:
```bash
graft remove <name> [options]
```

**Arguments**:
- `<name>`: Name of the dependency to remove

**Options**:
- `--keep-files`: Remove from config but keep `.graft/<name>/` directory
- `--json`: Output as JSON

**Behavior**:
1. Verify dependency exists in graft.yaml
2. Remove dependency from graft.yaml
3. Remove dependency from graft.lock
4. Remove submodule entry from `.gitmodules`
5. Delete `.graft/<name>/` directory (unless --keep-files)

**Note**: With `--keep-files`, the submodule entry is still removed from `.gitmodules`, but the files remain as an untracked directory.

**Output** (success):
```
Removing dependency: meta-kb

✓ Removed from graft.yaml
✓ Removed from graft.lock
✓ Removed from .gitmodules
✓ Deleted .graft/meta-kb/

Dependency removed successfully!
```

**Output** (with --keep-files):
```
Removing dependency: meta-kb

✓ Removed from graft.yaml
✓ Removed from graft.lock
✓ Removed from .gitmodules
⚠ Kept .graft/meta-kb/ as untracked directory (use 'rm -rf .graft/meta-kb' to delete)

Dependency removed from configuration.
```

**Output** (failure):
```
Removing dependency: meta-kb

✗ Dependency 'meta-kb' not found in graft.yaml

Available dependencies:
  - coding-standards
  - templates-kb
```

**Exit codes**:
- `0` - Dependency removed successfully
- `1` - Failed to remove (not found, permission error, etc.)

**Examples**:
```bash
# Remove dependency completely
graft remove meta-kb

# Remove from config but keep cloned files
graft remove meta-kb --keep-files
```

**Notes**:
- This operation does NOT revert any migrations that were applied
- Consumer is responsible for any cleanup of files modified by the dependency
- Use `--keep-files` if you want to preserve the cloned repository for reference

---

## Mutation Operations

### graft upgrade

**Purpose**: Upgrade a dependency to a new version. Atomic operation that runs migration, verification, and updates lock file.

**Syntax**:
```bash
graft upgrade <dep-name> [options]
```

**Options**:
- `--to <ref>`: Target ref (default: latest)
- `--dry-run`: Show what would be done without executing
- `--skip-migration`: Skip migration command (not recommended)
- `--skip-verify`: Skip verification command (not recommended)

**Behavior** (atomic):
1. Validate target ref exists
2. Create snapshot for rollback
3. Update files to new version
4. Run migration command (if defined)
5. Run verification command (if defined)
6. Update lock file
7. On failure: rollback all changes

**Output** (success):
```
Upgrading meta-kb: v1.5.0 → v2.0.0

Running migration: migrate-v2
  Command: npx jscodeshift -t codemods/v2.js src/
  ✓ Processed 15 files

Running verification: verify-v2
  Command: npm test && ! grep -r 'getUserData' src/
  ✓ 127 tests passed
  ✓ No occurrences of 'getUserData' found

✓ Upgrade complete
Updated graft.lock: meta-kb@v2.0.0
```

**Output** (failure):
```
Upgrading meta-kb: v1.5.0 → v2.0.0

Running migration: migrate-v2
  Command: npx jscodeshift -t codemods/v2.js src/
  ✓ Processed 15 files

Running verification: verify-v2
  Command: npm test && ! grep -r 'getUserData' src/
  ✗ 3 tests failed

Upgrade failed. Rolling back changes...
  ✓ Reverted file modifications

Lock file remains at v1.5.0

Error: Verification failed
To retry after fixing:
  1. Fix failing tests
  2. Run: graft upgrade meta-kb --to v2.0.0
```


---

### graft apply

**Purpose**: Update lock file to acknowledge a version without running migrations. For manual migration workflows.

**Syntax**:
```bash
graft apply <dep-name> --to <ref>
```

**Behavior**:
1. Validate ref exists
2. Update lock file immediately
3. Do not run migrations or verification

**Use case**: When user has manually performed migrations and wants to update lock file.

**Output**:
```
Applied meta-kb@v2.0.0
Updated graft.lock

Note: No migrations were run. Ensure you've completed all required migrations manually.
```


---

### graft validate

**Purpose**: Validate graft.yaml and graft.lock for correctness.

**Syntax**:
```bash
graft validate [options]
```

**Options**:
- `--schema`: Validate YAML schema only
- `--refs`: Validate git refs exist
- `--lock`: Validate lock file consistency

**Behavior**:
1. Validate graft.yaml structure
2. Check that all migration/verify commands exist
3. Check that all refs in changes exist in git
4. Validate lock file structure
5. Verify lock file refs resolve to expected commits

**Output** (success):
```
Validating graft.yaml...
  ✓ Schema is valid
  ✓ All migration commands exist
  ✓ All verify commands exist
  ✓ All refs exist in git repository

Validating graft.lock...
  ✓ Schema is valid
  ✓ All refs exist
  ✓ All commits match
  ✓ No integrity issues

Validation successful
```

**Output** (failure):
```
Validating graft.yaml...
  ✓ Schema is valid
  ✗ Change 'v2.0.0': migration 'migrate-v2' not found in commands
  ✗ Ref 'v3.0.0' does not exist in git repository

Validating graft.lock...
  ✓ Schema is valid
  ⚠ Warning: meta-kb ref 'main' has moved (commit changed)

Validation failed with 2 errors, 1 warning
```


---

## Command Execution

### graft <dep>:<command>

**Purpose**: Execute a command defined in dependency's graft.yaml.

**Syntax**:
```bash
graft <dep-name>:<command-name> [args...]
```

**Behavior**:
1. Load dependency's graft.yaml
2. Find command definition
3. Execute in consumer's context
4. Pass through stdout/stderr

**Example**:
```bash
$ graft meta-kb:migrate-v2

Running: npx jscodeshift -t codemods/v2.js src/
Processing 15 files...
✓ Completed
```

### graft run <command>

**Purpose**: Execute a command defined in current repository's graft.yaml.

**Syntax**:
```bash
graft run <command-name> [args...]
graft run <command-name> --dry-run  # Show what would be executed
graft run                           # List available commands
```

**Options**:
- `--dry-run`: Show what would be executed without running the command. Resolves context, renders stdin templates, and prints the resolved command configuration. Does not execute the command.

**Behavior**:
1. Search for graft.yaml in current directory, then parent directories (like git)
2. Parse `commands` section from graft.yaml
3. If no arguments provided, list available commands and exit
4. If command name provided, find command definition
5. If command has `context:` entries, resolve each state query and expose results as:
   - Environment variables: `GRAFT_STATE_<NAME>` (uppercase, hyphens → underscores)
   - Template variables: `{{ state.<name> }}` (for stdin template rendering)
6. If command has `stdin:`, prepare the text:
   - Literal: use as-is
   - Template: render with the specified engine (or tera by default), injecting context variables
7. If `--dry-run`: print resolved configuration (command, env, stdin text) and exit
8. Execute in current directory context, piping stdin text if present
9. Pass through stdout/stderr in real-time
10. Exit with command's exit code

**Command Resolution**:
- If `<command-name>` contains `:`, parse as `<dep>:<cmd>` and execute dependency command
- Otherwise, look up command in current repo's graft.yaml
- If command not found, display error and list available commands

**Example** (execute command):
```bash
$ graft run test

Executing: test
  Command: pytest tests/

Running tests...
.............................
✓ Command completed successfully

$ echo $?
0
```

**Example** (list commands):
```bash
$ graft run

Available commands in ./graft.yaml:

  test          Run test suite
  migrate       Run database migrations
  build         Build documentation

Use: graft run <command-name>
```

**Example** (command not found):
```bash
$ graft run invalid

Error: Command 'invalid' not found in graft.yaml

Available commands:
  test          Run test suite
  migrate       Run database migrations
  build         Build documentation
```

**Example** (no graft.yaml found):
```bash
$ graft run test

Error: No graft.yaml found in current directory or parent directories
```

**Example** (with arguments):
```bash
$ graft run test --verbose --filter=unit

Executing: test
  Command: pytest tests/
  Arguments: --verbose --filter=unit

pytest tests/ --verbose --filter=unit
...
```

**Example** (dependency command via run):
```bash
$ graft run meta-kb:migrate-v2

Executing: meta-kb:migrate-v2
  Command: npx jscodeshift -t codemods/v2.js src/
...
```

**Example** (dry-run with context and stdin):
```bash
$ graft run generate-report --dry-run

Dry run: generate-report
  Command: report-tool generate
  Context:
    coverage: 87.5
  Stdin (rendered from templates/report.md):
    # Coverage Report
    Current coverage: 87.5%
  Environment:
    GRAFT_STATE_COVERAGE=87.5
```

**Example** (dry-run for simple command):
```bash
$ graft run test --dry-run

Dry run: test
  Command: cargo test
  No stdin
  No context
```

**Error Cases**:
- No graft.yaml found: Exit with code 1, display helpful message
- Command not found: Exit with code 1, list available commands
- Command execution fails: Exit with command's exit code
- graft.yaml parse error: Exit with code 1, display parse error

**Related**:
- For dependency commands: `graft run <dep>:<command>` or legacy `graft <dep>:<command>`
- For current repo commands: `graft run <command>`


---

### graft ps

**Purpose**: List active processes currently managed by graft.

**Syntax**:
```bash
graft ps [options]
```

**Options**:
- `--repo <path>`: Filter results to processes associated with the given repository path

**Behavior**:
1. Open `FsProcessRegistry` at `~/.cache/graft/processes/`
2. Call `list_active()`, which prunes stale entries for dead PIDs automatically
3. If `--repo` is given, filter to entries whose `repo_path` matches
4. Display results; exit 0 even if no processes are found

**Output** (processes active):
```
Active processes (2):
  PID 12345  make test
    Repository: /home/user/my-project
    Started:    2026-02-19T10:00:00Z
    Status:     Running

  PID 67890  pytest tests/ --cov
    Repository: /home/user/other-project
    Started:    2026-02-19T09:55:00Z
    Status:     Running
```

**Output** (no active processes):
```
No active processes.
```

**Registry path**: `~/.cache/graft/processes/{pid}.json` (created automatically on first use)

**Note**: `graft ps` shows only processes spawned with registry integration enabled (via
`ProcessHandle::spawn_registered` or `run_to_completion_with_timeout_registered`). Processes
spawned without registry integration are not listed.

---

## Operation Flow Diagram

```
┌──────────────┐
│ graft status │ → Read lock file → Display current state
└──────────────┘

┌──────────────┐
│ graft fetch  │ → Fetch from remote → Update cache
└──────────────┘

┌───────────────┐
│ graft changes │ → Load graft.yaml → Filter & display
└───────────────┘

┌─────────────┐
│ graft show  │ → Load change details → Display
└─────────────┘

┌───────────────┐
│ graft upgrade │ → Create snapshot → Update files
└───────────────┘         ↓
                  Run migration
                          ↓
                  Run verification
                          ↓
                  Update lock file
                          ↓
                  (On fail: rollback)

┌──────────────┐
│ graft apply  │ → Update lock file (no migration)
└──────────────┘

┌─────────────────┐
│ graft validate  │ → Validate YAML → Check refs → Verify integrity
└─────────────────┘

┌─────────────┐
│ graft run   │ → Find graft.yaml → Execute command → Stream output
└─────────────┘
```

## Related

- [Specification: Change Model](./change-model.md)
- [Specification: graft.yaml Format](./graft-yaml-format.md)
- [Specification: Lock File Format](./lock-file-format.md)
- [Specification: Dependency Layout](./dependency-layout.md)
- [Decision 0004: Atomic Upgrades](../decisions/decision-0004-atomic-upgrades.md)
- [Decision 0007: Flat-Only Dependency Model](../decisions/decision-0007-flat-only-dependencies.md)

## Changelog

- **2026-01-31**: Updated for flat-only dependency model (v3)
  - Updated `graft resolve` to clarify direct dependencies only
  - Updated `graft validate` to remove dependency graph checks
  - Added `graft sync` operation
  - Added `graft inspect` operation
  - Added references to Decision 0007

- **2026-02-21**: Added stdin/context/dry-run to graft run
  - `--dry-run` option resolves context and renders stdin without executing
  - Context resolution: state queries resolved and exposed as env vars
  - Stdin piping: literal or template text piped to command process

- **2026-01-01**: Initial draft
