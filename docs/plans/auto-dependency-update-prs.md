---
title: "Plan: Auto-Create PRs for Graft Dependency Updates"
date: 2026-01-05
status: draft
version: 1.0
---

# Plan: Auto-Create PRs for Graft Dependency Updates

## Overview

This plan describes a CI automation strategy that automatically creates pull requests in downstream repositories when upstream Graft dependencies are updated. This enables:

1. **Keeping dependencies current** - Downstream projects stay up-to-date with upstream changes
2. **Evolution feedback loop** - Each dependency update serves as an axis of change that informs Graft's own improvement
3. **Continuity affordance** - PRs include links to Coder Task workspaces, enabling seamless handoff if implementation requires additional work

## Context

### Current State

**Repositories and dependency chain:**

```
meta-knowledge-base
        ↓ (upstream of)
graft-knowledge
        ↓ (upstream of)
graft
```

**Configuration format (graft.yaml):**
```yaml
apiVersion: graft/v0
deps:
  graft-knowledge: "ssh://forgejo@platform-vm:2222/daniel/graft-knowledge.git#main"
```

**CI Platform:** Forgejo Actions (GitHub Actions compatible)

**Current gaps:**
- No automation for dependency updates
- Manual process to update refs and create PRs
- No tracking of which workspace was used for implementation

### Design Principles

1. **Push-based triggers** - Upstream repos trigger downstream updates (real-time, not polling)
2. **Single source of truth** - Workflow logic lives in upstream repos (knows when changes occur)
3. **Reusable components** - Shared action for common PR creation logic
4. **Extensibility** - Easy to add new dependency relationships
5. **Testability** - Each component independently testable

## Goals

### Primary Goals

1. **Implement push-based auto-PR workflow** - When upstream main is updated, create PR in downstream repo
2. **Include Coder workspace links** - PRs reference the workspace where work was initiated
3. **Initial scope** - `graft-knowledge` → `graft` and `meta-knowledge-base` → `graft-knowledge`
4. **Comprehensive tests** - Unit and integration tests for all components

### Secondary Goals

5. **Excellent ergonomics** - Clear PR descriptions, meaningful commit messages
6. **Failure resilience** - Graceful handling of conflicts, network issues
7. **Audit trail** - Log dependency update events for analysis

### Success Criteria

- [ ] Push to `graft-knowledge/main` triggers PR creation in `graft` repo
- [ ] Push to `meta-knowledge-base/main` triggers PR creation in `graft-knowledge` repo
- [ ] PRs include link to originating Coder workspace (when applicable)
- [ ] PRs have clear descriptions of what changed upstream
- [ ] Workflow handles edge cases (no changes, conflicts, existing PRs)
- [ ] All workflow logic has tests

## Technical Plan

### Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         UPSTREAM REPOSITORY                          │
│  (e.g., graft-knowledge)                                            │
│                                                                      │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │  .forgejo/workflows/notify-dependents.yml                      │ │
│  │                                                                 │ │
│  │  Trigger: push to main                                         │ │
│  │  Action:  For each dependent repo:                             │ │
│  │           1. Clone downstream repo                             │ │
│  │           2. Update graft.yaml with new commit                 │ │
│  │           3. Create branch and PR                              │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                                                                      │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │  dependents.yaml                                               │ │
│  │                                                                 │ │
│  │  Configuration file listing downstream repositories:           │ │
│  │  - repo: ssh://forgejo@platform-vm:2222/daniel/graft.git      │ │
│  │    dep_name: graft-knowledge                                   │ │
│  └────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│                      DOWNSTREAM REPOSITORY                          │
│  (e.g., graft)                                                      │
│                                                                      │
│  Receives PR with:                                                  │
│  - Updated graft.yaml (new ref for upstream dep)                   │
│  - PR description with upstream changes summary                     │
│  - Link to Coder workspace for continuation                        │
└─────────────────────────────────────────────────────────────────────┘
```

### Phase 1: Core Workflow Implementation

**Location:** `.forgejo/workflows/notify-dependents.yml` (in each upstream repo)

**Workflow structure:**
```yaml
name: Notify Dependents

