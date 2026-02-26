---
title: "graft.yaml Format Specification"
date: 2026-02-25
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
    category: string              # Optional: role classification (core|diagnostic|optional|advanced)
    example: string               # Optional: complete invocation example
    working_dir: string           # Optional: working directory (default: consumer root)
    env: object                   # Optional: environment variables
    stdin: string | object        # Optional: text piped to stdin (literal or template)
    context: list[string]         # Optional: state query names to resolve before running

# State query definitions (see State Queries spec)
state:
  <query-name>:
    run: string                   # Required: command outputting JSON
    cache:                        # Optional: cache configuration
      deterministic: bool         # Default: true
    timeout: integer              # Optional: seconds (default: 300)

# Sequence definitions (multi-step command pipelines)
sequences:
  <sequence-name>:
    description: string           # Optional: human-readable summary
    category: string              # Optional: role classification (core|diagnostic|optional|advanced)
    example: string               # Optional: complete invocation example
    steps:                        # Required: ordered command names or step objects
      - <command-name>            #   String form: no per-step timeout or condition
      - name: <command-name>      #   Object form: supports timeout and when condition
        timeout: integer          #   Optional: timeout in seconds for this step
        when:                     #   Optional: condition for executing this step
          state: string           #     Required: run-state file name (e.g. "verify")
          field: string           #     Required: JSON field name (e.g. "lint")
          # Exactly one operator must be specified:
          equals: string          #   Execute only when field equals this value
          not_equals: string      #   Execute only when field does not equal this value
          starts_with: string     #   Execute only when field starts with this prefix
          not_starts_with: string #   Execute only when field does not start with this prefix
        # When condition is false (or state file/field is absent): step is skipped
    args: list[ArgDef]            # Optional: argument declarations (same schema as commands)
    on_step_fail:                 # Optional: retry on named step failure
      step: string                # Required: step that triggers recovery
      recovery: string            # Required: recovery command name
      max: integer                # Required: max retry iterations
    checkpoint: bool              # Optional: write checkpoint.json on success (default: false)

# graft catalog — Command metadata subcommand
#
# graft catalog <dep>               Lists all commands and sequences with metadata
# graft catalog <dep>:<name>        Prints full metadata for a single command or sequence
# graft catalog <dep> --json        Full catalog as machine-readable JSON
# graft catalog <dep>:<name> --json Single entry as JSON
#
# category values:
#   core        Primary workflow steps (implement, verify, approve)
#   diagnostic  Run when something is wrong (diagnose, resume)
#   optional    Enrichment steps (spec-check, review)
#   advanced    Power-user tools (implement-parallel)

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
    stdin: string | object # Optional: text piped to stdin
    context:               # Optional: state query names
      - string
```

**Command Name Constraints**:
- Command names MUST NOT contain `:` (colon character)
- Rationale: Colon is reserved as separator for dependency command syntax (`graft run dep:cmd`)
- Recommended naming: Use kebab-case (`test-unit`), snake_case (`test_unit`), or camelCase (`testUnit`)
- Invalid examples: `test:unit`, `build:prod`, `db:migrate`
- Valid examples: `test-unit`, `build-prod`, `db-migrate`

**Validation Error Example**:
```
Error: Invalid command name in graft.yaml
  Line 15: Command 'test:unit'
  Reason: Command names cannot contain ':' (reserved separator)
  Suggestion: Rename to 'test-unit' or 'test_unit'
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

#### stdin (optional)
**Type**: `string | object`

**Description**: Text to pipe to the command's stdin. Supports three forms:

1. **Literal string** — piped as-is, no template evaluation.
2. **Template object** — `{ template: "<path>" }` — rendered with the default template engine (tera).
3. **Template with engine override** — `{ template: "<path>", engine: "tera" | "none" }`.

**Default**: None (stdin is not connected).

**Constraints**:
- Literal text must not be empty.
- Template path must be relative (no leading `/`).
- Template path must not be empty.
- Engine must be `tera` or `none`.

**Examples**:
```yaml
# Literal string
stdin: "Hello, world!"

# Template file (default engine: tera)
stdin:
  template: "templates/prompt.md"

# Template file with explicit engine
stdin:
  template: "templates/report.md"
  engine: tera

# Raw template (no rendering)
stdin:
  template: "templates/raw.txt"
  engine: none
```

#### context (optional)
**Type**: `list[string]`

**Description**: State query names to resolve before running the command. Each entry must correspond to a key in the `state:` section.

