---
status: stable
updated: 2026-01-05
---

# CLI Reference

Complete reference for all graft commands.

## Documentation Sources

This reference documents implemented commands with links to specifications and code.

**For each command:**
- **Specification:** [Core Operations Spec](../../graft-knowledge/docs/specification/core-operations.md)
- **Implementation:** `src/graft/cli/commands/` (linked per command below)
- **Tests:** `tests/integration/test_cli_commands.py` (805 lines of CLI tests)

---

## Command Overview

| Command | Purpose |
|---------|---------|
| [resolve](#graft-resolve) | Clone all dependencies |
| [fetch](#graft-fetch) | Update remote cache |
| [apply](#graft-apply) | Update lock file without migrations |
| [status](#graft-status) | Show current versions |
| [changes](#graft-changes) | List available changes |
| [show](#graft-show) | Display change details |
| [upgrade](#graft-upgrade) | Atomic upgrade with migrations |
| [dep:command](#graft-depcommand) | Execute dependency command |
| [validate](#graft-validate) | Validate configuration |

---

## graft resolve

Clone or fetch all dependencies declared in `graft.yaml`.

```bash
uv run python -m graft resolve
```

**Behavior**:
- Clones dependencies to `.graft/deps/<dep-name>/`
- Fetches latest from remote if already cloned
- Does NOT create or modify lock file
- Does NOT checkout any specific ref

**Use Cases**:
- Initial setup after cloning a project
- Synchronize dependency repositories
- Ensure all dependencies are available locally

---

## graft fetch

Update local cache of dependencies from remote repositories.

```bash
# Fetch all dependencies
uv run python -m graft fetch

# Fetch specific dependency
uv run python -m graft fetch my-knowledge
```

**Behavior**:
- Runs `git fetch` to update remote-tracking branches
- Does NOT modify the lock file
- Does NOT modify working directory
- Does NOT checkout any refs

**Use Cases**:
- Check for new versions without modifying lock file
- Update local knowledge of what's available before upgrading
- Refresh repository metadata

**Note**: Use `graft changes` after fetching to see what's available.

---

## graft apply

Update the lock file to acknowledge a specific version without running migrations.

```bash
uv run python -m graft apply <dep-name> --to <ref>
```

**Examples**:
```bash
# Initial lock file creation
uv run python -m graft apply my-knowledge --to main

# Update to specific version without migration
uv run python -m graft apply my-knowledge --to v1.0.0
```

**Arguments**:
- `dep-name` - Name of dependency (must exist in graft.yaml)
- `--to <ref>` - Git ref to apply (branch, tag, or commit hash)

**Behavior**:
- Resolves ref to commit hash
- Updates `graft.lock` with new version
- Does NOT run migration commands
- Does NOT run verification commands

**Use Cases**:
- Initial lock file creation
- Manual migration workflows
- Acknowledging a version without automated migration

**Difference from upgrade**: `apply` skips migration and verification commands. Use `upgrade` for automated migrations.

---

## graft status

Show current consumed versions from the lock file.

```bash
# Show all dependencies
uv run python -m graft status

# Show specific dependency
uv run python -m graft status <dep-name>

# JSON output for scripting
uv run python -m graft status --format json

# Check for available updates
uv run python -m graft status --check-updates
```

**Options**:
- `--format <text|json>` - Output format (default: text)
- `--check-updates` - Fetch latest and check for available updates

**Output (text)**:
```
Dependencies:
  my-knowledge: v1.5.0 (commit: abc123..., consumed: 2026-01-04 10:30:00)
```

**Output (json)**:
```json
{
  "dependencies": {
    "my-knowledge": {
      "current_ref": "v1.5.0",
      "commit": "abc123...",
      "consumed_at": "2026-01-04T10:30:00+00:00"
    }
  }
}
```

**With --check-updates**:
- Runs `git fetch` on dependencies
- Shows if ref has moved (updates available)
- Does NOT modify lock file

---

## graft changes

List available changes/versions for a dependency.

```bash
# List all changes
uv run python -m graft changes <dep-name>

# Filter by type
uv run python -m graft changes <dep-name> --type feature
uv run python -m graft changes <dep-name> --breaking

# Filter by ref range
uv run python -m graft changes <dep-name> --from-ref v1.0.0 --to-ref v2.0.0

# Show changes since a specific ref
uv run python -m graft changes <dep-name> --since v1.0.0

# JSON output
uv run python -m graft changes <dep-name> --format json
```

**Options**:
- `--type <breaking|feature|fix>` - Filter by change type
- `--breaking` - Shortcut for `--type breaking`
- `--from-ref <ref>` - Show changes from this ref (exclusive)
- `--to-ref <ref>` - Show changes to this ref (inclusive)
- `--since <ref>` - Alias for `--from-ref`
- `--format <text|json>` - Output format

**Output (text)**:
```
Changes for my-knowledge:
  v2.0.0 (breaking)
    Major restructuring
    Migration: migrate-v2

  v1.5.0 (feature)
    Additional examples
```

**Use Cases**:
- Explore available versions
- Identify breaking changes before upgrading
- See what's new in a dependency

---

## graft show

Display detailed information about a specific change.

```bash
# Show change details
uv run python -m graft show <dep-name@ref>

# Show only specific field
uv run python -m graft show <dep-name@ref> --field <field>

# JSON output
uv run python -m graft show <dep-name@ref> --format json
```

**Examples**:
```bash
uv run python -m graft show my-knowledge@v2.0.0
uv run python -m graft show my-knowledge@v2.0.0 --field migration
uv run python -m graft show my-knowledge@v2.0.0 --format json
```

**Options**:
- `--field <type|description|migration|verify>` - Show only specific field
- `--format <text|json>` - Output format

**Output (text)**:
```
Change: my-knowledge@v2.0.0
Type: breaking
Description: Major restructuring

Migration: migrate-v2
  Command: ./scripts/migrate-to-v2.sh
  Description: Migrate to v2 structure

Verification: verify-v2
  Command: ./scripts/verify-v2.sh
  Description: Verify v2 migration succeeded
```

---

## graft upgrade

Perform an atomic upgrade with migration execution and automatic rollback on failure.

```bash
# Upgrade with migration and verification
uv run python -m graft upgrade <dep-name> --to <ref>

# Preview upgrade without making changes
uv run python -m graft upgrade <dep-name> --to <ref> --dry-run

# Skip migration
uv run python -m graft upgrade <dep-name> --to <ref> --skip-migration

# Skip verification
uv run python -m graft upgrade <dep-name> --to <ref> --skip-verify
```

**Arguments**:
- `dep-name` - Name of dependency
- `--to <ref>` - Target version (required)

**Options**:
- `--dry-run` - Preview upgrade without making any changes
- `--skip-migration` - Skip migration command execution
- `--skip-verify` - Skip verification command execution

**Upgrade Process**:
1. Creates snapshot of current state
2. Runs migration command (if defined and not skipped)
3. Runs verification command (if defined and not skipped)
4. Updates lock file
5. Automatically rolls back on any failure

**Automatic Rollback**:
- If migration fails: Restore snapshot, revert lock file
- If verification fails: Restore snapshot, revert lock file
- If lock update fails: Restore snapshot

**Use Cases**:
- Upgrade to new version with automated migration
- Test upgrade with `--dry-run` before committing
- Manual migration with `--skip-migration`, then run migration separately

**Difference from apply**: `upgrade` runs migrations and verifications. Use `apply` to skip migrations.

---

## graft dep:command

Execute a command defined in a dependency's graft.yaml.

```bash
uv run python -m graft <dep-name>:<command> [args...]
```

**Examples**:
```bash
# Execute migration command from dependency
uv run python -m graft my-knowledge:migrate-v2

# Execute with additional arguments
uv run python -m graft my-knowledge:build --production
```

**Behavior**:
- Loads command from dependency's graft.yaml
- Executes in consumer's context (current directory, not `.graft/deps/`)
- Streams stdout/stderr in real-time
- Returns same exit code as command
- Passes additional arguments to the command

**Use Cases**:
- Run migration commands manually
- Execute verification commands
- Run utility scripts defined by dependencies

**Example graft.yaml (in dependency)**:
```yaml
commands:
  migrate-v2:
    run: "./scripts/migrate-to-v2.sh"
    description: "Migrate to v2 structure"
```

---

## graft validate

Validate graft.yaml and graft.lock for correctness.

```bash
# Validate everything (default)
uv run python -m graft validate

# Validate only graft.yaml schema
uv run python -m graft validate --schema

# Validate only graft.lock
uv run python -m graft validate --lock

# Validate only git refs exist
uv run python -m graft validate --refs
```

**Options**:
- `--schema` - Validate only graft.yaml schema
- `--lock` - Validate only graft.lock consistency
- `--refs` - Validate only git ref existence

**Checks Performed**:

**Schema validation** (`--schema`):
- graft.yaml structure is valid
- API version is correct
- At least one dependency exists
- Command references are valid

**Git ref validation** (`--refs`):
- All refs in dependency changes exist in git repositories

**Lock file validation** (`--lock`):
- Lock file format is correct
- All dependencies in lock file exist in graft.yaml
- Warns if refs have moved (commit hash changed)

**Exit Codes**:
- `0` - Validation successful (warnings allowed)
- `1` - Validation failed (errors found)

**Output Symbols**:
- ✓ Green checkmark - Validation passed
- ✗ Red X - Error found
- ⚠ Yellow warning - Non-critical issue

**Use Cases**:
- Pre-commit validation
- CI/CD pipeline checks
- Debugging configuration issues

---

## Global Options

All commands support:
- Standard input/output redirection
- Exit codes (0 = success, 1 = error)
- Color-coded output (can be disabled with `NO_COLOR=1`)

## Environment Variables

- `NO_COLOR=1` - Disable colored output
- `GRAFT_DEBUG=1` - Enable debug logging (not yet implemented)

---

See [User Guide](guides/user-guide.md) for workflow examples and [Configuration Guide](configuration.md) for file format details.
