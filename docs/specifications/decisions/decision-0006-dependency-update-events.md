---
title: "Dependency Update Event Strategy"
date: 2026-01-05
status: accepted
---

# Dependency Update Event Strategy

## Context

Graft needs a mechanism to automatically detect when upstream dependencies have been updated and notify consuming repositories. This enables keeping dependency chains current without manual intervention.

Key constraints:
1. **Zero upstream configuration**: Upstream repos should not need to know about or list their consumers
2. **Fast execution**: Checks should run quickly without heavy build steps
3. **Near real-time**: Updates should propagate promptly when changes occur
4. **Resilient**: System should handle missed events gracefully

The dependency graph is already declared in `graft.yaml` files—we need to decide how to trigger checks and propagate updates.

## Decision

**Use organization-wide push events as the primary trigger, with scheduled polling as a fallback.**

When any repository in the organization is pushed to:
1. Organization webhook fires
2. Central handler identifies which repos consume the pushed repo (by parsing `graft.yaml` files)
3. For each consumer with outdated refs, create/update a PR

Additionally, run a scheduled check (e.g., hourly) to catch any events missed due to webhook failures.

## Alternatives Considered

### Alternative 1: Upstream Maintains Consumer List

**Approach**: Each upstream repo maintains a `dependents.yaml` listing its consumers. On push, upstream triggers updates in listed repos.

```yaml
# In upstream repo
dependents:
  - repo: graft
    dep_name: graft-knowledge
```

**Pros**:
- Explicit, easy to understand
- Direct push notification
- No scanning required

**Cons**:
- Violates zero-config requirement
- Creates coupling—upstream must know about all consumers
- Maintenance burden—list must be kept in sync
- Doesn't scale—adding consumers requires upstream changes

**Why rejected**: Violates the core requirement that upstream repos need no configuration. The dependency relationship is already declared in consumer `graft.yaml`; duplicating it in upstream creates maintenance burden and coupling.

### Alternative 2: Pure Polling (Scheduled Only)

**Approach**: Each consumer or a central service periodically checks all dependencies for updates.

```yaml
on:
  schedule:
    - cron: '0 * * * *'  # hourly
```

**Pros**:
- Simple to implement
- No webhook configuration needed
- Works with any git remote

**Cons**:
- Delayed updates (up to polling interval)
- Wastes resources checking when nothing changed
- Higher latency for urgent updates

**Why not primary**: Acceptable as fallback, but polling-only means updates are always delayed. Combined with event-driven trigger provides best of both.

### Alternative 3: Per-Consumer Workflows

**Approach**: Each consumer repo runs its own scheduled check workflow.

```yaml
# In each consumer repo
on:
  schedule:
    - cron: '0 6 * * *'  # daily
```

**Pros**:
- Fully decentralized
- No central service needed
- Consumer controls their update frequency

**Cons**:
- Requires workflow in every consumer repo
- Not zero-config for consumers
- Inconsistent timing across repos
- Harder to coordinate

**Why not chosen**: Adds configuration burden to every consumer. Central service achieves same result with single point of configuration.

### Alternative 4: Git Server Hooks

**Approach**: Configure server-side post-receive hooks to trigger updates.

**Pros**:
- Most direct trigger
- No webhook latency

**Cons**:
- Requires server admin access
- Not portable across platforms
- Harder to version control

**Why rejected**: Too tightly coupled to specific server infrastructure. Org webhooks achieve similar result without server access.

### Alternative 5: Watch/Subscribe Model

**Approach**: Consumers explicitly subscribe to upstream repos via API.

**Pros**:
- Explicit intent
- Could support cross-organization

**Cons**:
- Requires subscription management
- Not currently supported by Forgejo/GitHub as first-class feature
- Additional state to maintain

**Why rejected**: Over-engineering for the use case. Org webhook + graph inversion achieves the goal without new subscription infrastructure.

## Consequences

### Positive

- **Zero config for upstreams**: No changes needed in dependency repositories
- **Near real-time**: Push events trigger immediate checks
- **Resilient**: Scheduled fallback catches missed webhooks
- **Centralized logic**: Single service handles all update propagation
- **Leverages existing data**: Uses `graft.yaml` as source of truth

### Negative

- **Central service required**: Need to maintain `graft-ci` or equivalent
- **Org webhook setup**: One-time configuration in org settings
- **Write access**: Central service needs push access to consumer repos
- **Latency for cross-org**: Only works within single organization

### Mitigations

- **Central service**: Small, focused scope—just dependency checking
- **Org webhook**: One-time setup, well-documented
- **Write access**: Use deploy keys with minimal scope
- **Cross-org**: Future enhancement; document limitation

## Implementation Notes

### Org Webhook Configuration

```
Forgejo Org Settings → Webhooks → Add Webhook
  URL: <central-service-endpoint>
  Events: Push
  Content-Type: application/json
```

### Graph Building Efficiency

Cache the inverted dependency graph:
```python
# Build once, update incrementally
graph = {}  # upstream_repo → [consumer_repos]

for repo in org_repos:
    graft_yaml = fetch_graft_yaml(repo)
    for dep_name, dep_url in graft_yaml.deps.items():
        upstream = extract_repo_name(dep_url)
        graph.setdefault(upstream, []).append(repo)
```

Invalidate cache entry when a repo's `graft.yaml` changes.

### Fast Execution

Use shell script with minimal dependencies:
- `curl` for API calls
- `git ls-remote` for ref checking (no clone)
- `yq` for YAML parsing (single static binary)

Total execution: typically <5 seconds for small organizations.

## Related

- [Specification: Dependency Update Notification](../graft/dependency-update-notification.md)
- [Decision 0005: No Partial Resolution](./decision-0005-no-partial-resolution.md)
- Forgejo webhooks: https://forgejo.org/docs/latest/user/webhooks/

## References

- Organization webhooks: Common pattern in GitHub/GitLab/Forgejo for cross-repo automation
- Event-driven architecture: https://martinfowler.com/articles/201701-event-driven.html