Resolved state values are exposed to the command in two ways:
- **Environment variables**: `GRAFT_STATE_<NAME>` (uppercase, hyphens replaced with underscores)
- **Template variables**: `{{ state.<name> }}` (available when `stdin` uses a template)

**Default**: Empty list.

**Constraints**:
- Each entry must exist in the `state:` section (cross-validated at parse time).
- Empty entries are rejected.

**Examples**:
```yaml
# Single context entry
context:
  - coverage

# Multiple context entries
context:
  - coverage
  - test-results
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

## Section: sequences

Defines multi-step command pipelines that the `graft run` tool can execute. Each sequence
chains named commands in order, shares arguments across all steps, and can optionally
retry on step failure or write a checkpoint gate when all steps succeed.

**Name Constraints**: Same as commands — no `:` character.

### Structure

```yaml
sequences:
  <sequence-name>:
    description: string              # Optional: human-readable summary
    steps:                           # Required: ordered list of command names
      - <command-name>
      - <command-name>
    args:                            # Optional: positional/choice arg declarations
      - name: string
        type: string | choice | flag
        description: string          # Optional
        required: bool               # Default: false
        positional: bool             # Default: false
        options: [string]            # Required when type: choice
        options_from: string         # Optional: state query name for dynamic options
    on_step_fail:                    # Optional: retry configuration
      step: string                   # Required: which step name triggers recovery
      recovery: string               # Required: command name to run before retry
      max: integer                   # Required: maximum retry iterations
    checkpoint: bool                 # Optional: write checkpoint.json on success (default: false)
```

### Fields

#### steps (required)
**Type**: `list[string]`

**Description**: Ordered list of command names to execute. Each entry must match a key
in the same `commands:` section. Commands are executed left-to-right; if any step exits
non-zero the sequence stops (unless `on_step_fail` is configured for that step).

**Semantics**: "Pass-all" args — every positional argument given to `graft run <seq>
<arg>` is forwarded to every step. Steps that ignore extra positional args are unaffected.

**Constraints**:
- Minimum 1 step.
- Each step name must match a key in the `commands:` section. This is validated at
  execution time (not parse time): a missing step name causes the sequence to abort
  with an error when that step is reached.

**Example**:
```yaml
steps:
  - build
  - test
  - deploy
```

#### description (optional)
**Type**: `string`

**Description**: Human-readable summary shown in `graft run --list` output and in Grove's
Commands section (prefixed with `» `).

**Example**:
```yaml
description: "Build, test, and deploy to staging"
```

#### args (optional)
**Type**: `list[ArgDef]`

**Description**: Declares positional or choice arguments accepted by this sequence. Same
schema as `args:` on individual commands. Declared args drive the Grove form-input overlay
when the sequence is invoked interactively.

**Example**:
```yaml
args:
  - name: slice
    type: choice
    description: "Which slice to implement"
    required: true
    positional: true
    options_from: slices
```

#### on_step_fail (optional)
**Type**: `object`

**Description**: Configures automatic retry with a recovery command when a specific step
fails. If the named step exits non-zero, graft runs the `recovery` command (passing the
same args), then re-runs the failed step — up to `max` times. If the recovery command
itself exits non-zero, the sequence aborts immediately without retrying.

Only one step may be configured for retry per sequence. Steps not named in `on_step_fail`
cause immediate sequence failure.

**Fields**:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `step` | `string` | Yes | Name of the step that triggers recovery on failure |
| `recovery` | `string` | Yes | Command name to run before each retry |
| `max` | `integer` | No (default: 3) | Maximum number of retry iterations (not counting the initial attempt) |

**Example**:
```yaml
on_step_fail:
  step: verify
  recovery: resume
  max: 3
```

**`sequence-state.json` schema** (written to `.graft/run-state/` on every state transition):
```json
{
  "sequence": "<sequence-name>",
  "step":     "<current-step-name>",
  "step_index": 0,
  "step_count": 2,
  "phase": "running | retrying | complete | failed",
  "iteration": 1
}
```

`iteration` is only present during retries. Phase transitions per step:
`running → (retrying with iteration) → complete | failed`.

#### checkpoint (optional)
**Type**: `bool`

**Default**: `false`

**Description**: When `true`, writes `.graft/run-state/checkpoint.json` after all steps
complete successfully. The checkpoint pauses automated orchestration until an explicit
approve/reject command is run. Useful for human-review gates in automated pipelines.

**checkpoint.json schema**:
```json
{
  "phase": "awaiting-review",
  "sequence": "<sequence-name>",
  "args": { "<arg-name>": "<arg-value>" },
  "message": "Sequence complete. Review and approve or reject to continue.",
  "created_at": "<RFC 3339 timestamp>"
}
```

After calling `graft run <dep>:approve`, the file is updated to `{"phase": "approved"}`.
After `graft run <dep>:reject`, it becomes `{"phase": "rejected"}`.

**Example**:
```yaml
checkpoint: true
```

### Sequence Examples

#### Simple build-test pipeline

```yaml
commands:
  build:
    run: "cargo build"
    description: "Compile the project"
  test:
    run: "cargo test"
    description: "Run all tests"

