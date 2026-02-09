---
title: "graft.yaml Format Specification"
date: 2026-01-01
status: draft
---

# graft.yaml Format Specification

## Overview

The `graft.yaml` file is the configuration file for Graft dependencies. It defines:
- Dependency metadata
- Changes (identified by git refs)
- Commands (migrations, verification, utilities)
- Dependencies on other Graft modules

This file lives in the root of a dependency repository and is the **source of truth for automation**.

## File Location

```
repository-root/
  graft.yaml          ← This file
  CHANGELOG.md        ← Optional human-readable changelog
  README.md
  src/
  codemods/
```

## Schema

### Top-Level Structure

```yaml
# Optional metadata
metadata:
  name: string                    # Dependency name
  description: string             # Brief description
  version: string                 # Current version (optional)
  changelog: string               # Path to CHANGELOG.md (default: "CHANGELOG.md")

# Change definitions (see Change Model spec)
changes:
  <git-ref>:
    type: string                  # Optional: "breaking", "feature", "fix", etc.
    description: string           # Optional: brief summary
    migration: string             # Optional: command name
    verify: string                # Optional: command name
    [custom-key]: any             # Optional: extensible metadata

# Command definitions
commands:
  <command-name>:
    run: string                   # Required: command to execute
    description: string           # Optional: human-readable description
    working_dir: string           # Optional: working directory (default: consumer root)
    env: object                   # Optional: environment variables

# Dependencies (for Graft-aware dependencies)
dependencies:
  <dep-name>:
    source: string                # Required: git URL or path
    ref: string                   # Optional: specific ref (default: main)
```

## Section: metadata

Optional metadata about this dependency.

### Fields

#### name (optional)
**Type**: `string`

**Description**: Human-readable name of the dependency.

**Example**:
```yaml
metadata:
  name: "meta-knowledge-base"
```

#### description (optional)
**Type**: `string`

**Description**: Brief description of what this dependency provides.

**Example**:
```yaml
metadata:
  description: "Shared knowledge base for meta-cognitive patterns"
```

#### version (optional)
**Type**: `string`

**Description**: Current version. Informational only; actual version is determined by git refs.

**Example**:
```yaml
metadata:
  version: "2.0.0"
```

#### changelog (optional)
**Type**: `string`

**Description**: Path to human-readable changelog file (relative to repository root).

**Default**: `"CHANGELOG.md"`

**Example**:
```yaml
metadata:
  changelog: "CHANGELOG.md"
  changelog: "docs/RELEASES.md"
```

## Section: changes

Defines changes identified by git refs. See [Change Model Specification](./change-model.md) for detailed field definitions.

### Structure

```yaml
changes:
  <git-ref>:           # Key is the git ref (commit, tag, branch)
    type: string       # Optional
    description: string  # Optional
    migration: string  # Optional: command name
    verify: string     # Optional: command name
    [custom]: any      # Optional: extensible
```

### Example

```yaml
changes:
  v2.0.0:
    type: breaking
    description: "Renamed getUserData → fetchUserData"
    migration: migrate-v2
    verify: verify-v2

  v1.5.0:
    type: feature
    description: "Added caching support"
    # No migration needed

  abc123:
    type: fix
    migration: fix-abc
```

### Ordering

Changes are applied in **declaration order**. First change in the file is applied first.

**Important**: When upgrading from v1.0.0 to v3.0.0, list intermediate versions in order:

```yaml
changes:
  v1.0.0:
    migration: migrate-v1
  v2.0.0:
    migration: migrate-v2
  v3.0.0:
    migration: migrate-v3
```

## Section: commands

Defines executable commands that can be invoked by consumers or referenced by changes.

