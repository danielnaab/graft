---
status: stable
updated: 2026-01-05
---

# Configuration Guide

Reference for graft.yaml and graft.lock formats.

**Canonical Specifications:**
- [graft.yaml Format](../../graft-knowledge/docs/specification/graft-yaml-format.md)
- [Lock File Format](../../graft-knowledge/docs/specification/lock-file-format.md)

**Implementation:**
- Parser: `src/graft/services/config_service.py`
- Lock adapter: `src/graft/adapters/lock_file.py`
- Models: `src/graft/domain/graft_config.py`, `src/graft/domain/lock_entry.py`

> **Note:** This interprets canonical specifications. When in doubt, refer to specifications above.

---

## Files Overview

**graft.yaml** - user-edited configuration:
- Declares dependencies
- Defines changes (for KB maintainers)
- Defines reusable commands

**graft.lock** - auto-generated lock file:
- Records consumed versions
- Includes commit hashes for reproducibility
- Updated only on successful operations

Commit both files to version control.

---

## graft.yaml Format

**Minimal:**
```yaml
apiVersion: graft/v0
deps:
  my-kb: "https://github.com/user/kb.git#main"
```

**Complete:**
```yaml
apiVersion: graft/v0

metadata:
  description: "Project knowledge dependencies"
  version: "1.0.0"

deps:
  meta-kb: "https://github.com/org/meta#v2.0.0"
  utils: "ssh://git@server/repo.git#main"
  local: "file:///path/to/repo#develop"

changes:
  v2.0.0:
    type: breaking
    description: "Major restructuring"
    migration: migrate-v2
    verify: verify-v2

commands:
  migrate-v2:
    run: "./scripts/migrate.sh"
    working_dir: "."
    env:
      MODE: "production"
```

## Field Reference

**apiVersion** (required): `graft/v0`

**metadata** (optional):
- `description`: string
- `version`: string
- `author`: string

**deps** (required): map of dependency name to git URL
- Format: `<url>#<ref>`
- Protocols: https, ssh, file
- Ref: branch, tag, or commit

**changes** (optional): map of ref to change definition
- `type`: breaking, feature, fix
- `description`: string
- `migration`: command name (optional)
- `verify`: command name (optional)
- `metadata`: arbitrary key-value (optional)

**commands** (optional): map of command name to definition
- `run`: shell command (required)
- `description`: string (optional)
- `working_dir`: path (optional, default: ".")
- `env`: environment variables (optional)

---

## graft.lock Format

Auto-generated. Do not edit manually.

**Format (v2):**
```yaml
apiVersion: graft/v0
dependencies:
  # Direct dependency
  meta-kb:
    source: "https://github.com/org/meta.git"
    ref: "v2.0.0"
    commit: "abc123def456789012345678901234567890abcd"
    consumed_at: "2026-01-05T10:30:00Z"
    direct: true
    requires: ["standards-kb"]
    required_by: []

  # Transitive dependency (pulled in by meta-kb)
  standards-kb:
    source: "https://github.com/org/standards.git"
    ref: "v1.5.0"
    commit: "def456abc123789012345678901234567890abcd"
    consumed_at: "2026-01-05T10:30:00Z"
    direct: false
    requires: []
    required_by: ["meta-kb"]
```

**Fields:**
- `source`: git URL
- `ref`: consumed reference (tag, branch, or commit)
- `commit`: full 40-character commit hash (SHA-1)
- `consumed_at`: ISO 8601 timestamp
- `direct`: boolean - true for direct dependencies, false for transitive
- `requires`: list of dependency names this dep needs
- `required_by`: list of dependency names that require this dep

The lock file tracks ALL dependencies (direct + transitive) with their
complete dependency graph relationships.

---

## Examples

**Multiple dependencies:**
```yaml
deps:
  meta-kb: "https://github.com/org/meta#v1.0"
  docs: "https://github.com/org/docs#main"
  local: "file:///home/user/kb#develop"
```

**Changes with migrations:**
```yaml
changes:
  v2.0.0:
    type: breaking
    migration: migrate-v2
    verify: verify-v2

commands:
  migrate-v2:
    run: "python scripts/migrate.py"
  verify-v2:
    run: "pytest tests/v2/"
```

**Custom commands:**
```yaml
commands:
  build:
    run: "npm run build"
    working_dir: "docs"

  test:
    run: "pytest"
    env:
      PYTHONPATH: "src"
```

See [User Guide](guides/user-guide.md) for usage examples.
See specifications for complete validation rules.
