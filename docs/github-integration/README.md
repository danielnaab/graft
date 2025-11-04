# GitHub Integration for Graft

This directory contains the design and implementation of Graft's GitHub Actions and Claude Code integration.

## Overview

The GitHub integration enables automated validation of documentation in pull requests and provides Claude Code slash commands for rapid iteration on documentation.

## Key Features

- **Automated Validation**: GitHub Actions check that generated docs match their sources and prompts
- **Claude Code Commands**: Slash commands for validation, regeneration, and impact analysis
- **Merge Protection**: Prevent merging PRs with stale documentation
- **Fast Feedback**: Validation completes in seconds, full regeneration in minutes

## Documentation Structure

### 00-sources/
- **design-philosophy.md**: Strategic thinking on how Graft, GitHub PRs, and Claude Code intersect

### 01-explorations/
Deep-dive analyses of specific aspects:
- **workflow-patterns.md**: Developer, CI/CD, and review workflow specifications
- **github-actions-strategy.md**: GitHub Actions implementation approach and design decisions
- **claude-code-integration.md**: Slash command and skill enhancement designs
- **validation-strategy.md**: Multi-layer validation system architecture

### 02-frameworks/
- **implementation-framework.md**: Synthesis into actionable roadmap with phase breakdown

## Implementation Status

### Phase 1: Foundation (MVP) ✅ In Progress

- [x] Design documentation complete
- [x] GitHub Actions validation workflow created
- [x] Essential slash commands implemented
- [ ] AWS authentication documented
- [ ] End-to-end testing complete
- [ ] Documentation published

### Phase 2: Enhanced Workflows (Planned)

- Preview generation workflow
- Enhanced steward skill
- Auto-commit capability
- Performance optimizations

### Phase 3: Advanced Features (Planned)

- Advanced CI/CD integration
- Sophisticated validation modes
- Full workflow automation
- Monitoring and metrics

## Quick Start

### For Contributors

1. **GitHub Actions** automatically runs on PRs - no setup needed for validation

2. **Claude Code Commands** are available immediately:
   - `/graft-validate` - Check documentation status
   - `/graft-regen` - Regenerate stale docs
   - `/graft-preview` - Preview changes before regenerating
   - `/graft-impact <file>` - See which docs depend on a file

### For Maintainers

See [GitHub Actions Setup](../github-actions-setup.md) for:
- Configuring AWS credentials
- Enabling the validation workflow
- Setting up branch protection rules

## Architecture

The integration follows Graft's core principles:

1. **Git-Native**: Everything in version control
2. **Intelligent Change Detection**: Distinguish source vs instruction changes
3. **DAG-Based**: Automatic cascade through documentation hierarchy
4. **Reproducible**: Same inputs → identical outputs
5. **Minimal Regeneration**: Only regenerate what changed

## Files

### GitHub Actions Workflows
- `.github/workflows/graft-validate.yml` - Validation workflow (Layer 1 & 2 checks)

### Claude Code Commands
- `.claude/commands/graft-validate.md` - Validation command
- `.claude/commands/graft-regen.md` - Regeneration command
- `.claude/commands/graft-preview.md` - Preview command
- `.claude/commands/graft-impact.md` - Impact analysis command

### Documentation
- `docs/github-actions-setup.md` - Setup guide (to be created)
- This README - Overview and status

## Design Philosophy Highlights

From [design-philosophy.md](00-sources/design-philosophy.md):

**The Powerful Intersection**

When Graft meets GitHub PRs, documentation changes become:
- **Reviewable**: See sources, prompts, and outputs together
- **Validated**: CI/CD ensures synchronization
- **Assisted**: Claude Code helps refine prompts and sources
- **Auditable**: Commit history shows human and AI changes
- **Trustworthy**: Reproducible regeneration provides confidence

This isn't just automation - it's creating a workflow where documentation naturally stays synchronized, quality emerges from process, and AI assistance amplifies human capability.

## Contributing

1. Read the [implementation framework](02-frameworks/implementation-framework.md)
2. Pick a task from the appropriate phase
3. Implement following the technical specifications
4. Test with `/graft-validate` before committing
5. Open a PR - validation runs automatically

## Support

- Issues: https://github.com/danielnaab/graft/issues
- Framework Document: [implementation-framework.md](02-frameworks/implementation-framework.md)
- Design Philosophy: [design-philosophy.md](00-sources/design-philosophy.md)
