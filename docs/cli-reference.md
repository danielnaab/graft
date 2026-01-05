---
status: stable
updated: 2026-01-05
---

# CLI Reference

Command reference for graft.

**Specification:** [Core Operations](../../graft-knowledge/docs/specification/core-operations.md)
**Implementation:** `src/graft/cli/commands/`
**Tests:** `tests/integration/test_cli_commands.py`

---

## Commands

| Command | Purpose |
|---------|---------|
| resolve | Clone/fetch all dependencies (including transitive) |
| tree | Visualize dependency graph |
| apply | Update lock without migrations |
| status | Show consumed versions |
| changes | List available changes |
| show | Display change details |
| upgrade | Atomic upgrade with migrations |
| fetch | Update remote cache |
| dep:cmd | Execute dependency command |
| validate | Validate configuration |

---

## resolve

Recursively resolve all dependencies (direct + transitive) from `graft.yaml`.

```bash
graft resolve
```

**Behavior (v2):**
- Clones dependencies to `.graft/<name>/`
- Recursively resolves transitive dependencies
- Detects version conflicts and fails with clear error
- Writes complete lock file with all dependencies
- Shows direct vs transitive deps in output

**Example output:**
```
Resolving dependencies (including transitive)...

Direct dependencies:
  ✓ meta-kb: v2.0.0 → .graft/meta-kb

Transitive dependencies:
  ✓ standards-kb: v1.5.0 → .graft/standards-kb (via meta-kb)

Writing lock file...
  ✓ Updated ./graft.lock

Resolved: 2 dependencies
  Direct: 1
  Transitive: 1
```

---

## tree

Visualize dependency tree from lock file.

```bash
graft tree              # Tree view
graft tree --show-all   # Detailed view
```

**Tree view:**
```
Dependencies:
  meta-kb (v2.0.0) [direct]
    └── standards-kb (v1.5.0)
        └── templates-kb (v1.0.0)
```

**Detailed view:**
```
Dependencies:

  meta-kb (v2.0.0) [direct]
    source: git@github.com:org/meta.git
    requires: standards-kb

  standards-kb (v1.5.0) [transitive via meta-kb]
    source: https://github.com/org/standards.git
    requires: templates-kb
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

Validate graft.yaml and graft.lock for correctness.

```bash
graft validate [MODE]
```

**Modes:**
- `all` (default): Run all validations
- `config`: Validate graft.yaml structure and schema only
- `lock`: Validate graft.lock file consistency only
- `integrity`: Verify .graft/ directories match lock file commits

**Examples:**
```bash
graft validate              # Validate everything
graft validate config       # Check graft.yaml only
graft validate lock         # Check graft.lock only
graft validate integrity    # Verify .graft/ commits match lock file
```

**Exit codes:**
- 0: Success
- 1: Validation error (invalid configuration)
- 2: Integrity mismatch (lock file vs .graft/ directory)

**Config mode checks:**
- YAML schema validity
- Required fields present
- Dependency references exist
- No circular dependencies

**Lock mode checks:**
- Lock file schema validity
- All refs exist in repositories
- Commit hashes valid

**Integrity mode checks:**
- .graft/ directories exist
- Actual commits match lock file entries
- No manual modifications to dependencies

---

## Global Options

All commands support:
- `--help`: show help
- `--json`: JSON output
- `--verbose`: detailed logging

---

## Examples

**Initial setup:**
```bash
graft resolve
graft apply my-kb --to main
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

**Execute dependency command:**
```bash
graft meta-kb:build
```

See [User Guide](guides/user-guide.md) for workflows.