**IMPORTANT:** All commands, especially migrations, MUST be self-contained. See [Migration Self-Containment](#migration-self-containment) below.

### Structure

```yaml
commands:
  <command-name>:          # Key is the command name
    run: string            # Required: shell command to execute
    description: string    # Optional: human-readable description
    working_dir: string    # Optional: working directory
    env:                   # Optional: environment variables
      KEY: value
```

### Fields

#### run (required)
**Type**: `string`

**Description**: Shell command to execute. Runs in consumer's context.

**Interpolation**: May use variables:
- `${CONSUMER_ROOT}`: Consumer's repository root
- `${DEP_ROOT}`: This dependency's root (if installed)

**Examples**:
```yaml
run: "npx jscodeshift -t codemods/v2.js src/"
run: "python migrations/migrate.py"
run: "./scripts/migrate.sh"
run: |
  npm test
  ./verify.sh
```

#### description (optional)
**Type**: `string`

**Description**: Human-readable description of what this command does.

**Example**:
```yaml
description: "Rename getUserData to fetchUserData"
```

#### working_dir (optional)
**Type**: `string`

**Description**: Working directory for command execution. Relative to consumer root.

**Default**: Consumer's repository root

**Example**:
```yaml
working_dir: "src/"
```

#### env (optional)
**Type**: `object` (key-value pairs)

**Description**: Environment variables to set during command execution.

**Example**:
```yaml
env:
  NODE_ENV: "production"
  MIGRATION_DRY_RUN: "false"
```

### Command Examples

#### Simple Migration

```yaml
commands:
  migrate-v2:
    run: "npx jscodeshift -t codemods/v2.js src/"
    description: "Rename getUserData → fetchUserData"
```

#### Multi-Step Migration

```yaml
commands:
  migrate-v3:
    run: |
      echo "Running migration v3..."
      ./scripts/step1.sh
      npx jscodeshift -t codemods/step2.js src/
      python scripts/step3.py
    description: "Multi-step migration for v3"
```

#### Migration with Verification

```yaml
commands:
  migrate-v2:
    run: "npx jscodeshift -t codemods/v2.js src/"

  verify-v2:
    run: |
      npm test
      ! grep -r 'getUserData' src/
    description: "Verify v2 migration: tests pass and no old API usage"
```

#### Conditional Migration

```yaml
commands:
  migrate-optional:
    run: |
      if [ -f "src/legacy.js" ]; then
        ./migrate-legacy.sh
      fi
    description: "Migrate legacy code if it exists"
```

---

## Migration Self-Containment

### The Constraint

**All migration commands MUST be self-contained.** They cannot reference files from transitive dependencies (dependencies of your dependencies).

This is a fundamental requirement of the flat-only dependency model introduced in v3.

### Why Self-Containment?

With flat-only dependencies:
- Consumers only clone dependencies they explicitly declare
- Your graft's dependencies are YOUR implementation details
- Consumers don't have access to your dependencies' files

If your migration needs content from another graft, you have two options:
1. **Bundle it** - Copy needed files into your graft at publish time
2. **Document it** - Tell consumers to add that graft as their own dependency

### Invalid Migration Example

```yaml
commands:
  migrate-v2:
    # ❌ BAD - references transitive dependency
    run: |
      cp ${DEP_ROOT}/../standards-kb/template.md ./
      cp ${DEP_ROOT}/../standards-kb/config.yaml ./config/
```

**Problem:** Consumer may not have `standards-kb` installed. It's YOUR dependency, not theirs.

### Valid Migration Examples

**Option 1: Bundle what you need**

```yaml
commands:
  migrate-v2:
    # ✅ GOOD - uses bundled content
    run: |
      cp ${DEP_ROOT}/bundled/template.md ./
      cp ${DEP_ROOT}/bundled/config.yaml ./config/
```

```
my-graft/
  bundled/
    template.md       # Copied from standards-kb at publish time
    config.yaml
  commands/
  graft.yaml
```

**Option 2: Document required dependencies**

```yaml
# graft.yaml
metadata:
  name: "web-app-template"
  description: "Web app scaffolding - works with coding-standards"

commands:
  init:
    # References consumer's own dependencies
    run: |
      # Generate structure
      mkdir -p src/ test/
      # If consumer has coding-standards, use it
      if [ -d ../.graft/coding-standards ]; then
        cp ../.graft/coding-standards/.eslintrc ./
      fi
```

```markdown
# README.md

## Installation

Add both this graft and coding-standards:

​```yaml
deps:
  web-app-template: "git@github.com:org/web-app.git#v2.0.0"
  coding-standards: "git@github.com:org/standards.git#v1.5.0"
​```
```

### Bundling Strategy

If your graft depends on content from other grafts, bundle at **publish time**:

```bash
# Before tagging a release
./scripts/bundle-deps.sh

# Copies needed files from dependencies into bundled/
cp -r .graft/standards-kb/templates/ bundled/standards-templates/
cp -r .graft/config-lib/configs/ bundled/configs/

# Commit bundled content
git add bundled/
git commit -m "Bundle dependencies for v2.0.0"
git tag v2.0.0
```

This way, consumers get a self-contained graft.

### Variables Available

Your commands run in the **consumer's context**. These variables are available:

- `${CONSUMER_ROOT}` - Consumer's repository root
- `${DEP_ROOT}` - Your graft's root (in consumer's `.graft/<your-name>/`)

**Do NOT use:**
- `${DEP_ROOT}/../other-dep/` - Consumer may not have `other-dep`

**Safe patterns:**
```bash
# Use content within your graft
${DEP_ROOT}/scripts/migrate.sh
${DEP_ROOT}/bundled/template.md

# Write to consumer's repo
${CONSUMER_ROOT}/src/generated.ts

# Check for optional dependencies (consumer's choice)
if [ -d "${CONSUMER_ROOT}/.graft/optional-dep" ]; then
  # Use it
fi
```

---

## Section: dependencies

Declares dependencies on other Graft-enabled modules (optional).

**Note:** In the flat-only model (v3), these are YOUR graft's dependencies. Consumers won't automatically get them. If consumers need these dependencies, document that in your README.

### Structure

```yaml
dependencies:
  <dep-name>:
    source: string      # Required: git URL or path
    ref: string         # Optional: specific ref (default: main/master)
```

### Fields

#### source (required)
**Type**: `string`

**Description**: Git URL or local path to dependency repository.

**Formats**:
- SSH: `git@github.com:user/repo.git`
- HTTPS: `https://github.com/user/repo.git`
- Local: `../local-repo`