on:
  push:
    branches: [main]
  workflow_dispatch:
    inputs:
      dry_run:
        description: 'Dry run (no PR creation)'
        type: boolean
        default: false

jobs:
  notify-dependents:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout this repository
        uses: actions/checkout@v3
        with:
          fetch-depth: 2  # Need previous commit for changelog

      - name: Get commit info
        id: commit
        run: |
          echo "sha=$(git rev-parse HEAD)" >> $GITHUB_OUTPUT
          echo "short_sha=$(git rev-parse --short HEAD)" >> $GITHUB_OUTPUT
          echo "message=$(git log -1 --pretty=%s)" >> $GITHUB_OUTPUT
          echo "author=$(git log -1 --pretty=%an)" >> $GITHUB_OUTPUT

      - name: Read dependents config
        id: config
        run: |
          if [ -f dependents.yaml ]; then
            # Parse YAML and output as JSON for processing
            echo "dependents=$(yq -o=json dependents.yaml)" >> $GITHUB_OUTPUT
          else
            echo "dependents=[]" >> $GITHUB_OUTPUT
          fi

      - name: Create PRs in dependent repositories
        env:
          UPSTREAM_REPO: ${{ github.repository }}
          UPSTREAM_SHA: ${{ steps.commit.outputs.sha }}
          UPSTREAM_SHORT_SHA: ${{ steps.commit.outputs.short_sha }}
          UPSTREAM_MESSAGE: ${{ steps.commit.outputs.message }}
          UPSTREAM_AUTHOR: ${{ steps.commit.outputs.author }}
          CODER_WORKSPACE_URL: ${{ secrets.CODER_WORKSPACE_URL || '' }}
          DRY_RUN: ${{ inputs.dry_run || 'false' }}
          SSH_PRIVATE_KEY: ${{ secrets.DEPENDENT_REPOS_SSH_KEY }}
        run: |
          # Script to iterate dependents and create PRs
          ./scripts/create-dependent-prs.sh
```

**Files to create in each upstream repo:**

1. `.forgejo/workflows/notify-dependents.yml` - Main workflow
2. `dependents.yaml` - Configuration for downstream repos
3. `scripts/create-dependent-prs.sh` - PR creation logic

### Phase 2: Dependents Configuration

**File:** `dependents.yaml` (in upstream repo root)

**Format:**
```yaml
# dependents.yaml - Repositories that depend on this one
apiVersion: graft/v0

dependents:
  - name: graft
    repo: ssh://forgejo@platform-vm:2222/daniel/graft.git
    dep_name: graft-knowledge  # Name of this dep in their graft.yaml
    branch_prefix: deps/graft-knowledge

  # Future: easy to add more dependents
  # - name: another-project
  #   repo: ssh://forgejo@platform-vm:2222/daniel/another.git
  #   dep_name: graft-knowledge
```

**For meta-knowledge-base:**
```yaml
dependents:
  - name: graft-knowledge
    repo: ssh://forgejo@platform-vm:2222/daniel/graft-knowledge.git
    dep_name: meta-knowledge-base
    branch_prefix: deps/meta-knowledge-base
```

### Phase 3: PR Creation Script

**File:** `scripts/create-dependent-prs.sh`

**Logic:**
```bash
#!/bin/bash
set -euo pipefail

# Parse dependents.yaml and iterate
for dependent in $(yq -r '.dependents[]' dependents.yaml | jq -c '.'); do
    name=$(echo "$dependent" | jq -r '.name')
    repo=$(echo "$dependent" | jq -r '.repo')
    dep_name=$(echo "$dependent" | jq -r '.dep_name')
    branch_prefix=$(echo "$dependent" | jq -r '.branch_prefix')

    echo "Processing dependent: $name"

    # Clone downstream repo
    git clone "$repo" "/tmp/$name"
    cd "/tmp/$name"

    # Create branch
    branch_name="${branch_prefix}/update-${UPSTREAM_SHORT_SHA}"
    git checkout -b "$branch_name"

    # Update graft.yaml
    # Change: dep_name: "url#old_ref" → dep_name: "url#new_sha"
    yq -i ".deps.\"${dep_name}\" |= sub(\"#.*\"; \"#${UPSTREAM_SHA}\")" graft.yaml

    # Commit changes
    git add graft.yaml
    git commit -m "chore(deps): update ${dep_name} to ${UPSTREAM_SHORT_SHA}

