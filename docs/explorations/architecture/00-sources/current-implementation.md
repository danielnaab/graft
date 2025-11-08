# Current Graft Implementation

## Overview

Graft is an LLM-powered documentation pipeline that maintains living documents by intelligently synthesizing source content according to instructions in prompt files.

## Current Architecture

### File Processing Model

- **Input**: `*.prompt.md` files with YAML frontmatter containing dependencies
- **Output**: Corresponding `.md` files (e.g., `foo.prompt.md` → `foo.md`)
- **Processing**: Native LLM patch application via `pack_prompt.py`

### Core Components

1. **generate_dvc.py**: Scans for `*.prompt.md` files and generates DVC pipeline stages
2. **pack_prompt.py**: Packs prompts with source diffs and previous output into LLM context
3. **render_llm.sh**: Executes the LLM to generate/update output
4. **DVC**: Orchestrates dependency tracking and change detection

### Change Detection Intelligence

The system uses git to track changes and determine actions:

- **GENERATE**: No previous output exists
- **REFINE**: Prompt changed, sources unchanged
- **UPDATE**: Sources changed, prompt unchanged
- **REFRESH**: Both changed
- **MAINTAIN**: Nothing changed

### Current Limitations

1. **Single output per prompt**: Each `.prompt.md` produces exactly one `.md` file
2. **Native LLM handling**: LLM patches are applied directly by graft's Python code
3. **Fixed naming convention**: `<name>.prompt.md` → `<name>.md` only
4. **No lock mechanism**: All documents regenerate when dependencies change

## Git-Native Philosophy

Graft is fundamentally git-aware:
- Reads previous state from `HEAD` for reproducibility
- Generates diffs to show what changed
- Allows rollback via git
- Treats commits as source of truth

## Build Artifacts

The `build/` directory contains:
- `<name>.promptpack.txt`: Packed prompt with all context
- `<name>.params.json`: Effective parameters (model, etc.)
- `<name>.context.json`: Dependency and metadata summary
- `<name>.attachments.json`: List of binary attachments (PDFs, images)
