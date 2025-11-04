# Getting Started

Graft is an LLM-powered documentation pipeline that generates Markdown documents from Markdown prompts using DVC, AWS Bedrock, and Claude Sonnet 4.5.

## Prerequisites

- Docker
- AWS credentials with Bedrock access to Claude Sonnet 4.5

## Installation

### Build graft

```bash
git clone <graft-repo>
cd graft
make build
```

This builds the `graft:local` Docker image with all dependencies.

### Set up your project

```bash
cd ../your-project

# Configure AWS credentials (choose one method):
# Method 1 (recommended): Use your existing ~/.aws configuration
#   - Graft automatically mounts ~/.aws for profiles and SSO
#   - No additional setup needed
#
# Method 2: Use environment variables in .env file
cp .env.example .env
# Edit .env: AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_REGION
```

### Initialize graft

```bash
bin/graft init
```

This initializes DVC and sets up git hooks for automatic `dvc.yaml` regeneration.

## Create your first document

### Scaffold a new prompt

```bash
bin/graft new executive-summary strategy
```

This creates `docs/strategy/executive-summary.prompt.md` with template frontmatter.

### Add dependencies and instructions

Edit the prompt file:

```yaml
---
deps:
  - docs/strategy/foundations.md
  - docs/strategy/market-analysis.md
---

Create an executive summary that synthesizes the strategic foundations and market analysis.

Focus on key insights and actionable recommendations.
```

### Generate the document

```bash
bin/graft rebuild
```

This:
1. Regenerates `dvc.yaml` from all prompt files
2. Runs the DVC pipeline
3. Generates `docs/strategy/executive-summary.md`

## Review and iterate

View the generated document:

```bash
cat docs/strategy/executive-summary.md
```

To refine:
- Edit source files to add content → triggers UPDATE (incorporates new content only)
- Edit the prompt instructions → triggers full regeneration with new instructions
- Run `bin/graft rebuild` again

The system automatically detects what changed and applies appropriate updates.

## Next steps

- Explore [use-cases.md](use-cases.md) for powerful workflows and creative applications
- Read [how-it-works.md](how-it-works.md) to understand change detection and DAGs
- See [configuration.md](configuration.md) for all options
- Check [command-reference.md](command-reference.md) for all CLI commands
