---
deps:
  - docs/naming-exploration/02-final/recommendation.md
  - docs/getting-started.md
  - docs/use-cases.md
  - docs/how-it-works.md
  - docs/configuration.md
  - docs/command-reference.md
  - docs/project-overview.md
---

Generate a professional, comprehensive README.md file for the Graft project that follows modern open-source README best practices.

**IMPORTANT**: Output the README content directly as markdown. Do NOT wrap the output in code fences or any other container.

## Key Requirements

### Logo and Branding
- Start with the Graft logo centered using a `<picture>` element for dark mode support:
  ```html
  <div align="center">

  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/assets/logo-dark.svg">
    <img src="docs/assets/logo.svg" alt="Graft Logo" width="200">
  </picture>

  </div>
  ```
- Center the logo and main heading in a clean, professional layout

### Project Description and Metaphor
The name "Graft" reflects the core functionality: like grafting branches in horticulture or commits in git, this tool takes content from multiple sources and carefully integrates them into a unified whole. The description should:
- Lead with a clear, concise one-liner explaining what Graft does
- Naturally incorporate the grafting metaphor (synthesis, integration, binding sources together)
- Emphasize the git-native approach and how Graft "grafts" documentation sources while preserving semantic intent
- Be professional and accessible without being overly technical in the introduction

### Structure
Follow this structure, keeping each section CONCISE but IMPACTFUL:
1. **Header** - Logo, title, tagline (centered)
2. **Overview** - What is Graft and why it exists (1-2 SHORT paragraphs incorporating the grafting metaphor)
3. **Key Features** - Bullet list of main capabilities (5-7 items max)
4. **Quick Start** - Minimal steps to get running (keep this section SHORT - max 5-6 commands)
5. **How It Works** - BRIEF explanation with simple example (keep it tight)
6. **Core Commands** - Quick reference table (keep minimal)
7. **Documentation** - Links to detailed docs (must include: Getting Started, Use Cases, How It Works, Configuration, Command Reference, Troubleshooting)
8. **Examples** - 2-3 POWERFUL examples that demonstrate real capabilities:
   - Show multi-level DAG power (how changes cascade through dependency chains)
   - Show intelligent change detection (UPDATE vs full regeneration)
   - Use realistic scenarios (changelogs from git, API docs, strategic docs, etc.)
   - Keep examples concise but impactful - show the "wow factor"
9. **License** - MIT License

IMPORTANT: Keep the README scannable but make sure examples DEMONSTRATE POWER. Show what makes Graft special.

### Tone and Style
- Professional but approachable
- Clear and concise - FAVOR BREVITY
- Developer-focused - respect their time
- Use active voice
- Incorporate the grafting metaphor naturally in the overview without forcing it

### Content Synthesis
Draw from the input documents to create a cohesive README that:
- Synthesizes information from getting-started.md, how-it-works.md, and project-overview.md
- References command-reference.md, configuration.md, and use-cases.md for detailed information
- Highlights what makes Graft unique (git-native, DVC-based, change detection, multi-level DAGs)
- Shows practical value quickly
- Points readers to use-cases.md for inspiration on powerful workflows and creative applications
- **CRITICAL**: Use the ACTUAL commands from command-reference.md - all commands start with `bin/graft` (e.g., `bin/graft init`, `bin/graft rebuild`, `bin/graft status`)
- **CRITICAL**: Use the ACTUAL implementation details from how-it-works.md - Graft uses `.prompt.md` files with frontmatter, not `.graft/config.yml`

### Terminology - Be Accurate About What Changes
**CRITICAL**: When discussing what happens when prompts change vs sources change, be ACCURATE:

**Source changes** (changes to files in `deps:`):
- Trigger UPDATE action
- Apply only the semantic changes from the modified sources
- Keep the rest of the document identical
- The LLM receives the git diff and instructions to incorporate only those changes

**Prompt changes** (changes to the prompt instructions themselves):
- Trigger RESTYLE action (though "restyle" is just one type of change)
- The prompt instructions can change ANYTHING: structure, focus, depth, format, audience, selection criteria, etc.
- This is a FULL REGENERATION with new instructions
- Don't reduce this to just "tone" or "style" - it's much more powerful than that

Use accurate language like:
- ✓ "Change the prompt instructions to..."
- ✓ "Modify how the document is generated..."
- ✓ "Update the generation instructions..."
- ✗ "Restyle the document..." (unless specifically talking about style)
- ✗ "Change the tone..." (unless specifically talking about tone)

### AWS Credentials
**CRITICAL**: Graft mounts your `~/.aws` directory for credentials. It supports:
- AWS profiles and SSO (via ~/.aws directory mount)
- Environment variables (passed through from host)
- .env file (optional fallback)

Do NOT say that AWS access keys are "required" in .env - they're one of several options. The ~/.aws mounting is the primary method.

### Technical Details
- Ensure all code blocks have appropriate language tags (bash, markdown, yaml, etc.)
- Keep the Quick Start section truly quick (max 5-6 commands)
- **CRITICAL COMMAND SYNTAX**: Use ONLY these actual commands from command-reference.md:
  - `bin/graft init` - initializes project (no arguments)
  - `bin/graft rebuild` - regenerates everything (no arguments - you cannot pass specific files)
  - `bin/graft new <name> [topic]` - creates new prompt file (e.g., `bin/graft new executive-summary strategy`)
  - `bin/graft status` - shows pipeline status (no arguments)
  - `bin/graft uses <file>` - shows dependencies
  - `bin/graft diff <stage_name>` - inspects prompt context
- **DO NOT** invent commands like `validate`, `clean`, or `rebuild <file>`
- **CRITICAL FRONTMATTER FORMAT**: Use ONLY the actual supported fields:
  ```yaml
  ---
  deps:           # REQUIRED - list of source files
    - file1.md
    - file2.md
  model: bedrock-claude-v4.5-sonnet-us  # OPTIONAL - defaults to this if not specified
  ---
  ```
- DO NOT show invented frontmatter fields like `sources:`, `type:`, `token_budget:`, `output:`, `action:`, or other fields not in configuration.md
- The `model:` field is optional and rarely needed in examples (defaults to Claude Sonnet 4.5)
- Link to detailed docs rather than duplicating everything
- Show the multi-level DAG capability with a clear example using only the actual frontmatter format
- Accurately reflect how Graft works with DVC, not invented configuration formats

The README should make a developer immediately understand what Graft does, why they'd want to use it, and how to get started, all while subtly reinforcing the grafting metaphor of bringing multiple sources together into a unified whole.
