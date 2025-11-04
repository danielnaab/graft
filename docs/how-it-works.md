# How It Works

Graft combines git-based change detection with DVC pipeline management to generate documentation efficiently.

## Architecture

The pipeline has five components:

1. **Source documents** - Manual `.md` files you write
2. **Prompt files** - `.prompt.md` files with YAML frontmatter specifying dependencies
3. **Pack prompt** - Script that analyzes git history and creates context
4. **Render** - Invokes Claude Sonnet 4.5 via AWS Bedrock
5. **Generate** - Creates output `.md` file co-located with prompt

## Change detection

The system tracks two types of changes:

### Source changes

Detected via git diff of files listed in `deps:` frontmatter. The system:
- Compares current version with previous commit
- Includes only changed sections in the packed prompt
- Passes a directive to update only affected content

### Prompt changes

Detected by comparing the prompt body with the previous commit. The system:
- Hashes the prompt instructions
- Detects any modification to the prompt text
- Triggers a complete regeneration when changed

## Actions

Based on what changed, the system determines the action:

- **GENERATE** - No previous draft exists → create document from scratch
- **UPDATE** - Sources changed, prompt unchanged → apply semantic changes only, keep rest identical
- **RESTYLE** - Prompt changed, sources unchanged → regenerate entire document with new instructions
- **REFRESH** - Both changed → apply source updates AND new instructions
- **MAINTAIN** - Nothing changed → keep document unchanged

These directives are passed to Claude, which applies them intelligently.

## Multi-level documentation DAGs

Generated documents can be inputs to other prompts, creating documentation hierarchies.

Example:
```
sources/strategy.md (manual)
  └─> explorations/analysis.md (generated)
      └─> frameworks/technical.md (generated)
          └─> artifacts/brief.md (generated)
```

DVC manages this efficiently:
- Stages use `cache: false` for outputs
- Files are git-tracked, no cache duplication
- Changes cascade through the dependency graph
- Only affected documents regenerate

## Auto-generated dvc.yaml

The `dvc.yaml` file is generated automatically by scanning all `*.prompt.md` files:

1. Each prompt becomes a DVC stage
2. Dependencies come from frontmatter `deps:` list
3. Output is co-located with prompt (same name, `.md` extension)
4. Model parameters from frontmatter become command arguments

Never edit `dvc.yaml` manually - it's regenerated on every run.

## Git hooks

The pre-commit hook ensures `dvc.yaml` stays in sync:
- Detects changes to any `.prompt.md` file
- Regenerates `dvc.yaml` automatically
- Stages both the prompt and updated `dvc.yaml`

This prevents desync between prompts and pipeline configuration.

## Docker execution

All operations run in Docker:
- Consistent environment across local and CI
- No local Python dependencies needed
- AWS credentials passed via environment
- Working directory mounted as volume

The `bin/graft` wrapper handles Docker invocation transparently.
