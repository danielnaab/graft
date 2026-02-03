---
status: stable
updated: 2026-01-05
---

# Graft User Guide

Practical guide for using Graft semantic dependency management.

> **Authority Note:** Interprets canonical specifications from [graft-knowledge](../../../graft-knowledge/). When in conflict, specifications are authoritative.

**Specifications:**
- [Change Model](../../../graft-knowledge/docs/specification/change-model.md)
- [graft.yaml Format](../../../graft-knowledge/docs/specification/graft-yaml-format.md)
- [Lock File Format](../../../graft-knowledge/docs/specification/lock-file-format.md)
- [Core Operations](../../../graft-knowledge/docs/specification/core-operations.md)

---

## Quick Start

Install:
```bash
git clone <graft-repository-url>
cd graft
uv sync
uv run python -m graft --help
```

Create project:
```bash
mkdir my-project && cd my-project
cat > graft.yaml <<EOF
apiVersion: graft/v0
deps:
  my-kb: "https://github.com/user/kb.git#main"
EOF
```

Initialize:
```bash
uv run python -m graft resolve
uv run python -m graft apply my-kb --to main
```

## Core Concepts

**Dependencies** - Git repositories tracked by graft:
- Declared in `graft.yaml`
- Pinned in `graft.lock` with commit hash
- Stored in `.graft/` as git submodules (preferred) or clones

**Changes** - Semantic evolution markers:
- Types: breaking, feature, fix
- Optional migration and verify commands
- Declared in dependency's `graft.yaml`

**Lock file** - Reproducible state:
- Tracks consumed version per dependency
- Includes commit hash for integrity
- Updated only on successful operations

**Atomic upgrades** - All-or-nothing:
- Snapshot before upgrade
- Run migrations in sequence
- Rollback on failure

## Common Workflows

**Add dependency:**
```bash
# Edit graft.yaml, add to deps section
uv run python -m graft resolve
uv run python -m graft apply <dep> --to <ref>
```

**Check for updates:**
```bash
uv run python -m graft fetch <dep>
uv run python -m graft changes <dep>
```

**Upgrade dependency:**
```bash
uv run python -m graft upgrade <dep> --to <ref>
# Runs migrations, updates lock on success
```

**Execute dependency command:**
```bash
uv run python -m graft <dep>:<command>
```

**View current state:**
```bash
uv run python -m graft status
uv run python -m graft show <dep> <ref>
```

## Troubleshooting

**Upgrade fails:**
- Check `.graft/snapshots/` for rollback point
- Review migration output in error message
- Run migrations manually to debug

**Lock file conflicts:**
- Resolve conflicts in `graft.lock`
- Verify consumed versions with `graft status`
- Reapply if needed: `graft apply <dep> --to <ref>`

**Dependency not found:**
- Check URL in `graft.yaml`
- Verify network access / SSH keys
- Try `graft resolve --verbose`

## Best Practices

**Version control:**
- Commit both `graft.yaml` and `graft.lock`
- Never edit `graft.lock` manually
- Use semantic refs in deps (tags/branches)

**Changes declaration:**
- Declare all breaking changes
- Include migration commands
- Test migrations before releasing

**Upgrade strategy:**
- Review changes before upgrading: `graft changes <dep>`
- Test in development environment first
- Use `--dry-run` to preview (when available)

## Git Submodules

Graft uses git submodules to manage dependencies when the deps directory
(`.graft/` by default) is inside your git repository. This provides:

- **Easy setup**: New clones can use `git clone --recursive` to get all dependencies
- **Reproducible state**: Submodule commits are tracked in the parent repo
- **Standard git workflow**: Works with existing git tools and services

**Starting fresh (if you have legacy clones):**
```bash
# Remove existing dependencies
rm -rf .graft/

# Re-resolve as submodules
uv run python -m graft resolve

# Stage the changes
git add .gitmodules .graft/
git commit -m "Add dependencies as submodules"
```

**Clone a project with dependencies:**
```bash
# Option 1: Clone with submodules
git clone --recursive <project-url>

# Option 2: Clone then init submodules
git clone <project-url>
git submodule update --init
```

## Advanced

**Custom commands** in `graft.yaml`:
```yaml
commands:
  build:
    run: "npm run build"
    working_dir: "."
```

**Change metadata** for AI assistance:
```yaml
changes:
  v2.0.0:
    type: breaking
    migration: migrate-v2
    metadata:
      ai_summary: "Restructured content organization"
```

**Multiple dependencies:**
```yaml
deps:
  meta-kb: "https://github.com/org/meta#v1.0"
  utils: "https://github.com/org/utils#main"
```

See [CLI Reference](../cli-reference.md) for complete command documentation.
See [Configuration Guide](../configuration.md) for format details.