sequences:
  ci:
    description: "Build then test"
    steps:
      - build
      - test
```

#### Implement-verify with retry and checkpoint

This is the canonical `implement-verified` pattern used in software-factory workflows:
an AI agent implements a slice, then a verification step checks correctness. If verify
fails, a `resume` command re-runs the agent with failure context before retrying verify.
A checkpoint gate requires human review before the result is accepted.

```yaml
commands:
  implement:
    run: "claude --resume -p $GRAFT_STATE_DIR/session.json"
    description: "Run AI agent implementation step"
    args:
      - name: slice
        type: choice
        positional: true
        required: true
        options_from: slices

  verify:
    run: "./scripts/verify.sh"
    description: "Verify implementation correctness"
    writes: [verify]

  resume:
    run: "./scripts/resume.sh"
    description: "Resume agent with failure context"
    reads: [verify]
    args:
      - name: slice
        type: choice
        positional: true
        required: true
        options_from: slices

sequences:
  implement-verified:
    description: "Implement next slice step and verify, retrying on verify failure"
    steps:
      - implement
      - verify
    args:
      - name: slice
        type: choice
        description: "Slice to implement"
        required: true
        positional: true
        options_from: slices
    on_step_fail:
      step: verify
      recovery: resume
      max: 3
    checkpoint: true
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

  generate-report:
    run: "report-tool generate"
    description: "Generate coverage report from template"
    stdin:
      template: "templates/report.md"
      engine: tera
    context:
      - coverage

state:
  coverage:
    run: "pytest --cov --cov-report=json --quiet | jq '.totals.percent_covered'"
    cache:
      deterministic: true

sequences:
  test-and-report:
    description: "Run tests then generate coverage report"
    steps:
      - verify-v2
      - generate-report

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

                # Validate stdin field
                if 'stdin' in cmd_data:
                    stdin = cmd_data['stdin']
                    if isinstance(stdin, dict):
                        if 'template' not in stdin:
                            errors.append(f"Command '{cmd_name}': stdin object must have 'template' field")
                        if 'engine' in stdin and stdin['engine'] not in ('tera', 'none'):
                            errors.append(f"Command '{cmd_name}': unsupported engine '{stdin['engine']}'")
                    elif not isinstance(stdin, str):
                        errors.append(f"Command '{cmd_name}': stdin must be string or object")

                # Validate context entries reference state section
                if 'context' in cmd_data:
                    if not isinstance(cmd_data['context'], list):
                        errors.append(f"Command '{cmd_name}': context must be a list")
                    else:
                        state_keys = set(config.get('state', {}).keys())
                        for entry in cmd_data['context']:
                            if entry not in state_keys:
                                errors.append(
                                    f"Command '{cmd_name}': context entry '{entry}' "
                                    f"not found in state section"
                                )

    # Validate state section
    if 'state' in config:
        if not isinstance(config['state'], dict):
            errors.append("'state' must be an object")
        else:
            for query_name, query_data in config['state'].items():
                if 'run' not in query_data:
                    errors.append(f"State query '{query_name}': missing required 'run' field")

    # Validate sequences section
    if 'sequences' in config:
        if not isinstance(config['sequences'], dict):
            errors.append("'sequences' must be an object")
        else:
            command_keys = set(config.get('commands', {}).keys())
            for seq_name, seq_data in config['sequences'].items():
                if 'steps' not in seq_data or not seq_data['steps']:
                    errors.append(f"Sequence '{seq_name}': missing required 'steps' field")
                # Note: individual step names are NOT validated at parse time.
                # A step name that does not match any command causes an error at
                # execution time when that step is reached.
                if 'on_step_fail' in seq_data:
                    osf = seq_data['on_step_fail']
                    for field in ('step', 'recovery'):
                        if field not in osf:
                            errors.append(
                                f"Sequence '{seq_name}': on_step_fail missing required '{field}'"
                            )
                    # max has a default of 3; specifying max: 0 is accepted and
                    # causes the failed step to be reported immediately without any retry.

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