Upstream commit: ${UPSTREAM_MESSAGE}
Upstream author: ${UPSTREAM_AUTHOR}

${CODER_WORKSPACE_URL:+Coder workspace: ${CODER_WORKSPACE_URL}}"

    # Push and create PR (using Forgejo API or git push with PR creation)
    if [ "$DRY_RUN" != "true" ]; then
        git push origin "$branch_name"
        # Create PR via API
        create_pr "$name" "$branch_name" "$dep_name"
    else
        echo "[DRY RUN] Would push branch and create PR"
    fi

    cd -
done
```

### Phase 4: PR Description Template

**PR Title:** `chore(deps): update {dep_name} to {short_sha}`

**PR Body:**
```markdown
## Summary

Updates `{dep_name}` dependency to latest commit from upstream.

**Upstream changes:**
- Commit: `{short_sha}` - {commit_message}
- Author: {author}
- Repository: {upstream_repo}

## What Changed

{upstream_changelog_excerpt}

## Test Plan

- [ ] Run `graft resolve` to fetch updated dependency
- [ ] Verify dependency works as expected
- [ ] Run existing tests

## Continuation

{if coder_workspace_url}
If additional implementation work is needed, continue in this Coder workspace:
**[Continue in Coder]({coder_workspace_url})**
{endif}

---
🤖 This PR was automatically created by the dependency update workflow.
```

### Phase 5: Testing Strategy

**Unit Tests (scripts/):**
```bash
# test-update-graft-yaml.sh
test_updates_ref_correctly() {
    # Create mock graft.yaml
    cat > /tmp/graft.yaml << 'EOF'
apiVersion: graft/v0
deps:
  graft-knowledge: "ssh://forgejo@platform-vm:2222/daniel/graft-knowledge.git#abc123"
EOF

    # Run update
    UPSTREAM_SHA="def456" dep_name="graft-knowledge" \
        ./scripts/update-graft-yaml.sh /tmp/graft.yaml

    # Assert
    grep -q "#def456" /tmp/graft.yaml || fail "Ref not updated"
}

test_preserves_other_deps() {
    # Create mock with multiple deps
    # Run update on one
    # Assert other deps unchanged
}

test_handles_missing_dep() {
    # graft.yaml doesn't have the dep we're trying to update
    # Should fail gracefully with clear error
}
```

**Integration Tests (CI workflow):**
```yaml
# .forgejo/workflows/test-notify-dependents.yml
name: Test Notify Dependents

on:
  pull_request:
    paths:
      - 'scripts/create-dependent-prs.sh'
      - 'dependents.yaml'
      - '.forgejo/workflows/notify-dependents.yml'

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Run unit tests
        run: ./scripts/test-*.sh

      - name: Dry run workflow
        run: |
          DRY_RUN=true \
          UPSTREAM_SHA=test123 \
          UPSTREAM_SHORT_SHA=test1 \
          UPSTREAM_MESSAGE="Test commit" \
          ./scripts/create-dependent-prs.sh
```

### Phase 6: Coder Workspace Integration

**Environment variables available in Coder:**
- `CODER_WORKSPACE_NAME` - e.g., `task-charming-ganguly-0947`
- `CODER_WORKSPACE_AGENT_NAME` - e.g., `main`
- Coder host from configuration

**Workflow URL construction:**
```bash
if [ -n "${CODER_WORKSPACE_NAME:-}" ]; then
    CODER_WORKSPACE_URL="http://${CODER_HOST}/@${CODER_USER}/${CODER_WORKSPACE_NAME}"
