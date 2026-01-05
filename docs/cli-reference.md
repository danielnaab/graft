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
| resolve | Clone/fetch dependencies |
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

Clone or fetch all dependencies from `graft.yaml`.

```bash
graft resolve
```

Clones to `.graft/deps/<name>/`. Fetches if already cloned. Does not modify lock file.

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

Validate `graft.yaml` configuration.

```bash
graft validate
```

Checks:
- YAML syntax
- Required fields
- Dependency URL format
- Command references

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
