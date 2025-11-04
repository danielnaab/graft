---
model: bedrock-claude-v4.5-sonnet-us
deps:
  - scripts/pack_prompt.py
  - scripts/generate_dvc.py
  - scripts/render_llm.sh
---

# How It Works - Technical Architecture

Create comprehensive technical documentation explaining how Graft's change detection and pipeline management work.

## Required Sections

1. **Architecture Overview**
   - List and briefly describe the five components:
     1. Source documents
     2. Prompt files
     3. Pack prompt (change detection)
     4. Render (LLM invocation)
     5. Generate (output creation)
   - Show how these components fit together

2. **Change Detection**
   - Explain the two types of changes tracked:
     - **Source changes**: Detected via git diff of files in `deps:`
     - **Prompt changes**: Detected by comparing prompt body with previous commit
   - Detail the technical mechanism:
     - How git diffs are used
     - What sections are included in packed prompts
     - How directives are passed to the LLM

3. **Actions and Directives**
   - Document all five actions with precise definitions:
     - **GENERATE**: When and why
     - **UPDATE**: When and why (emphasize: semantic changes only, keep rest identical)
     - **RESTYLE**: When and why (emphasize: regenerate entire document with new instructions)
     - **REFRESH**: When and why (both types of changes)
     - **MAINTAIN**: When and why (no changes)
   - Explain that these directives are passed to Claude which applies them intelligently
   - Reference the actual logic from pack_prompt.py

4. **Multi-level Documentation DAGs**
   - Explain how generated documents can be inputs to other prompts
   - Show a concrete example of a documentation hierarchy
   - Explain how DVC manages this efficiently:
     - `cache: false` for outputs
     - Git-tracked files (no cache duplication)
     - Change cascades through dependency graph
     - Selective regeneration

5. **Auto-generated dvc.yaml**
   - Explain that dvc.yaml is generated automatically by scanning *.prompt.md files
   - Detail the generation process:
     - Each prompt becomes a DVC stage
     - Dependencies from `deps:` frontmatter
     - Output co-located with prompt
     - Model parameters become command arguments
   - Emphasize: never edit dvc.yaml manually

6. **Git Hooks**
   - Document the pre-commit hook functionality
   - Explain what it detects and what it does
   - Why this prevents desynchronization

7. **Docker Execution**
   - List the benefits of running everything in Docker
   - Explain how credentials and working directory are mounted
   - Note that bin/graft handles Docker invocation transparently

## Style Guidelines

- Use precise technical language
- Include actual code references where relevant (e.g., "defined in pack_prompt.py:116-126")
- Use concrete examples to illustrate abstract concepts
- Show file structures and DAGs visually (ASCII diagrams or indented lists)
- Explain the "why" behind design decisions, not just the "what"
- Cross-reference related documentation
- Assume reader has basic understanding of git, DVC, and Docker
- Be thorough but avoid unnecessary verbosity

## Technical Accuracy

- Reference actual function names, algorithms, and logic from the Python scripts
- Document the exact conditions that trigger each action
- Show the actual command structure and arguments
- Explain the git operations (rev-parse, diff, show) used for change detection
- Document the exact format of packed prompts and system messages

Generate documentation that gives developers a complete mental model of how Graft works under the hood.
