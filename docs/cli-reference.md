---
status: stable
updated: 2026-02-02
---

# CLI Reference

Command reference for graft.

**Implementation:** `src/graft/cli/commands/`
**Tests:** `tests/integration/test_cli_commands.py`

> **Note:** Graft uses a flat-only dependency model (Decision 0007). All dependencies
> must be explicitly declared in `graft.yaml`. There is no transitive resolution.

---

## Commands

| Command | Purpose |
|---------|---------|
| resolve | Clone/fetch all dependencies from graft.yaml |
| sync | Sync .graft/ to match lock file state |
| add | Add a dependency to graft.yaml |
| remove | Remove a dependency from graft.yaml |
| tree | List dependencies |
| apply | Update lock without migrations |
| status | Show consumed versions |
| changes | List available changes |
| show | Display change details |
| upgrade | Atomic upgrade with migrations |
| fetch | Update remote cache |
| dep:cmd | Execute dependency command |
| validate | Validate configuration and integrity |

---

## resolve

Resolve all dependencies from `graft.yaml`.

```bash
graft resolve
```

**Behavior:**
- Clones dependencies to `.graft/<name>/`
- Fetches and checks out specified ref for existing repos
- Writes lock file with commit hashes

**Example output:**
```
Resolving dependencies...

  ✓ meta-knowledge-base: main → .graft/meta-knowledge-base
  ✓ python-starter: main → .graft/python-starter

Writing lock file...
  ✓ Updated graft.lock

Resolved: 2 dependencies

All dependencies resolved successfully!
```

---

## sync

Sync `.graft/` to match lock file state.

```bash
graft sync
```

**Behavior:**
- Reads `graft.lock`
- Clones missing dependencies
- Checks out correct commits for existing repos

**Example output:**
```
Syncing dependencies to lock file...

  ✓ meta-knowledge-base: Already at dd6ac96
  ✓ python-starter: Checked out 3eac821

Synced: 2/2 dependencies
```

---

## add

Add a dependency to graft.yaml.

```bash
graft add <name> <url>#<ref>
```

**Example:**
```bash
graft add my-kb https://github.com/user/repo.git#main
```

Does NOT resolve - run `graft resolve` after adding.

---

## remove

Remove a dependency from graft.yaml.

```bash
graft remove <name> [--keep-files]
```

**Options:**
- `--keep-files`: Keep the dependency files in `.graft/`

**Example:**
```bash
graft remove my-kb           # Removes from config and deletes files
graft remove my-kb --keep-files  # Removes from config only
```

---

## tree

List dependencies from lock file.

```bash
graft tree              # Simple list
graft tree --details    # With source and commit
```

**Simple view:**
```
Dependencies:

  meta-knowledge-base (main)
  python-starter (main)

Total: 2 dependencies
```

**Detailed view:**
```
Dependencies:

  meta-knowledge-base (main)
    source: https://github.com/user/meta-kb.git
    commit: dd6ac96

  python-starter (main)
    source: https://github.com/user/python-starter.git
    commit: 3eac821

Total: 2 dependencies
```

---

## apply

Update lock file without running migrations.

```bash
graft apply <dep> --to <ref>
```

Use cases:
- Initial lock file creation
- Manual migration workflows
- Acknowledge manual upgrades

Updates `graft.lock` with ref and commit hash.

---

## status

Show consumed versions from lock file.

```bash
graft status [<dep>]
```

Output:
- Dependency name
- Consumed ref
- Commit hash
- Consumed timestamp

---

## changes

List available changes for dependency.

```bash
graft changes <dep> [--type TYPE] [--from-ref REF] [--to-ref REF]
```

Options:
- `--type`: filter by breaking/feature/fix
- `--breaking`: show only breaking changes
- `--from-ref`, `--to-ref`: limit range

Output: ref, type, description, migration, verify.

---

## show

Display details for specific change.

```bash
graft show <dep> <ref> [--fields FIELDS]
```

Options:
- `--fields`: comma-separated fields to display

Output: all change metadata.

---

## upgrade

Atomic upgrade with migrations and rollback.

```bash
graft upgrade <dep> --to <ref> [--dry-run]
```

Behavior:
1. Create filesystem snapshot
2. Check for changes between current and target
3. Run migration commands in sequence
4. Update lock file on success
5. Rollback snapshot on failure

Options:
- `--dry-run`: show what would happen without executing

---

## fetch

Update remote cache for dependency.

```bash
graft fetch <dep>
```

Fetches latest from remote. Useful before checking for changes.

---

## dep:command

Execute command defined in dependency.

```bash
graft <dep>:<command>
```

Example:
```bash
graft meta-kb:build
```

Executes command in dependency's working directory.

---

## validate

Validate configuration and integrity.

```bash
graft validate [--config] [--lock] [--integrity]
```

**Modes:**
- `--config`: Validate graft.yaml schema only
- `--lock`: Validate graft.lock schema only
- `--integrity`: Validate .graft/ matches lock file
- (no flags): Run all validations

**Exit codes:**
- 0: All validations passed
- 1: Validation errors found
- 2: Integrity mismatch (with `--integrity`)

**Example:**
```
Validating graft.yaml...
  ✓ Schema is valid

Validating graft.lock...
  ✓ Schema is valid

Validating integrity...
  ✓ meta-knowledge-base: Commit matches
  ✓ python-starter: Commit matches

Validation successful
```

---

## Global Options

All commands support:
- `--help`: show help

---

## Examples

**Initial setup:**
```bash
graft add my-kb https://github.com/user/repo.git#main
graft resolve
```

**Check for updates:**
```bash
graft fetch my-kb
graft changes my-kb
```

**Upgrade:**
```bash
graft upgrade my-kb --to v2.0.0
```

**Sync to lock file:**
```bash
graft sync
```

**Validate integrity:**
```bash
graft validate --integrity
```

**Execute dependency command:**
```bash
graft meta-kb:build
```

---

## Sources

**Canonical Specifications:**
- [Core Operations](specifications/graft/core-operations.md) - operation definitions and behavior
- [graft.yaml Format](specifications/graft/graft-yaml-format.md) - dependency and command format
- [Lock File Format](specifications/graft/lock-file-format.md) - lock file schema
- [ADR 0001: Require Explicit Ref in Upgrade](specifications/decisions/decision-0001-initial-scope.md) - initial scope and design
- [ADR 0004: Atomic Upgrades](specifications/decisions/decision-0004-atomic-upgrades.md) - snapshot and rollback behavior
- [ADR 0007: Flat-Only Dependencies](specifications/decisions/decision-0007-flat-only-dependencies.md) - dependency resolution model

**Rust Implementation (Primary):**
- Command Dispatch: `crates/graft-cli/src/main.rs` (clap command definitions)
- Command Execution: `crates/graft-engine/src/command.rs` (dep:cmd implementation)
- Core Operations: `crates/graft-engine/src/` (resolve, apply, upgrade, sync, validate)
- Git Operations: `crates/graft-common/src/git.rs` (timeout-protected git ops)

**Python Implementation (Deprecated):**
- Commands: `src/graft/cli/commands/` (all CLI commands)
- Integration Tests: `tests/integration/test_cli_commands.py` (command validation)
