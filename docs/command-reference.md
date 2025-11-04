# Command Reference

All commands run via `bin/graft` wrapper which handles Docker execution.

## Core commands

### init

Initialize graft in a project.

```bash
bin/graft init
```

Sets up:
- DVC initialization with local cache
- Git hooks for auto-regeneration
- Default `.dvc/config`

Run once per project.

### rebuild

Regenerate `dvc.yaml` and run the full pipeline.

```bash
bin/graft rebuild
```

This is the main command for generating documentation. It:
1. Scans all `*.prompt.md` files
2. Generates `dvc.yaml` with DVC stages
3. Runs `dvc repro` to execute pipeline
4. Only regenerates changed documents

Default command if no arguments provided.

### sync

Regenerate `dvc.yaml` without running the pipeline.

```bash
bin/graft sync
```

Useful when:
- Adding/removing prompts
- Changing dependencies
- Validating configuration

Also aliased as `bin/graft check`.

### status

Show DVC pipeline status.

```bash
bin/graft status
```

Displays:
- Which stages are up to date
- Which stages need to run
- Which outputs have changed

Equivalent to `dvc status`.

## Document management

### new

Scaffold a new prompt file.

```bash
bin/graft new <name> [topic]
```

Examples:

```bash
# Create docs/executive-summary.prompt.md
bin/graft new executive-summary

# Create docs/strategy/board-brief.prompt.md
bin/graft new board-brief strategy
```

Creates a template with:
- Frontmatter with empty deps list
- Placeholder prompt instructions
- Proper YAML formatting

Edit the file to add dependencies and instructions, then run `rebuild`.

## Inspection

### diff

View the packed prompt context for a stage.

```bash
bin/graft diff <stage_name>
```

Example:

```bash
bin/graft diff executive_summary
```

Opens `build/<stage_name>.promptpack.txt` showing:
- Full assembled prompt
- All source content
- Change detection directives
- Model parameters

Useful for debugging why a document generates specific content.

### uses

Show which prompts depend on a file.

```bash
bin/graft uses <file>
```

Example:

```bash
bin/graft uses docs/strategy/foundations.md
```

Displays all prompt files that list this file in their `deps:`. Helps understand:
- Impact of changing a source file
- Which docs will regenerate
- Dependency graph structure

## Environment

The `bin/graft` wrapper accepts environment variables:

```bash
# Use graft from custom location
DOCFLOW_DIR=/custom/path bin/graft rebuild

# Pass through to docker
AWS_REGION=us-east-1 bin/graft rebuild
```

AWS credentials should be in `.env` file, but can be overridden on command line.

## Exit codes

- **0** - Success
- **1** - Error (missing dependencies, AWS auth failure, etc.)
- **2** - Docker image not found (run `make build`)

## Common workflows

### Add a new document

```bash
bin/graft new report analysis
# Edit docs/analysis/report.prompt.md
bin/graft rebuild
```

### Update dependencies

```bash
# Edit frontmatter deps: list in prompt file
bin/graft rebuild
```

### Change prompt tone

```bash
# Edit prompt instructions
bin/graft rebuild
# Triggers RESTYLE action
```

### Debug generation

```bash
bin/graft diff report
# Review packed prompt context
# Edit sources or prompt
bin/graft rebuild
```

### Check what will regenerate

```bash
# Before editing a source file
bin/graft uses docs/strategy/foundations.md
# Shows impact

# After changes
bin/graft status
# Shows what needs to run
```
