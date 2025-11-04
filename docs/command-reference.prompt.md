---
deps:
  - scripts/entrypoint.sh
  - bin/graft
  - docs/how-it-works.md
  - docs/configuration.md
  - docs/troubleshooting.md
---

Generate comprehensive command reference documentation for Graft that serves as both a quick reference and detailed guide.

## Document Purpose

Create clear, authoritative documentation of all Graft commands. Users should be able to:
- Quickly look up command syntax and options
- Understand what each command does and when to use it
- Learn common workflows and usage patterns
- Troubleshoot issues with command invocation

## Structure

### Introduction
Brief paragraph explaining:
- All commands run via `bin/graft` wrapper (handles Docker execution)
- Default command behavior (rebuild if none specified)
- Where to find detailed conceptual information (how-it-works.md)

### Command Organization

Group commands into these sections:

#### 1. Core Commands
Commands for the main documentation generation workflow:
- **init** - Initialize Graft in a project (DVC + hooks + .env)
- **rebuild** - Regenerate dvc.yaml and run the full pipeline (default command)
- **sync** - Regenerate dvc.yaml without running the pipeline
- **status** - Show DVC pipeline status

For each command:
- Show syntax with code block
- Explain what it does (2-3 sentences)
- Describe what gets set up/executed
- When to use it
- Note any aliases (e.g., sync/check)

#### 2. Document Management
Commands for working with prompt files:
- **new** - Scaffold a new prompt file with template

Include:
- Full syntax with parameters
- Multiple concrete examples showing different use cases
- What the generated template contains
- Next steps after creation

#### 3. Inspection & Debugging
Commands for understanding the system:
- **diff** - View the packed prompt context for a stage
- **uses** - Show which prompts depend on a file (reverse dependency lookup)

For each:
- Syntax and parameters
- What information it reveals
- How to interpret the output
- When this is useful (debugging scenarios)

### Environment & Configuration

Brief section covering:
- Environment variables the wrapper accepts (DOCFLOW_DIR, AWS credentials)
- How AWS credentials are passed through (~/.aws mount, env vars, .env file)
- Reference configuration.md for detailed credential setup

### Exit Codes

Document the three exit codes:
- **0** - Success
- **1** - Error (with examples: missing deps, AWS auth failure, etc.)
- **2** - Docker image not found or invalid command

### Common Workflows

Provide 5-7 practical workflow examples with bash commands and brief explanations:
1. **Add a new document** - new, edit, rebuild
2. **Update dependencies** - edit frontmatter, rebuild
3. **Change prompt instructions** - edit prompt, rebuild (triggers RESTYLE)
4. **Debug generation** - use diff to inspect packed prompt context
5. **Check impact** - use uses to see what will regenerate before editing a source
6. **Force regeneration** - touch prompt file if needed
7. **Validate configuration** - use sync/check before running pipeline

Each workflow should:
- Show actual bash commands
- Use realistic file/stage names
- Include brief comment explaining each step
- Be immediately actionable

## Writing Guidelines

**Tone & Style:**
- Technical reference style - clear, precise, authoritative
- Active voice for command descriptions
- Present tense ("generates", "shows", "runs")
- Professional but approachable

**Clarity:**
- One clear purpose per command
- Distinguish between similar commands (sync vs rebuild, diff vs uses)
- Explain both what happens and why you'd use it
- Use consistent terminology matching other docs

**Examples:**
- Every command should have at least one example
- Use realistic names (not foo/bar)
- Show complete command invocations
- Include comments for clarity in workflow sections

**Accuracy:**
- Use ONLY commands defined in entrypoint.sh
- Match actual parameter names and syntax exactly
- Reference actual model identifiers from configuration.md
- Ensure workflow examples would actually work

**Organization:**
- Scannable structure with clear headings
- Commands grouped by purpose
- Most common commands first within each section
- Workflows section shows commands in context

**Integration:**
- Reference how-it-works.md for concepts (actions, change detection, DAGs)
- Reference configuration.md for detailed AWS setup
- Reference troubleshooting.md for error scenarios
- Keep this doc focused on command syntax and usage

## Output Format

Generate the complete command reference as clean markdown suitable for docs/command-reference.md. The document should:
- Be comprehensive but scannable
- Cover all commands in entrypoint.sh
- Include 5-7 practical workflow examples
- Use proper markdown formatting (code blocks with bash syntax highlighting)
- Length: approximately 200-250 lines

**IMPORTANT**: Output ONLY the markdown content. Do NOT wrap in code fences. Start with level-1 heading.

**CRITICAL**: When describing the actions system (GENERATE, UPDATE, RESTYLE, etc.), be accurate:
- Source changes → UPDATE action (applies only the semantic changes from modified sources)
- Prompt changes → RESTYLE action (full regeneration with new instructions, not just style)
- Use precise language about what changes: "prompt instructions" not just "tone" or "style"
