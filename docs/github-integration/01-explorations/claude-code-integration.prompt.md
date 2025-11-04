---
deps:
  - docs/github-integration/00-sources/design-philosophy.md
  - docs/how-it-works.md
  - docs/claude-skills.md
  - .claude/skills/steward/SKILL.md
---
# Claude Code Integration Design

Based on the design philosophy and understanding of Claude Code's capabilities, produce a detailed design for Claude Code integration with Graft workflows.

## Claude Code Capabilities to Leverage

Research and analyze:
- **Slash commands**: Custom commands in `.claude/commands/`
- **Skills**: Complex multi-step workflows in `.claude/skills/`
- **GitHub integration**: How Claude Code interacts with PRs
- **Tool usage**: File operations, bash commands, git operations
- **Context management**: How Claude Code maintains understanding of the codebase

## Required Design Elements

### 1. Slash Commands for Common Operations

Design slash commands for:

#### `/graft-validate`
- Purpose: Quick validation of docs without regeneration
- Implementation: Run `bin/graft check` and report results
- Output: Clear success/failure with actionable errors
- When to use: Before committing, during review

#### `/graft-preview`
- Purpose: Show what will regenerate for current changes
- Implementation: Use `bin/graft status` to identify affected docs
- Output: List of docs with change detection actions (GENERATE/UPDATE/REFINE/etc)
- When to use: Before regeneration, during planning

#### `/graft-regen`
- Purpose: Regenerate all affected docs
- Implementation: Run `bin/graft rebuild` with progress reporting
- Output: Show each doc being generated with timing
- When to use: After editing sources/prompts

#### `/graft-impact <file>`
- Purpose: Show which docs depend on a specific file
- Implementation: Use `bin/graft uses <file>`
- Output: List of prompts that will regenerate if file changes
- When to use: Before editing shared sources

#### `/graft-new-doc`
- Purpose: Scaffold a new document prompt interactively
- Implementation: Ask for name, topic, deps, then run `bin/graft new`
- Output: Created prompt file, guidance on next steps
- When to use: Adding new documentation

### 2. Enhanced Steward Skill

Extend the existing steward skill with GitHub-specific capabilities:

#### New Operations
- **PR preparation**: Validate docs before push, suggest fixes
- **Impact analysis**: Automatically show cascade when editing sources
- **Prompt refinement**: Analyze generated output, suggest prompt improvements
- **Dependency management**: Add/remove deps with validation

#### Integration with Workflows
- Automatically offer to regenerate when editing sources
- Warn if editing generated files instead of prompts
- Suggest relevant source files when creating new prompts
- Validate before allowing git operations

### 3. GitHub Integration Features

Design how Claude Code should interact with GitHub:

#### PR Context Awareness
- Detect when working in PR branch
- Show which docs will regenerate for current PR
- Offer to validate before pushing
- Suggest including regenerated docs in commits

#### Review Response Workflow
- Parse review comments about documentation
- Identify relevant prompts/sources to edit
- Make changes and offer to regenerate
- Prepare response commit

#### Commit Message Assistance
- Suggest commit messages that explain both human and AI changes
- Template: "Add X feature docs\n\n- Created source Y\n- Updated prompt Z\n- Generates W documentation"

### 4. Intelligent Assistance Patterns

Design proactive assistance:

#### Before Regeneration
- Check if AWS credentials are configured
- Verify dependencies exist
- Estimate time and cost
- Ask for confirmation if many docs will regenerate

#### During Regeneration
- Show progress (X of Y docs complete)
- Report any failures with context
- Allow cancellation if needed
- Provide ETA based on remaining docs

#### After Regeneration
- Show git diff summary
- Highlight significant changes
- Offer to review output quality
- Suggest refinements if output has issues

#### Error Recovery
- Parse error messages from Graft/DVC
- Explain error in user-friendly terms
- Suggest specific fixes
- Offer to implement fix automatically

### 5. Workflow Automation

Design higher-level workflows:

#### "Update Documentation" Flow
1. Ask user what changed
2. Identify relevant source files
3. Offer to edit sources or create new ones
4. Update prompts if needed
5. Validate dependencies
6. Regenerate affected docs
7. Review output with user
8. Iterate if needed
9. Commit with good message

#### "New Feature Documentation" Flow
1. Ask about feature and audience
2. Suggest documentation structure
3. Create source files
4. Scaffold prompts with deps
5. Generate initial drafts
6. Review and refine
7. Add to documentation index
8. Commit complete documentation set

#### "Fix Stale Docs" Flow
1. Detect stale docs (run validation)
2. Show which sources/prompts changed
3. Explain what will regenerate
4. Offer to regenerate automatically
5. Review changes
6. Commit fix

### 6. Configuration and Setup

Design setup experience:

#### First-Time Setup
- Detect if Graft is configured
- Check AWS credentials
- Verify Docker is available
- Build/pull Graft image
- Initialize DVC if needed
- Explain workflow basics

#### Health Checks
- `/graft-doctor` command to diagnose issues
- Check all prerequisites
- Verify credentials work
- Test LLM connectivity
- Validate configuration files

### 7. User Experience Principles

Design for:
- **Clarity**: Always explain what will happen before doing it
- **Safety**: Never destructive operations without confirmation
- **Feedback**: Show progress, report success/failure clearly
- **Learning**: Teach users about Graft concepts through use
- **Efficiency**: Common operations should be fast and simple

## Deliverables

Provide:

1. **Command Specifications**: For each slash command, full specification
2. **Skill Extensions**: Changes needed to steward skill
3. **Workflow Diagrams**: Visual flow for complex multi-step workflows
4. **Error Handling Matrix**: Common errors → user-friendly explanations → fixes
5. **Setup Guide**: What configuration is needed
6. **Integration Points**: How commands interact with GitHub Actions

Be specific about user interactions, command syntax, and expected outputs. This will guide the implementation of Claude Code integration.
