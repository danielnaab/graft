# Configuration Guide

Complete reference for graft.yaml and graft.lock file formats.

---

## Overview

Graft uses two configuration files:

- **graft.yaml** - User-edited configuration declaring dependencies, changes, and commands
- **graft.lock** - Auto-generated lock file recording exact consumed versions

**Important**: Commit both files to version control for reproducible builds.

---

## graft.yaml Format

The `graft.yaml` file declares your dependencies and how to manage them.

### Minimal Example

```yaml
apiVersion: graft/v0
deps:
  my-knowledge: "https://github.com/user/knowledge.git#main"
```

### Complete Example

```yaml
apiVersion: graft/v0

# Optional metadata
metadata:
  description: "My project's knowledge dependencies"
  version: "1.0.0"
  author: "Your Name"

# Required: Dependency declarations
deps:
  meta-kb: "https://github.com/org/meta-kb.git#v2.0.0"
  shared-utils: "ssh://git@server/repo.git#main"
  local-kb: "file:///path/to/local/repo#develop"

# Optional: Change declarations (for knowledge base maintainers)
changes:
  v1.0.0:
    type: feature
    description: "Initial release"

  v1.5.0:
    type: feature
    description: "Additional examples and templates"

  v2.0.0:
    type: breaking
    description: "Major restructuring of content organization"
    migration: migrate-v2
    verify: verify-v2

# Optional: Reusable commands
commands:
  migrate-v2:
    run: "./scripts/migrate-to-v2.sh"
    description: "Migrate to v2 structure"
    working_dir: "."
    env:
      MIGRATION_MODE: "production"

  verify-v2:
    run: "npm test && ./scripts/verify-structure.sh"
    description: "Verify v2 migration succeeded"
```

---

## graft.yaml Field Reference

### apiVersion (required)

```yaml
apiVersion: graft/v0
```

Specifies the configuration format version. Currently only `graft/v0` is supported.

### metadata (optional)

```yaml
metadata:
  description: "Project description"
  version: "1.0.0"
  author: "Your Name"
  custom_field: "any value"
```

Arbitrary metadata about the configuration. Graft does not interpret these fields but preserves them.

### deps (required)

```yaml
deps:
  dep-name: "url#ref"
```

Dependency declarations in the format: `"<git-url>#<ref>"`

**Supported URL schemes**:
- `https://` - HTTPS git repositories
- `ssh://` - SSH git repositories (e.g., `ssh://git@github.com/user/repo.git`)
- `git@` - SSH shorthand (e.g., `git@github.com:user/repo.git`)
- `file://` - Local git repositories

**Ref format**:
- Branch name: `main`, `develop`
- Tag: `v1.0.0`, `release-2024`
- Commit hash: Full 40-character SHA-1

**Examples**:
```yaml
deps:
  # Track main branch
  my-kb: "https://github.com/org/kb.git#main"

  # Track specific tag
  meta-kb: "https://github.com/org/meta.git#v2.0.0"

  # SSH access
  private-kb: "git@github.com:org/private.git#main"

  # Local repository
  local-kb: "file:///home/user/repos/kb#develop"
```

**Validation**:
- At least one dependency required
- Dependency names must be unique
- URLs must be valid git repository URLs
- Refs must exist in the repository (validated by `graft validate --refs`)

### changes (optional)

```yaml
changes:
  ref:
    type: breaking | feature | fix
    description: "Human-readable description"
    migration: command-name  # optional
    verify: command-name      # optional
```

Declares semantic versioned changes. Used by knowledge base maintainers to communicate changes to consumers.

**Fields**:
- `type` - Change type (breaking, feature, or fix)
- `description` - Human-readable explanation of the change
- `migration` - Name of command to run during upgrade (must exist in `commands`)
- `verify` - Name of command to verify upgrade success (must exist in `commands`)

**Example**:
```yaml
changes:
  v2.0.0:
    type: breaking
    description: |
      Renamed getUserData → fetchUserData

      This change requires updating all call sites to use the new function name.
    migration: migrate-v2
    verify: verify-v2
```

**Semantic versioning guidelines**:
- `breaking` - Incompatible changes requiring consumer action
- `feature` - New functionality, backward compatible
- `fix` - Bug fixes, backward compatible

### commands (optional)

```yaml
commands:
  command-name:
    run: "shell command"
    description: "Human-readable description"
    working_dir: "."  # optional
    env:              # optional
      KEY: "value"
```

Reusable shell commands that can be executed by consumers or referenced in changes.

**Fields**:
- `run` (required) - Shell command to execute
- `description` (optional) - Human-readable explanation
- `working_dir` (optional) - Working directory (default: current directory)
- `env` (optional) - Environment variables as key-value pairs

**Example**:
```yaml
commands:
  test:
    run: "pytest tests/"
    description: "Run test suite"

  build:
    run: "npm run build"
    description: "Build production assets"
    working_dir: "./frontend"
    env:
      NODE_ENV: "production"

  migrate-v2:
    run: |
      npx jscodeshift -t codemods/v2.js src/
      npm test
    description: "Migrate to v2 API"
```