fi
```

**Passing to downstream PR:**
- Store as repository secret or workflow input
- Include in PR body when available

## Implementation Order

### Milestone 1: Core Infrastructure

1. **Create shared script library**
   - `scripts/lib/graft-yaml.sh` - Functions for parsing/updating graft.yaml
   - `scripts/lib/pr-api.sh` - Functions for Forgejo PR API
   - Unit tests for each function

2. **Implement graft-knowledge → graft notification**
   - Add `dependents.yaml` to graft-knowledge
   - Add `notify-dependents.yml` workflow to graft-knowledge
   - Add `create-dependent-prs.sh` script
   - Test with dry-run mode

3. **Test end-to-end**
   - Make test commit to graft-knowledge
   - Verify PR created in graft
   - Verify PR content and format

### Milestone 2: Full Chain

4. **Implement meta-knowledge-base → graft-knowledge notification**
   - Add `dependents.yaml` to meta-knowledge-base
   - Add `notify-dependents.yml` workflow
   - Test end-to-end

5. **Verify cascade works**
   - Update meta-knowledge-base
   - → Creates PR in graft-knowledge
   - → When merged, creates PR in graft

### Milestone 3: Coder Integration & Polish

6. **Add Coder workspace link support**
   - Configure secrets for Coder URL
   - Update PR template
   - Test link generation

7. **Error handling and edge cases**
   - Handle existing open PRs (update instead of duplicate)
   - Handle merge conflicts
   - Add failure notifications

8. **Documentation**
   - Document workflow in each repo
   - Add troubleshooting guide
   - Document how to add new dependents

## Configuration Summary

### In graft-knowledge (upstream of graft):

```
graft-knowledge/
├── .forgejo/
│   └── workflows/
│       ├── ci.yml                    # Existing
│       └── notify-dependents.yml     # NEW
├── dependents.yaml                   # NEW
├── scripts/
│   ├── create-dependent-prs.sh       # NEW
│   └── lib/
│       ├── graft-yaml.sh             # NEW
│       └── pr-api.sh                 # NEW
└── graft.yaml                        # Existing
```

### In meta-knowledge-base (upstream of graft-knowledge):

```
meta-knowledge-base/
├── .forgejo/
│   └── workflows/
│       └── notify-dependents.yml     # NEW
├── dependents.yaml                   # NEW
├── scripts/
│   └── create-dependent-prs.sh       # NEW (copy/symlink)
└── meta.yaml                         # Existing
```

## Risks & Mitigations

### Risk 1: Cascade Storms

**Risk:** Rapid upstream changes create many PRs before earlier ones merge

**Mitigation:**
- Check for existing open PR before creating new one
- Update existing PR branch instead of creating duplicate
- Optional: Add debounce delay for non-critical updates

### Risk 2: Breaking Changes Propagate Immediately

**Risk:** Breaking upstream change creates PR that would break downstream

**Mitigation:**
- PRs require manual merge (not auto-merge)
- CI runs on PRs to catch issues
- Consider: Only auto-PR for non-breaking changes (based on commit message convention)

### Risk 3: SSH Key Management

**Risk:** Workflow needs write access to downstream repos

**Mitigation:**
- Use deploy keys with limited scope
- Consider: Fine-grained access tokens
- Document key rotation process

### Risk 4: Forgejo API Compatibility

**Risk:** PR creation API differs from GitHub

**Mitigation:**
- Research Forgejo API docs
- Test API calls in isolation
- Consider: Use `tea` CLI tool for PR creation

## Open Questions

1. **Debouncing:** Should we wait before creating PRs to batch rapid changes?
   - Recommendation: Start without, add if needed

2. **Auto-merge for patches:** Should non-breaking updates auto-merge?
   - Recommendation: No, always require human review initially

3. **Changelog extraction:** How much upstream changelog to include in PR?
   - Recommendation: Just the commit message initially, expand if useful

4. **Shared action repository:** Should workflow logic live in a separate repo?
   - Recommendation: Start with copy in each repo, extract if duplication becomes burdensome

## Related Documents

- [Graft YAML Format Specification](../../graft-knowledge/docs/specification/graft-yaml-format.md)
- [Forgejo Actions Documentation](https://forgejo.org/docs/latest/user/actions/)
- [Coder Workspace Environment](https://coder.com/docs)

## Changelog

- **2026-01-05**: Initial draft (v1.0)
  - Designed push-based notification architecture
  - Defined configuration format for dependents
  - Outlined implementation phases
  - Identified risks and mitigations
