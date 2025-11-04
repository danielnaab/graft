# GitHub Integration for Graft

This directory contains the design and implementation of Graft's GitHub Actions and Claude Code integration.

## Overview

The GitHub integration enables automated validation of documentation in pull requests and provides Claude Code slash commands for rapid iteration on documentation.

## Key Features

- **Automated Validation**: GitHub Actions verify docs match sources/prompts with proper YAML parsing and dependency validation
- **Preview Generation**: Regenerate docs in CI and see diffs in PR comments before committing
- **Claude Code Commands**: Five slash commands for validation, regeneration, preview, impact analysis, and scaffolding
- **Merge Protection**: Prevent merging PRs with stale documentation or invalid frontmatter
- **Fast Feedback**: Validation completes in ~1-2 minutes, preview in ~3-5 minutes depending on doc count

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

### Phase 1: Foundation (MVP) ✅ Complete

- [x] Design documentation complete
- [x] GitHub Actions validation workflow created
- [x] Essential slash commands implemented (`/graft-validate`, `/graft-regen`, `/graft-preview`, `/graft-impact`)
- [x] AWS authentication documented
- [x] Documentation published

### Phase 2: Enhanced Workflows ✅ Complete

- [x] **Preview generation workflow** - Regenerate docs in CI and post diffs as PR comments
- [x] **Python validation script** - Proper YAML parsing with frontmatter, dependency, and cycle validation
- [x] **Enhanced slash commands** - `/graft-new-doc` for interactive documentation scaffolding
- [x] **Improved validation** - Detects missing dependencies and circular dependencies
- [x] **Documentation updates** - Complete setup guide and usage documentation

### Phase 3: Advanced Features (Future)

- Docker image caching to GHCR
- Enhanced steward skill with PR-aware capabilities
- Auto-commit workflow
- Advanced monitoring and metrics
- Performance optimizations

## Quick Start

### For Contributors

1. **GitHub Actions** automatically runs on PRs - no setup needed for validation

2. **Claude Code Commands** are available immediately:
   - `/graft-validate` - Check documentation status (DVC sync, deps, staleness)
   - `/graft-regen` - Regenerate stale docs with progress tracking
   - `/graft-preview` - Preview what will change without regenerating
   - `/graft-impact <file>` - Show which docs depend on a file (cascade analysis)
   - `/graft-new-doc` - Interactively scaffold a new documentation prompt

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
- `.github/workflows/graft-validate.yml` - Automated validation (DVC sync, prompt validation, staleness)
- `.github/workflows/graft-preview.yml` - On-demand preview generation (triggered by label)

### Scripts
- `scripts/validate.py` - Python validation script with PyYAML parsing
- `test-validation.sh` - Local validation test script

### Claude Code Commands
- `.claude/commands/graft-validate.md` - Check documentation status
- `.claude/commands/graft-regen.md` - Regenerate stale docs
- `.claude/commands/graft-preview.md` - Preview changes without regenerating
- `.claude/commands/graft-impact.md` - Analyze dependency cascades
- `.claude/commands/graft-new-doc.md` - Scaffold new documentation prompts

### Documentation
- `docs/github-actions-setup.md` - Complete setup guide with preview workflow usage
- `docs/github-integration/README.md` - This overview document
- `docs/github-integration/00-sources/design-philosophy.md` - Strategic design thinking
- `docs/github-integration/01-explorations/` - Deep-dive analyses (4 documents)
- `docs/github-integration/02-frameworks/implementation-framework.md` - Implementation roadmap

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
