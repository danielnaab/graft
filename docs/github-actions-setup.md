# GitHub Actions Setup for Graft

This guide explains how to configure GitHub Actions to validate Graft documentation automatically in pull requests.

## Overview

The Graft validation workflow runs automatically on pull requests to:
1. Verify `dvc.yaml` is synchronized with prompt files
2. Check all dependencies exist
3. Detect stale documentation

**Note**: The validation workflow does NOT regenerate documentation. It only checks for inconsistencies. Regeneration happens locally via `bin/graft rebuild`.

## Prerequisites

- GitHub repository with Graft documentation
- AWS account with Bedrock access (only needed for regeneration, not validation)
- Repository admin access (to configure secrets and branch protection)

## Workflow Behavior

### What It Validates

The workflow performs three checks:

#### 1. DVC Pipeline Synchronization
Ensures `dvc.yaml` matches the current `*.prompt.md` files.

**Why it matters**: If you add a new prompt but forget to run `bin/graft sync`, the pipeline won't include it.

**How to fix**: Run `bin/graft sync` locally and commit `dvc.yaml`.

#### 2. Missing Dependencies
Verifies all files referenced in prompt `deps:` sections exist.

**Why it matters**: Missing dependencies cause generation failures.

**How to fix**: Create the missing file or remove it from the deps list.

#### 3. Stale Documentation
Checks if generated documentation needs regeneration due to source or prompt changes.

**Why it matters**: Stale docs mean the committed documentation doesn't reflect current sources/prompts.

**How to fix**: Run `bin/graft rebuild` locally and commit the regenerated files.

### When It Runs

- **Pull Requests**: On open, synchronize, and reopen events
- **Push to Main**: On pushes that modify documentation files

The workflow only runs when relevant files change:
- `**.prompt.md` - Prompt files
- `**.md` - Documentation files
- `dvc.yaml` / `dvc.lock` - DVC pipeline files
- `scripts/**` - Graft scripts

### Performance

- **Validation time**: 1-2 minutes
- **No AWS costs**: Validation doesn't call LLM APIs
- **Caching**: Docker images are cached between runs

## Setup Instructions

### Step 1: Workflow File

The workflow file is at `.github/workflows/graft-validate.yml`. It's automatically included in the repository.

No changes needed for basic validation.

### Step 2: Configure Secrets (Optional for Validation)

Validation doesn't require AWS credentials, but if you want to enable full regeneration in CI (future feature), you'll need to configure AWS access.

#### Option A: AWS Access Keys (Simple)

1. Create an IAM user with Bedrock permissions:
   ```json
   {
     "Version": "2012-10-17",
     "Statement": [
       {
         "Effect": "Allow",
         "Action": [
           "bedrock:InvokeModel"
         ],
         "Resource": "arn:aws:bedrock:*::foundation-model/anthropic.claude-*"
       }
     ]
   }
   ```

2. Add secrets to your GitHub repository:
   - Go to Settings → Secrets and variables → Actions
   - Add `AWS_ACCESS_KEY_ID`
   - Add `AWS_SECRET_ACCESS_KEY`
   - Add `AWS_REGION` (e.g., `us-west-2`)

#### Option B: OIDC (More Secure)

For production use, OIDC federation is recommended. See [AWS documentation](https://docs.github.com/en/actions/deployment/security-hardening-your-deployments/configuring-openid-connect-in-amazon-web-services) for setup instructions.

### Step 3: Enable Branch Protection (Recommended)

To prevent merging PRs with stale documentation:

1. Go to Settings → Branches
2. Add a branch protection rule for `main`
3. Enable "Require status checks to pass before merging"
4. Select "Validate Graft Documentation" from the list
5. Enable "Require branches to be up to date before merging"

Now PRs with stale docs cannot be merged until fixed.

## Using the Workflow

### Successful Validation

When validation passes, you'll see:
```
✅ DVC pipeline synchronized
✅ All dependencies present
✅ Documentation up to date

🎉 All validation checks passed!
```

### Failed Validation

When validation fails, the workflow provides actionable errors:

**Example: Stale documentation**
```
❌ ERROR: Generated documentation is stale

The following documentation needs regeneration:
  docs/how-it-works.md:
    changed deps: scripts/pack_prompt.py

Fix: Run 'bin/graft rebuild' locally and commit the regenerated files
```

**Example: Missing dependency**
```
❌ ERROR: Missing dependencies detected

  - docs/api/endpoints.md (required by docs/api/reference.prompt.md)

Fix: Create the missing files or remove them from the prompt deps
```

**Example: Unsynchronized DVC**
```
❌ ERROR: dvc.yaml is not synchronized with prompt files

Fix: Run 'bin/graft sync' locally and commit the changes
```

### Fixing Issues

1. Read the error message in the GitHub Actions log
2. Run the suggested fix command locally
3. Commit and push the changes
4. The workflow re-runs automatically

## Local Development Workflow

Best practice is to validate locally before pushing:

```bash
# Check status
bin/graft status

# If stale, regenerate
bin/graft rebuild

# Verify no issues
bin/graft sync
git status

# Commit everything together
git add .
git commit -m "Update documentation"
git push
```

Or use Claude Code commands:

```
/graft-validate   # Check status
/graft-regen      # Regenerate if needed
```

## Troubleshooting

### Workflow doesn't run

**Problem**: PR created but workflow doesn't appear

**Solution**: Check that your PR modifies files matching the workflow paths filter. The workflow only runs for documentation-related changes.

### Validation fails but local check passes

**Problem**: `bin/graft rebuild` succeeds locally but CI fails

**Solution**: Ensure you committed all regenerated files:
```bash
git status  # Check for uncommitted changes
git add docs/
git commit --amend
git push --force-with-lease
```

### False positives

**Problem**: Validation reports stale docs but they're actually current

**Solution**: This may indicate:
1. DVC cache issues - try `dvc status` locally
2. Git state mismatch - ensure you pushed all commits
3. Prompt file modified but not committed

### Performance issues

**Problem**: Validation takes too long

**Solution**: The workflow should complete in 1-2 minutes. If slower:
1. Check GitHub Actions runner availability
2. Verify Docker caching is working
3. Review the workflow logs for bottlenecks

## Advanced Configuration

### Customize Path Filters

Edit `.github/workflows/graft-validate.yml` to change which files trigger validation:

```yaml
on:
  pull_request:
    paths:
      - '**.prompt.md'
      - '**.md'
      - 'custom-source-dir/**'  # Add your paths
```

### Adjust Timeout

Default timeout is 10 minutes. For large documentation sets:

```yaml
jobs:
  validate:
    timeout-minutes: 20  # Increase if needed
```

### Add Slack Notifications

See [GitHub Actions documentation](https://github.com/marketplace/actions/slack-notify) for integrating notifications.

## Future Enhancements

Planned improvements (Phase 2+):
- **Preview Generation**: Optionally regenerate docs in CI and post diffs as PR comments
- **Auto-commit**: Bot commits regenerated docs automatically
- **Cost Estimation**: Show estimated AWS costs before regeneration
- **Parallel Validation**: Faster checks through parallelization

## Support

- Issues: https://github.com/danielnaab/graft/issues
- Implementation Framework: [docs/github-integration/02-frameworks/implementation-framework.md](github-integration/02-frameworks/implementation-framework.md)
- Design Philosophy: [docs/github-integration/00-sources/design-philosophy.md](github-integration/00-sources/design-philosophy.md)
