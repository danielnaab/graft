---
title: "Dependency Update Notification Specification"
date: 2026-01-05
status: draft
---

# Dependency Update Notification Specification

## Overview

This specification defines how Graft-aware repositories can automatically detect and propagate dependency updates. When an upstream dependency is updated, consuming repositories should be notified and offered the opportunity to update.

The goal is **zero-configuration for upstream repositories**—the dependency graph is already declared in `graft.yaml` files; this specification describes how to leverage that existing information for automated update propagation.

## Requirements

### Functional Requirements

1. **Automatic detection**: Consumers must be able to detect when upstream dependencies have new commits beyond their pinned ref
2. **PR creation**: When updates are detected, the system should create pull requests in consumer repositories with updated refs
3. **Workspace continuity**: PRs should include context for continuation if implementation work is needed (e.g., links to task workspaces)
4. **No upstream configuration**: Upstream repositories should not need to maintain lists of their consumers

### Non-Functional Requirements

1. **Fast execution**: Update checks should complete quickly without heavy build steps
2. **Minimal dependencies**: Implementation should rely on standard tools (shell, git, curl)
3. **Idempotent**: Running the check multiple times should not create duplicate PRs
4. **Extensible**: Easy to add new consumer repositories without central coordination

## Core Concepts

### Dependency Graph Inversion

The `graft.yaml` files in consumer repositories already declare the dependency graph:

```yaml
# In consumer repo (e.g., graft)
apiVersion: graft/v0
deps:
  graft-knowledge: "ssh://forgejo@platform-vm:2222/daniel/graft-knowledge.git#main"
```

To find all consumers of a given upstream:
1. Enumerate all repositories in the organization
2. Parse each repository's `graft.yaml`
3. Build inverted index: `upstream → [consumers]`

This approach requires no configuration in upstream repositories.

### Update Detection

An update is available when:
```
upstream_head != consumer_pinned_ref
```

Where:
- `upstream_head`: Current HEAD of the upstream repository's tracked branch
- `consumer_pinned_ref`: The ref (commit, tag, branch) specified in the consumer's `graft.yaml`

Detection uses `git ls-remote` for efficiency (no clone required):
```bash
git ls-remote <upstream-url> HEAD
```

### PR Content

Update PRs should include:

1. **Updated `graft.yaml`**: New ref pointing to upstream HEAD
2. **Summary**: Which dependency was updated and to what ref
3. **Upstream context**: Commit message(s) from upstream
4. **Continuation link**: URL to resume work if manual intervention needed

## Event Strategy

See [Decision 0006: Dependency Update Event Strategy](../decisions/decision-0006-dependency-update-events.md) for the rationale behind the chosen approach.

### Recommended: Organization-wide Event with Polling Fallback

1. **Primary trigger**: Organization-level webhook fires on any repository push
2. **Handler**: Central workflow receives event, identifies affected consumers, creates PRs
3. **Fallback**: Scheduled polling (e.g., hourly) catches any missed events

This provides near-real-time updates without requiring upstream repositories to know about their consumers.

### Alternative Strategies

Other valid strategies (see ADR for full analysis):

- **Pure polling**: Scheduled checks without webhooks (simpler, higher latency)
- **Per-consumer workflows**: Each consumer runs its own update check (decentralized)
- **Upstream notification**: Upstream maintains consumer list (rejected—violates zero-config requirement)

## Implementation Components

### Central Update Service

A dedicated repository (e.g., `graft-ci`) containing:

1. **Update check workflow**: Triggered by org events or schedule
2. **Dependency graph builder**: Scans org repos, parses `graft.yaml` files
3. **PR creator**: Creates/updates PRs in consumer repositories

### Workflow Inputs

When triggered by push event:
```yaml
inputs:
  pushed_repo: string    # Repository that was updated
  pushed_ref: string     # Branch/ref that was pushed to
  pushed_sha: string     # New HEAD commit
```

When triggered by schedule:
- No inputs; scan all repos for any outdated dependencies

### PR Creation