**Execution context**:
- Commands execute in the consumer's repository (not in `.graft/deps/`)
- Working directory defaults to current directory
- Environment variables are merged with system environment

**Validation**:
- Command names must be unique
- Command names referenced in `changes` must exist

---

## graft.lock Format

The `graft.lock` file is auto-generated and should not be edited manually.

### Example

```yaml
version: 1
dependencies:
  my-knowledge:
    source: "https://github.com/user/knowledge.git"
    ref: "v2.0.0"
    commit: "abc123def456789012345678901234567890abcd"
    consumed_at: "2026-01-04T10:30:00+00:00"

  shared-utils:
    source: "ssh://git@server/repo.git"
    ref: "main"
    commit: "def456789012345678901234567890abcdef1234"
    consumed_at: "2026-01-03T14:20:00+00:00"
```

### Field Reference

**version** - Lock file format version (currently always `1`)

**dependencies** - Map of dependency name to lock entry

**Lock entry fields**:
- `source` - Git repository URL (canonical form)
- `ref` - Git ref that was resolved (branch, tag, or commit)
- `commit` - Full 40-character commit SHA-1
- `consumed_at` - ISO 8601 timestamp of when this version was consumed

### Purpose

The lock file ensures:
- **Reproducible builds** - Same commit hash across all environments
- **Audit trail** - Track when versions were consumed
- **Update detection** - Compare lock file to current state

### Commit to Version Control

**Always commit graft.lock to version control**. This ensures:
- All team members use the same dependency versions
- CI/CD builds are reproducible
- Dependency history is tracked

---

## File Locations

```
project/
├── graft.yaml          # User-edited configuration
├── graft.lock          # Auto-generated lock file
└── .graft/
    ├── deps/           # Cloned dependencies
    │   ├── my-knowledge/
    │   └── shared-utils/
    └── snapshots/      # Upgrade snapshots (temporary)
```

---

## Validation

Validate your configuration with:

```bash
# Validate everything
uv run python -m graft validate

# Validate only graft.yaml
uv run python -m graft validate --schema

# Validate only git refs
uv run python -m graft validate --refs

# Validate only lock file
uv run python -m graft validate --lock
```

See [CLI Reference](CLI_REFERENCE.md#graft-validate) for details.

---

## Best Practices

### For Consumers

1. **Pin to semantic versions** instead of branches:
   ```yaml
   # Good - stable, semantic version
   deps:
     my-kb: "https://github.com/org/kb.git#v1.0.0"

   # Less good - moving target
   deps:
     my-kb: "https://github.com/org/kb.git#main"
   ```

2. **Commit both graft.yaml and graft.lock** to version control

3. **Validate before committing**:
   ```bash
   uv run python -m graft validate
   ```

4. **Use --dry-run before upgrading**:
   ```bash
   uv run python -m graft upgrade my-kb --to v2.0.0 --dry-run
   ```

### For Knowledge Base Maintainers

1. **Document all breaking changes** with migrations:
   ```yaml
   changes:
     v2.0.0:
       type: breaking
       description: "Clear explanation of what changed and why"
       migration: migrate-v2
       verify: verify-v2
   ```

2. **Provide migration commands** for breaking changes

3. **Use semantic versioning consistently**:
   - Breaking changes → major version bump (v1.0.0 → v2.0.0)
   - New features → minor version bump (v1.0.0 → v1.1.0)
   - Bug fixes → patch version bump (v1.0.0 → v1.0.1)

4. **Test migrations** before releasing

5. **Keep descriptions clear and actionable**

---

## Examples

### Example 1: Simple Knowledge Base Consumer

```yaml
apiVersion: graft/v0
deps:
  meta-kb: "https://github.com/org/meta-kb.git#v2.0.0"
```

### Example 2: Knowledge Base with Changes

```yaml
apiVersion: graft/v0

deps:
  # Consumers depend on this repository
  external-consumer: "https://github.com/consumer/project.git#main"

changes:
  v1.0.0:
    type: feature
    description: "Initial release with core templates"

  v2.0.0:
    type: breaking
    description: "Restructured directory layout from flat to hierarchical"
    migration: migrate-v2
    verify: verify-v2

commands:
  migrate-v2:
    run: "./scripts/restructure-files.sh"
    description: "Migrate files to new directory structure"

  verify-v2:
    run: "./scripts/verify-structure.sh"
    description: "Verify new structure is correct"
```

### Example 3: Project with Multiple Dependencies

```yaml
apiVersion: graft/v0

metadata:
  description: "Documentation site with multiple knowledge sources"

deps:
  meta-kb: "https://github.com/org/meta-kb.git#v2.0.0"
  templates: "https://github.com/org/templates.git#v1.5.0"
  policies: "ssh://git@internal.server/policies.git#main"

commands:
  build:
    run: "hugo --minify"
    description: "Build static site"

  deploy:
    run: "./scripts/deploy.sh"
    description: "Deploy to production"
    env:
      DEPLOY_ENV: "production"
```

---

See [User Guide](guides/USER_GUIDE.md) for workflow examples and [CLI Reference](CLI_REFERENCE.md) for command usage.
