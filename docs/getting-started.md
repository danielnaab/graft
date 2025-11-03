# Getting Started

Docflow is an LLM-powered documentation pipeline that generates Markdown documents from Markdown prompts using DVC, AWS Bedrock, and Claude Sonnet 4.5.

## Prerequisites

- Docker
- AWS credentials with Bedrock access to Claude Sonnet 4.5

## Installation

### Build docflow

```bash
git clone <docflow-repo>
cd docflow
make build
```

This builds the `docflow:local` Docker image with all dependencies.

### Set up your project

```bash
cd ../your-project

# Copy environment template
cp .env.example .env

# Edit .env with your AWS credentials
# Required: AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_REGION
```

### Initialize docflow

```bash
bin/docflow init
```

This initializes DVC and sets up git hooks for automatic `dvc.yaml` regeneration.

## Create your first document

### Scaffold a new prompt

```bash
bin/docflow new executive-summary strategy
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
bin/docflow rebuild
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
- Edit source files to add content
- Edit the prompt to change tone or structure
- Run `bin/docflow rebuild` again

The system automatically detects what changed and applies appropriate updates.

## Next steps

- Read [how-it-works.md](how-it-works.md) to understand change detection and DAGs
- See [configuration.md](configuration.md) for all options
- Check [command-reference.md](command-reference.md) for all CLI commands