```yaml
# Pseudocode for PR content
title: "chore(deps): update {dep_name} to {short_sha}"

body: |
  ## Summary
  Updates `{dep_name}` dependency to latest upstream commit.

  **Upstream commit:** `{sha}` - {commit_message}
  **Author:** {author}

  ## Continuation
  {workspace_link if available}

  ---
  Automated by graft-ci
```

### PR Update and Lifecycle Strategy

#### Idempotency and PR Reuse

The dependency update automation MUST handle successive upstream commits intelligently to preserve work and reduce noise.

**When an upstream dependency has multiple commits while a PR is still open:**

1. **Detect existing PR**: Query for open PRs with branch matching `deps/update-{dep_name}-*`
2. **Check for stale PR**: Compare dependency version in main branch vs PR branch
   - If main branch has newer/equal version: Close PR as superseded, delete workspace (if applicable)
   - Rationale: Manual updates or other PRs have already brought in the dependency
3. **Check for workspace modifications**: Detect user/AI work in workspace (if using workspace-based execution)
   - Uncommitted changes in workspace
   - Additional commits beyond automation commits
   - If detected: Preserve workspace, add PR comment about new upstream version, do NOT force-push
   - Rationale: Respect LLM token investment and human effort
4. **Safe to update**: If workspace is clean (only automation commits) or no workspace in use:
   - Force-push updated branch with new commit
   - Update PR title/body to reflect latest upstream state
   - Add comment describing the new upstream commit
5. **Update metadata**: Ensure PR reflects latest upstream commit information

**Rationale:**
- Preserves human and LLM work investment
- Automatically cleans up superseded PRs
- Provides single source of truth when safe to update
- Reduces PR noise and notification fatigue
- Matches industry standard (Dependabot, Renovate behavior)
- Gives users control when manual intervention has occurred

**Example workflow:**
```
t0: upstream/main @ abc123
t1: PR #1 created: "update graft-knowledge to abc123"
t2: upstream/main @ def456 (new commit while PR #1 is open)
t3: Automation detects existing PR #1
    - Checks main branch: still at old version
    - Checks workspace: clean (no user changes)
    - Safe to update: force-push to PR #1 branch
    - PR #1 title → "update graft-knowledge to def456"
    - Comment added: "Updated to include upstream commit def456"
t4: User manually merges different PR updating to ghi789
t5: Next automation run detects PR #1
    - Checks main branch: now has ghi789 (newer than PR's def456)
    - PR #1 is stale: close with comment, delete workspace
```

#### Multiple Dependency Updates

**When a consumer has updates available for multiple dependencies:**

1. **Separate PRs**: Create one PR per dependency (not bundled)
2. **Independent review**: Each PR can be reviewed and merged independently
3. **Atomic changes**: Each PR represents a single logical change
4. **Sequential processing**: Process one dependency at a time per consumer to avoid conflicts

**Rationale:**
- Isolated changes are easier to review and test
- Can merge updates at different cadences based on risk/priority
- Clear rollback path if one dependency causes issues
- Matches graft's atomic upgrade principle

**Example:**
```
Consumer: graft-ci
Available updates:
  - graft-knowledge: old_sha → new_sha
  - meta-knowledge-base: old_sha → new_sha

Result:
  - PR #1: "chore(deps): update graft-knowledge to {sha}"
  - PR #2: "chore(deps): update meta-knowledge-base to {sha}"

Both PRs are independent and can be merged in any order.
```

#### Conflict Resolution

**Concurrent updates to same dependency:**

If a workspace is currently running an upgrade when a new update is detected:

1. **Skip new update**: Do not create concurrent upgrades for same consumer+dep pair
2. **Retry later**: Next scheduled run will pick up the latest commit
3. **State tracking**: Implementation should track workspace state during upgrade execution

**Overlapping dependency changes:**

If updating multiple dependencies that might interact:

1. **Sequential processing**: Process one dependency at a time per consumer
2. **Workspace isolation**: Each dependency gets its own workspace (no file conflicts during execution)
3. **Integration testing**: Rely on consumer's CI to test combined effect of all updates after merge

