# Troubleshooting

Common issues and solutions.

## Nothing regenerated

### Check pipeline status

```bash
bin/docflow status
```

If all stages show "up to date", no changes were detected.

### Verify dependencies

```bash
# Check which files a prompt depends on
cat docs/your-doc.prompt.md | grep -A 10 "^deps:"

# Ensure the files you changed are listed
```

The system only regenerates when files in `deps:` change.

### Force regeneration

```bash
# Touch the prompt file to trigger RESTYLE
touch docs/your-doc.prompt.md
bin/docflow rebuild
```

## AWS authentication errors

### Verify credentials

```bash
cat .env | grep AWS_
```

Ensure these are set:
- `AWS_ACCESS_KEY_ID`
- `AWS_SECRET_ACCESS_KEY`
- `AWS_REGION`

### Test Bedrock access

```bash
# Inside the container
docker run --rm --env-file .env docflow:local \
  llm -m bedrock-claude-v4.5-sonnet-us "test"
```

If this fails:
- Check IAM permissions (need `bedrock:InvokeModel`)
- Verify model access in Bedrock console for your region
- Confirm region supports Claude Sonnet 4.5

### Temporary credentials

For AWS SSO or temporary credentials:

```bash
# Add to .env
AWS_SESSION_TOKEN=your-token-here
```

## Docker image not found

### Build the image

```bash
cd /path/to/docflow
make build
```

### Verify image exists

```bash
docker images | grep docflow
```

Should show `docflow:local`.

### Custom docflow location

```bash
export DOCFLOW_DIR=/custom/path/to/docflow
bin/docflow rebuild
```

## Large diffs in generated docs

Generated documents should have minimal diffs when sources change slightly.

### Ensure UPDATE directives

Check that prompts instruct:
- "Edit only where diffs imply semantic changes"
- "Keep existing content unchanged if sources unchanged"
- "Maintain exact formatting and structure"

### Review packed prompt

```bash
bin/docflow diff your-stage
```

Verify:
- Action is UPDATE (not RESTYLE or REFRESH)
- Only changed sections included in diff
- Prompt includes preservation instructions

### Adjust prompt instructions

Update your prompt to emphasize preservation:

```yaml
---
deps: [...]
---

When updating this document, preserve existing structure and wording.
Only modify sections where source diffs indicate substantive changes.
Keep formatting, headings, and unchanged sections identical.
```

## Dependency errors

### Dependency not found

```bash
bin/docflow sync
# Error: docs/missing-file.md doesn't exist
```

Either:
1. Create the missing file
2. Remove from `deps:` list in prompt frontmatter

### Circular dependencies

```bash
bin/docflow check
# Error: Circular dependency detected
```

Check your prompts - a generated doc can't depend on itself transitively:

```
❌ Bad:
a.md -> b.prompt.md -> b.md -> a.prompt.md -> a.md

✓ Good:
sources.md -> stage1.md -> stage2.md
```

## DVC errors

### Corrupted cache

```bash
dvc cache dir
rm -rf .dvc/cache/*
bin/docflow rebuild
```

### Stage failed

```bash
bin/docflow status
# Shows failed stage

# View logs
cat build/<stage-name>.log
```

Check for:
- AWS credential expiration
- Rate limiting (wait and retry)
- Invalid prompt syntax

## Performance issues

### Slow generation

Each document generation calls Claude Sonnet 4.5, which takes:
- 10-30 seconds per document (typical)
- Longer for complex synthesis or large source files

To optimize:
- Use focused, specific prompts for targeted output
- Keep source files focused and modular
- DVC generates documents in parallel automatically when dependencies allow

### Many unnecessary regenerations

```bash
# Check what's using a file
bin/docflow uses docs/frequently-changing.md
```

If many docs depend on a frequently-changing file:
- Consider splitting into multiple source files
- Group related dependencies
- Use more specific source files

## Getting help

If you encounter issues not covered here:

1. Check git history for changes to prompts or sources
2. Review packed prompt context with `bin/docflow diff`
3. Validate dependencies with `bin/docflow sync`
4. Test AWS access directly with `llm` CLI
5. Check DVC status and logs

For bugs or feature requests, open an issue in the docflow repository.
