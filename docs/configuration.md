# Configuration

## Environment variables

Set in `.env` file or CI secrets:

```bash
# Required
AWS_ACCESS_KEY_ID=AKIA...
AWS_SECRET_ACCESS_KEY=...
AWS_REGION=us-west-2
AWS_DEFAULT_REGION=us-west-2

# Optional
AWS_SESSION_TOKEN=...           # For temporary credentials
DOCFLOW_DIR=/path/to/docflow    # Override docflow location
```

Docflow uses the `llm` CLI with the `llm-bedrock-anthropic` plugin to invoke AWS Bedrock.

## Prompt frontmatter schema

The frontmatter supports these parameters:

```yaml
---
model: bedrock-claude-v4.5-sonnet-us   # Optional: model to use
deps:                                  # REQUIRED: source files
  - docs/strategy/foundations.md
  - docs/strategy/market.md
---
```

### Model

Default: `bedrock-claude-v4.5-sonnet-us`

Available models via AWS Bedrock:
- `bedrock-claude-v4.5-sonnet-us` - Claude Sonnet 4.5 (recommended)
- `bedrock-claude-v4-sonnet-us` - Claude Sonnet 4
- `bedrock-claude-v3.5-sonnet` - Claude Sonnet 3.5
- `bedrock-claude-v3-haiku` - Claude Haiku 3 (faster, less capable)

Use US inference profiles (`-us` suffix) for cross-region routing and higher throughput.

### Dependencies

The `deps:` list specifies source files:

```yaml
deps:
  - docs/strategy/foundations.md       # Manual source
  - docs/analysis/market.md            # Generated doc as input
  - ../external/research.md            # Outside docs/
```

Rules:
- Paths relative to repo root
- Can include generated docs (creates DAG)
- Changes to any dep trigger regeneration
- Order doesn't matter

## Model parameters

Docflow does not currently expose model inference parameters like temperature, top_p, or max_tokens. The models use their default settings:

- **Temperature**: Model default (typically 1.0)
- **Top P**: Model default (typically 0.999)
- **Max tokens**: Plugin default (4096 for Claude Sonnet 4.5)

These defaults are suitable for most documentation generation tasks. If you need more control over model behavior, consider modifying your prompt instructions to request specific output characteristics (e.g., "Be concise and focused" vs "Provide comprehensive analysis").

## DVC configuration

DVC settings in `.dvc/config`:

```ini
[core]
    remote = local
    autostage = true

['remote "local"']
    url = .dvc/cache
```

The default configuration:
- Uses local cache (no remote storage needed)
- Auto-stages outputs on generation
- Disables cache for documentation outputs

You rarely need to modify DVC configuration.

## Docker configuration

The `Dockerfile` includes:
- Python 3.12 base image
- DVC with local support
- `llm` CLI with Bedrock plugins
- All docflow scripts

To customize:
1. Edit `Dockerfile`
2. Run `make build`
3. Use updated image in `bin/docflow`

## Git hooks

Pre-commit hook in `.git/hooks/pre-commit`:

```bash
#!/bin/bash
# Auto-regenerate dvc.yaml if prompts changed
if git diff --cached --name-only | grep -q '\.prompt\.md$'; then
  bin/docflow sync
  git add dvc.yaml
fi
```

This ensures `dvc.yaml` stays synchronized with prompt files.