**Future consideration**: Detect file-level conflicts using graft's migration metadata and intelligently batch non-conflicting updates.

### Workspace Integration

#### Workspace-Based Upgrade Execution

The dependency update automation SHOULD use isolated workspaces for upgrade execution to enable debugging and manual intervention:

**Workspace benefits:**
1. **Debuggability**: User can connect to workspace if upgrade fails or needs adjustment
2. **Continuity**: Workspace persists for additional manual work after automation
3. **Isolation**: Each upgrade runs in clean, dedicated environment
4. **Traceability**: Workspace URL included in PR for easy access

#### Workspace Lifecycle

**Creation:**
- Workspace created for each consumer+dependency pair
- Naming pattern: `{consumer}-deps-{dep_name}`
  - Example: `graft-ci-deps-graft-knowledge`
- Parameters: consumer repo URL, git credentials, Forgejo/Git server access
- Template: Lightweight template optimized for dependency upgrades

**Reuse:**
- Check if workspace exists before creating new one
- Restart stopped workspace rather than creating duplicate
- Pull latest consumer code before re-running upgrade
- Verify workspace state (clean vs modified) before force-pushing updates

**Cleanup:**
- Auto-stop after configurable timeout (e.g., 24 hours) to save resources
- Delete when associated PR is merged or closed
- Manual cleanup available via workspace management UI

#### Workspace Identification in PRs

Pull request descriptions MUST include workspace information when applicable:
- Workspace name for easy identification
- Workspace URL for manual access
- Upgrade execution status (migration/verification results)
- Instructions for connecting to workspace

**Example PR body with workspace:**
```markdown
## Summary
Updates `graft-knowledge` dependency to latest upstream commit.

**Upstream commit:** `abc1234` - Add new feature
**Author:** John Doe

## Workspace

Upgrade performed in workspace: [graft-ci-deps-graft-knowledge](http://coder.example.com/workspaces/graft-ci-deps-graft-knowledge)

**Status:**
✓ Graft upgrade completed
✓ Migration executed successfully
✓ Verification passed

The workspace is available for debugging or additional changes. Connect with:
```
coder ssh graft-ci-deps-graft-knowledge
```

---
Automated by graft-ci
```

## Integration with graft.yaml

### Current Format Support

The specification works with the existing `graft.yaml` format:

```yaml
apiVersion: graft/v0
deps:
  dep-name: "git-url#ref"
```

The `#ref` portion is updated to the new commit SHA.

### Extended Format (Future)

Future versions may support additional metadata:

```yaml
apiVersion: graft/v1
deps:
  dep-name:
    source: "git-url"
    ref: "main"
    auto_update: true        # Opt-in to automatic updates
    update_strategy: "pr"    # pr | auto-merge | notify-only
```

## CLI Integration

### graft check-updates

Query operation to check for available updates:

```bash
graft check-updates [options]

Options:
  --json           Output as JSON
  --create-pr      Create PR if updates available (requires git remote access)
  --dry-run        Show what would be done without making changes
```

**Output:**
```
Checking dependencies...
  graft-knowledge: abc123 → def456 (3 commits behind)
  meta-kb: up to date

1 update available. Run with --create-pr to create pull request.
```

### graft update

Mutation operation to update dependencies locally:

```bash
graft update [dep-name] [options]

Options:
  --all            Update all dependencies
  --to <ref>       Update to specific ref (default: upstream HEAD)
```

## Security Considerations

1. **Write access**: Central service needs write access to consumer repositories
2. **Credential management**: Use deploy keys or fine-grained tokens with minimal scope
3. **Branch protection**: PRs should go through normal review process, not direct push
4. **Webhook validation**: Verify webhook signatures to prevent spoofing

## Related

- [Decision 0006: Dependency Update Event Strategy](../decisions/decision-0006-dependency-update-events.md)
- [Specification: graft.yaml Format](./graft-yaml-format.md)
- [Specification: Core Operations](./core-operations.md)

## References

- Forgejo Actions documentation: https://forgejo.org/docs/latest/user/actions/
- Git ls-remote: https://git-scm.com/docs/git-ls-remote