**Example**:
```yaml
source: "git@github.com:org/meta-kb.git"
```

#### ref (optional)
**Type**: `string`

**Description**: Specific git ref to use. If not specified, uses default branch.

**Example**:
```yaml
ref: "v1.5.0"
ref: "stable"
```

### Example

```yaml
dependencies:
  meta-knowledge-base:
    source: "git@github.com:org/meta-kb.git"
    ref: "v1.5.0"

  shared-utils:
    source: "../shared-utils"
```

## Complete Example

```yaml
# graft.yaml - Complete example

metadata:
  name: "example-library"
  description: "Example library showing Graft integration"
  changelog: "CHANGELOG.md"

changes:
  v2.0.0:
    type: breaking
    description: "Renamed getUserData → fetchUserData"
    migration: migrate-v2
    verify: verify-v2
    jira_ticket: "LIB-123"

  v1.5.0:
    type: feature
    description: "Added caching support"
    # No migration needed

  v1.0.0:
    type: feature
    description: "Initial release"

commands:
  migrate-v2:
    run: "npx jscodeshift -t codemods/rename-getUserData.js src/"
    description: "Rename getUserData → fetchUserData"
    env:
      JSCODESHIFT_PARSER: "tsx"

  verify-v2:
    run: |
      npm test
      ! grep -r 'getUserData' src/
    description: "Verify v2 migration completed"

  changelog:
    run: "cat CHANGELOG.md"
    description: "Display changelog"

dependencies:
  meta-knowledge-base:
    source: "git@github.com:org/meta-kb.git"
    ref: "v1.5.0"
```

## Validation

### Schema Validation

```python
def validate_graft_yaml(config: dict) -> list[str]:
    """Validate graft.yaml structure. Returns list of errors."""
    errors = []

    # Validate changes section
    if 'changes' in config:
        if not isinstance(config['changes'], dict):
            errors.append("'changes' must be an object")
        else:
            for ref, change_data in config['changes'].items():
                # Validate migration references
                if 'migration' in change_data:
                    cmd = change_data['migration']
                    if 'commands' not in config or cmd not in config['commands']:
                        errors.append(f"Change '{ref}': migration '{cmd}' not found in commands")

                # Validate verify references
                if 'verify' in change_data:
                    cmd = change_data['verify']
                    if 'commands' not in config or cmd not in config['commands']:
                        errors.append(f"Change '{ref}': verify '{cmd}' not found in commands")

    # Validate commands section
    if 'commands' in config:
        if not isinstance(config['commands'], dict):
            errors.append("'commands' must be an object")
        else:
            for cmd_name, cmd_data in config['commands'].items():
                if 'run' not in cmd_data:
                    errors.append(f"Command '{cmd_name}': missing required 'run' field")

    # Validate dependencies section
    if 'dependencies' in config:
        if not isinstance(config['dependencies'], dict):
            errors.append("'dependencies' must be an object")
        else:
            for dep_name, dep_data in config['dependencies'].items():
                if 'source' not in dep_data:
                    errors.append(f"Dependency '{dep_name}': missing required 'source' field")

    return errors
```

### Git Ref Validation

```python
def validate_refs_exist(config: dict, repo_path: str) -> list[str]:
    """Validate that all refs in changes exist in git."""
    errors = []
    refs = set(config.get('changes', {}).keys())

    # Get all refs from git
    result = subprocess.run(
        ['git', 'show-ref'],
        cwd=repo_path,
        capture_output=True,
        text=True
    )

    available_refs = set()
    for line in result.stdout.splitlines():
        ref_name = line.split()[1]
        available_refs.add(ref_name.split('/')[-1])  # Get short name

    # Also get commit hashes
    result = subprocess.run(
        ['git', 'log', '--format=%H %h'],
        cwd=repo_path,
        capture_output=True,
        text=True
    )
    for line in result.stdout.splitlines():
        full_hash, short_hash = line.split()
        available_refs.add(full_hash)
        available_refs.add(short_hash)

    # Check each ref
    for ref in refs:
        if ref not in available_refs:
            errors.append(f"Ref '{ref}' does not exist in git repository")

    return errors
```

## CLI Validation

```bash
# Validate graft.yaml
$ graft validate

Validating graft.yaml...
✓ Schema is valid
✓ All migration commands exist
✓ All verify commands exist
✓ All refs exist in git repository
✓ All dependency sources are accessible

# Validate specific aspects
$ graft validate --schema-only
$ graft validate --refs-only
```

## Versioning

The graft.yaml format itself may evolve. Version can be specified:

```yaml
graft_version: "1.0"  # Optional: graft.yaml format version

metadata:
  name: "example"
```

If not specified, latest version is assumed.

## Related

- [Specification: Change Model](./change-model.md)
- [Specification: Lock File Format](./lock-file-format.md)
- [Specification: Core Operations](./core-operations.md)
- [Decision 0003: Explicit Change Declarations](../decisions/decision-0003-explicit-change-declarations.md)

## References

- YAML Specification: https://yaml.org/spec/
- Git refs: https://git-scm.com/book/en/v2/Git-Internals-Git-References
