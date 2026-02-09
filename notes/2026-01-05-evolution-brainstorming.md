---
title: "Brainstorming: Graft Evolution Directions"
date: 2026-01-05
status: working
participants: ["human", "agent"]
tags: [brainstorming, architecture, vision, roadmap]
---

# Brainstorming: Graft Evolution Directions

## Context

This document captures extensive brainstorming about Graft's future evolution. The goal is to explore elegant, simple abstractions that are powerful and expressive, providing excellent UX from every angle while maintaining the core philosophy.

**Guiding Principles** (from [architecture](../docs/specifications/architecture.md)):
1. Git-Native
2. Explicit Over Implicit
3. Minimal Primitives
4. Separation of Concerns
5. Atomic Operations
6. Composability

## Vision Themes

Seven interconnected themes for Graft's evolution:

1. [Commands as Transactions](#1-commands-as-transactions)
2. [Web UI for Graft Repositories](#2-web-ui-for-graft-repositories)
3. [Dependency Upgrade Affordances](#3-dependency-upgrade-affordances)
4. [Structured Transactions with Rich Output](#4-structured-transactions-with-rich-output)
5. [Multiple Interfaces](#5-multiple-interfaces)
6. [Agent-Driven Development Philosophy](#6-agent-driven-development-philosophy)
7. [Ecosystem of Composable Components](#7-ecosystem-of-composable-components)

---

## 1. Commands as Transactions

### Insight

Graft commands are already transaction-like (atomic upgrades with rollback). This could be formalized into a richer **Transaction Model** that enables auditability, visualization, and orchestration.

### Proposed Abstraction

```yaml
# Transaction record (stored in .graft/transactions/ or as git notes)
transaction:
  id: "tx-2026-01-05-abc123"
  type: "upgrade"
  timestamp: "2026-01-05T14:30:00Z"

  input_state:
    lock_commit: "abc123..."
    working_tree_dirty: false
    deps_state:
      meta-kb: { ref: "v1.5.0", commit: "def456..." }

  operation:
    command: "upgrade"
    args: ["meta-kb", "--to", "v2.0.0"]
    env: { GRAFT_DRY_RUN: "false" }

  output_state:
    lock_commit: "ghi789..."
    files_changed: ["src/api.ts", "src/utils.ts"]
    migration_output: "Modified 15 files"

  metadata:
    agent_id: "claude-code-session-xyz"  # Optional
    user: "developer@org.com"             # Optional
    duration_ms: 4532

  outcome: "success"  # or "failure", "rollback"
```

### Benefits

- **Auditability**: Full history of what changed, when, why, by whom
- **Debuggability**: Trace back through transactions when something breaks
- **Visualization**: Show transaction log in CLI or web UI
- **Agent Orchestration**: Agents can plan and replay transaction sequences
- **Reproducibility**: Replay transactions on different machines/branches

### Related Concepts

| Concept | Description |
|---------|-------------|
| Transaction Log | Append-only log of all Graft operations |
| Checkpoints | Named save points for rollback |
| Transaction Groups | Batch multiple operations atomically |
| Dry-Run Transactions | Preview without execution |

### Metaphor Alignment

This fits the "grafting" metaphor - each graft operation is like a surgical procedure with documented before/after states, creating a medical record of the repository's evolution.

### Open Questions

1. Where should transaction logs be stored? (`.graft/transactions/`, git notes, separate db?)
2. Should transactions be git-committed automatically?
3. How long should transaction history be retained?
4. Should failed transactions be logged differently than successes?

---

## 2. Web UI for Graft Repositories

### Vision

An intuitive web interface that:
- Visualizes dependency graphs and transaction history
- Links naturally from PRs for change review
- Supports domain-specific UI injection for non-technical users
- Enables editing structured configuration through forms/visual tools

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Graft Web UI                             │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │  Repository │  │ Transaction │  │   Domain    │          │
│  │   Browser   │  │     Log     │  │   Editor    │          │
│  └─────────────┘  └─────────────┘  └─────────────┘          │
│         ↓                ↓                ↓                  │
│  ┌─────────────────────────────────────────────────────────┐│
│  │              Graft API Server                            ││
│  │  - REST/GraphQL for queries                              ││
│  │  - WebSocket for live updates                            ││
│  │  - Plugin system for domain UIs                          ││
│  └─────────────────────────────────────────────────────────┘│
│         ↓                ↓                ↓                  │
│  ┌─────────────────────────────────────────────────────────┐│
│  │              Git Repository + Graft State                ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

### Core Capabilities

#### 2.1 Repository Browser
- Visualize dependency graph (tree view, network diagram)
- Show upstream/downstream relationships
- Highlight available upgrades, breaking changes
- Navigate through dependency contents

#### 2.2 Transaction Log View
- Timeline of all Graft operations
- Filter by type, date, author, outcome
- Link to git commits and diffs
- Show migration output, verification results
- Diff view between transaction states

#### 2.3 PR Integration
- Auto-generate PR comments showing Graft state changes
- Deep links to web UI for detailed exploration
- GitHub Actions / GitLab CI integration
- Status checks based on Graft validation

#### 2.4 Domain-Specific Editors (Plugin System)

For non-technical domain experts, inject custom UIs:

```yaml
# In graft.yaml
plugins:
  recipe-editor:
    source: "git@github.com:org/graft-plugin-recipes"
    version: "v1.0.0"
    entrypoint: "web/index.js"
    handles:
      files: ["recipes/**/*.yaml"]
      contentType: "application/x-recipe"
    permissions:
      read: ["recipes/**"]
      write: ["recipes/**"]
```

**Plugin Examples**:
- Recipe editor: Form-based editing for culinary knowledge base
- Infrastructure editor: Visual diagram for infrastructure-as-code
- Policy editor: Guided workflow for compliance rules
- Schedule editor: Calendar-based editing for time-based configs

### Technical Considerations

- **Hosting**: Self-hosted or SaaS? Consider both models
- **Authentication**: OAuth with GitHub/GitLab, or org SSO
- **Real-time**: WebSocket for live collaboration
- **Offline**: PWA support for disconnected editing
- **Mobile**: Responsive design for on-the-go review

### Metaphor Connection

The web UI is the "greenhouse" where you observe and tend grafted plants - seeing their health, history, and guiding growth through visual tools.

---

## 3. Dependency Upgrade Affordances

### Goal

Make upgrades easy - from fully automated to agent-assisted to manual, with appropriate guardrails at each level.

### Upgrade Intelligence Levels

| Level | Name | Description | Use Case |
|-------|------|-------------|----------|
| 0 | Manual | User runs explicit commands | Full control needed |
| 1 | Prompted | Graft suggests, user confirms | Regular maintenance |
| 2 | Agent-Assisted | Agent proposes, human reviews PR | Busy teams |
| 3 | Fully Automated | CI/CD pipeline handles everything | Trusted deps |

### Level 1: Prompted Upgrades

```bash
$ graft status
meta-kb: v1.5.0 → v2.0.0 available
  Type: breaking
  Migration: migrate-v2 (automatic)

  Run: graft upgrade meta-kb --to v2.0.0
  Or:  graft upgrade meta-kb --to v2.0.0 --dry-run (preview)
```

### Level 2: Agent-Upgrade Protocol

Structured output for AI assistants:

```yaml
# Output of: graft upgrade-plan meta-kb --to v2.0.0 --format yaml
plan:
  dependency: meta-kb
  current: v1.5.0
  target: v2.0.0

  changes:
    - ref: v1.6.0
      type: feature
      description: "Added caching support"
      action: "none required"

    - ref: v2.0.0
      type: breaking
      description: "Renamed getUserData → fetchUserData"
      action: "run migration"
      migration:
        command: migrate-v2
        estimated_impact:
          files_affected: ["src/**/*.ts"]
          pattern: "getUserData\\("
          occurrences: 15

  verification:
    - command: verify-v2
      description: "Run test suite"

  rollback_strategy:
    snapshot: true
    git_revert: "graft.lock changes can be reverted"

  agent_hints:
    context_files:
      - "CHANGELOG.md"
      - "docs/migration-v2.md"
    success_criteria:
      - "all tests pass"
      - "no TypeScript errors"
      - "no runtime errors in logs"
```

### Level 3: Fully Automated (CI/CD)

```yaml
# .github/workflows/graft-upgrades.yml
name: Graft Auto-Upgrades
on:
  schedule:
    - cron: '0 9 * * 1'  # Weekly Monday 9am

jobs:
  check-upgrades:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Check for Graft upgrades
        id: upgrades
        run: graft status --format json > upgrades.json

      - name: Apply non-breaking upgrades
        if: steps.upgrades.outputs.has_non_breaking
        run: |
          graft upgrade --all --type feature,fix
          graft upgrade --all --type feature,fix --apply

      - name: Create PR for breaking changes
        if: steps.upgrades.outputs.has_breaking
        run: |
          graft upgrade --all --type breaking --dry-run > breaking.md
          gh pr create --title "Breaking dependency upgrades available" \
                       --body-file breaking.md
```

### Grafting Metaphor Reinforcement

- Upgrades = propagating new growth from parent plant
- DNA (patterns, rules, interfaces) evolves upstream
- Consumers adopt genetic improvements through grafting
- Migration = the careful surgical technique to integrate new material
- Verification = checking that the graft has taken successfully

---

## 4. Structured Transactions with Rich Output

### Insight

Commands could be multi-modal operations producing various outputs - prompts for decision-making, documents for review, interactive sessions for complex operations.

### Command Type Taxonomy

| Type | Purpose | Example |
|------|---------|---------|
| Mutation | Changes files | `migrate-v2` |
| Prompt | Generates decision info | `analyze-upgrade` |
| Document | Produces artifacts | `generate-changelog` |
| Interactive | Requires input during run | `guided-setup` |
| Composite | Chains commands conditionally | `full-upgrade-workflow` |

### Extended Command Schema

```yaml
commands:
  # Type: Prompt - generates analysis for decision-making
  analyze-upgrade:
    type: prompt
    run: "scripts/generate-upgrade-analysis.sh"
    output:
      format: markdown
      sections:
        - "Impact Analysis"
        - "Recommended Actions"
        - "Risk Assessment"
      agent_hint: "Review this before running migrate-v2"

  # Type: Mutation - modifies files (current behavior)
  migrate-v2:
    type: mutation
    run: "npx jscodeshift -t codemods/v2.js"
    pre_check: analyze-upgrade   # Run prompt first
    post_check: verify-v2        # Verify after

  # Type: Document - produces artifacts
  generate-changelog:
    type: document
    run: "scripts/generate-changelog.sh"
    output:
      format: markdown
      file: "CHANGELOG-generated.md"

  # Type: Interactive - requires human/agent input
  guided-setup:
    type: interactive
    run: "scripts/guided-setup.sh"
    prompts:
      - id: auth_method
        question: "Which auth method?"
        options: ["oauth2", "api_key", "none"]
      - id: database
        question: "Which database?"
        options: ["postgres", "sqlite", "none"]

  # Type: Composite - chains commands
  full-upgrade-workflow:
    type: composite
    steps:
      - run: analyze-upgrade
        continue_on: always
      - ask: "Proceed with upgrade?"
        default: yes
      - run: migrate-v2
        continue_on: success
      - run: verify-v2
        continue_on: success
      - run: generate-changelog
        continue_on: success
```

### Higher-Order Tools Built on This

| Tool | Description |
|------|-------------|
| Upgrade Advisors | Analyze impact before mutation |
| Change Reporters | Generate PR descriptions automatically |
| Compliance Checkers | Verify changes meet organizational rules |
| Onboarding Generators | Create docs for new team members |
| Impact Analyzers | Show blast radius of changes |

---

## 5. Multiple Interfaces

### Goal

Expose Graft through various interfaces to maximize utility across contexts.

### Interface Architecture

```
                    ┌─────────────────────┐
                    │   Graft Core API    │
                    │   (Python/Rust)     │
                    └──────────┬──────────┘
                               │
        ┌──────────────────────┼──────────────────────┐
        │                      │                      │
        ▼                      ▼                      ▼
┌───────────────┐    ┌───────────────┐    ┌───────────────┐
│     CLI       │    │   HTTP API    │    │   Language    │
│ (graft ...)   │    │  (REST/gRPC)  │    │   Bindings    │
└───────────────┘    └───────────────┘    └───────────────┘
        │                    │                    │
        ▼                    ▼                    ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│    Human      │   │    Web UI     │   │   Python/     │
│   Terminal    │   │   GitHub App  │   │   Node.js     │
│   Scripts     │   │   CI/CD       │   │   Libraries   │
└───────────────┘   └───────────────┘   └───────────────┘
```

### Interface Catalog

#### 5.1 CLI (Current)
- Direct terminal use
- Shell scripts
- CI/CD pipelines

#### 5.2 HTTP API
- REST for CRUD operations on state
- WebSocket for streaming output
- GraphQL for complex queries
- OpenAPI spec for client generation

#### 5.3 GitHub/GitLab App
- PR comments with upgrade status
- Status checks based on validation
- Issue creation on upgrade failures
- Automated PR creation

#### 5.4 Language SDKs
```python
# Python: graft-py
from graft import GraftClient

client = GraftClient("/path/to/repo")
status = client.status()
for dep in status.upgrades_available:
    print(f"{dep.name}: {dep.current} → {dep.available}")

result = client.upgrade("meta-kb", to="v2.0.0", dry_run=True)
```

```javascript
// Node.js: @graft/client
import { GraftClient } from '@graft/client';

const client = new GraftClient('/path/to/repo');
const status = await client.status();
const changes = await client.changes('meta-kb');
```

#### 5.5 MCP Server (Model Context Protocol)

```python
# Graft as MCP server for AI assistants
from graft.mcp import GraftMCPServer

server = GraftMCPServer()

@server.tool("graft_status")
def get_status(repo_path: str) -> dict:
    """Get current graft dependency status"""
    return graft_core.status(repo_path)

@server.tool("graft_upgrade")
def upgrade_dependency(
    dep: str,
    target_ref: str,
    dry_run: bool = True
) -> dict:
    """Upgrade a dependency with optional dry-run"""
    return graft_core.upgrade(dep, target_ref, dry_run=dry_run)

@server.tool("graft_changes")
def list_changes(dep: str, since: str = None) -> list:
    """List available changes for a dependency"""
    return graft_core.changes(dep, since=since)
```

#### 5.6 VS Code Extension
- Visual dependency explorer in sidebar
- Upgrade notifications in editor
- Run commands from command palette
- Inline diff preview for changes

---

## 6. Agent-Driven Development Philosophy

### Core Philosophy

Agents excel at exploration and problem-solving, but should crystallize their learnings into disciplined software that follows traditional best practices.

### Structured Emergence Pattern

```
┌─────────────────────────────────────────────────────────────┐
│                    Agent-Driven Development                  │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   Exploration Phase        Crystallization Phase             │
│   (Agent-Led)              (Code-Led)                        │
│                                                              │
│   ┌───────────────┐        ┌───────────────┐                 │
│   │  Investigate  │   →    │    Codify     │                 │
│   │  Problem      │        │   Patterns    │                 │
│   │  Domain       │        │   in Code     │                 │
│   └───────────────┘        └───────────────┘                 │
│          │                        │                          │
│          ▼                        ▼                          │
│   ┌───────────────┐        ┌───────────────┐                 │
│   │  Experiment   │   →    │   Document    │                 │
│   │  with         │        │   Rules &     │                 │
│   │  Approaches   │        │   Constraints │                 │
│   └───────────────┘        └───────────────┘                 │
│          │                        │                          │
│          ▼                        ▼                          │
│   ┌───────────────┐        ┌───────────────┐                 │
│   │  Validate     │   →    │   Automate    │                 │
│   │  with         │        │   via Graft   │                 │
│   │  Users        │        │   Commands    │                 │
│   └───────────────┘        └───────────────┘                 │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Key Principles

#### 6.1 Configuration Over Conversation

Agent insights should crystallize into configuration:

```yaml
# Bad: Agent knows how to set up auth, but it's in chat history
# Good: Agent encodes knowledge into configuration

# graft.yaml - captured expertise
commands:
  setup-auth:
    run: "scripts/setup-auth.sh"
    description: |
      Configure OAuth2 authentication.
      Learned from initial setup session 2026-01-05.
      See docs/decisions/decision-0010-auth-approach.md

# Future agents just run: graft :setup-auth
```

#### 6.2 Schema Evolution as Learning

As agents explore, schemas should tighten:

```yaml
# Week 1: Loose schema (exploring)
config:
  database: any  # Accepts anything

# Week 4: Tighter schema (learned constraints)
config:
  database:
    type: "postgres" | "sqlite"
    version: ">=14.0"
    # Constraint learned: MySQL had compatibility issues
```

#### 6.3 Commands as Captured Expertise

When an agent figures out how to do something, encode it:

```yaml
# Before: Agent manually figures out deployment each time
# After: Expertise captured in command

commands:
  deploy-staging:
    run: "./scripts/deploy.sh staging"
    description: |
      Deploy to staging environment.
      Requirements:
      - AWS credentials configured
      - Docker daemon running
      - Valid kubeconfig

      Troubleshooting:
      - If pod fails: check resource limits
      - If timeout: check VPN connection
```

#### 6.4 Documentation as Context

Agent-generated docs live alongside code for future context:

```markdown
<!-- docs/decisions/decision-0010-auth-approach.md -->
# Decision: Use OAuth2 for Authentication

## Context
During initial setup (2026-01-05), we evaluated authentication options.

## Options Considered
1. API Keys - Simple but less secure
2. OAuth2 - Standard, secure, supports SSO
3. SAML - Complex, enterprise-focused

## Decision
OAuth2 with PKCE flow.

## Consequences
- Requires OAuth provider configuration
- Enables SSO integration in future
- Adds token refresh complexity

## Agent Notes
When implementing new auth features, prefer extending OAuth2 flow
rather than adding parallel auth methods.
```

### How Graft Enables This

| Capability | How Graft Helps |
|------------|-----------------|
| Capture expertise | Commands encode repeatable operations |
| Track evolution | Changes document how understanding evolved |
| Share learning | Dependencies propagate patterns across projects |
| Maintain discipline | Migrations enforce consistency |
| Enable automation | Once captured, operations run deterministically |

---

## 7. Ecosystem of Composable Components

### Vision

An ecosystem of shareable, composable knowledge and patterns that enable rapid application development and configuration.

### Ecosystem Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Graft Ecosystem                           │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐      │
│  │  Core       │    │  Domain     │    │  Community  │      │
│  │  Components │    │  Patterns   │    │  Plugins    │      │
│  └─────────────┘    └─────────────┘    └─────────────┘      │
│        │                  │                  │               │
│        └──────────────────┴──────────────────┘               │
│                           │                                  │
│                    ┌──────▼──────┐                          │
│                    │   Registry  │                          │
│                    │  (Optional) │                          │
│                    └─────────────┘                          │
│                           │                                  │
│         ┌─────────────────┼─────────────────┐               │
│         ▼                 ▼                 ▼               │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐        │
│  │  Project A  │   │  Project B  │   │  Project C  │        │
│  └─────────────┘   └─────────────┘   └─────────────┘        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Component Types

#### 7.1 Knowledge Bases
| Component | Purpose |
|-----------|---------|
| meta-knowledge-base | Documentation patterns and conventions |
| coding-standards-kb | Linting rules, style guides, best practices |
| security-kb | Security patterns, threat models, compliance |
| api-design-kb | REST/GraphQL design patterns |

#### 7.2 Domain Templates
| Component | Purpose |
|-----------|---------|
| web-app-template | Full stack application starter |
| api-service-template | REST API service patterns |
| ml-pipeline-template | ML workflow patterns |
| cli-tool-template | Command-line tool patterns |

#### 7.3 Operational Patterns
| Component | Purpose |
|-----------|---------|
| ci-cd-patterns | GitHub Actions, GitLab CI configurations |
| deployment-patterns | K8s, Docker, serverless patterns |
| monitoring-patterns | Observability, alerting setup |
| backup-patterns | Data protection, disaster recovery |

#### 7.4 Tooling Plugins
| Component | Purpose |
|-----------|---------|
| graft-plugin-web-ui | Web dashboard for Graft |
| graft-plugin-pr-bot | GitHub/GitLab integration |
| graft-plugin-mcp | AI assistant integration |
| graft-plugin-vscode | VS Code extension |

### Composition Example

```yaml
# A new project's graft.yaml
deps:
  # Foundation
  meta-kb: "git@github.com:org/meta-knowledge-base#v2.0"
  coding-standards: "git@github.com:org/coding-standards-kb#v1.5"

  # Domain template
  web-template: "git@github.com:org/web-app-template#v3.0"

  # Operational patterns
  ci-patterns: "git@github.com:org/ci-cd-patterns#v2.1"
  monitoring: "git@github.com:org/monitoring-patterns#v1.3"

  # Tooling
  graft-web-ui: "git@github.com:org/graft-plugin-web-ui#v1.0"

# Result: New project gets
# - Documentation conventions
# - Coding standards with linting
# - Full web app structure
# - CI/CD already configured
# - Monitoring pre-wired
# - Web UI for managing it all
```

### Future: Registry/Marketplace Concepts

| Feature | Description |
|---------|-------------|
| Discovery | Search by domain, tags, popularity |
| Trust | Verified publishers, security scans |
| Compatibility | Version compatibility matrix |
| Metrics | Downloads, usage, update frequency |
| Reviews | Community feedback, quality ratings |

### Composability Principles

1. **Minimal coupling**: Components should work independently
2. **Clear contracts**: Explicit interfaces between components
3. **Graceful degradation**: Missing optional deps shouldn't break things
4. **Semantic versioning**: Breaking changes clearly signaled
5. **Migration paths**: Upgrade from old to new versions smoothly

---

## Naming and Metaphor Analysis

### Current Terminology Assessment

| Term | Horticultural Fit | Software Fit | Assessment |
|------|------------------|--------------|------------|
| Graft | Excellent | Moderate | Keep - core identity |
| Dependency | Weak | Excellent | Keep - universal |
| Change | Neutral | Excellent | Keep - clear |
| Upgrade | Weak | Excellent | Consider enriching |
| Migration | Weak | Excellent | Consider enriching |
| Lock file | None | Excellent | Keep - too universal |

### Potential Horticultural Enrichments

| Current | Alternative | Metaphorical Meaning |
|---------|-------------|---------------------|
| Upgrade | Propagate | Spread growth to consumers |
| Migration | Transplant | Move to new environment |
| Change | Cultivar | A cultivated variety |
| Breaking change | Rootstock change | Fundamental regrafting |
| Feature | New growth | Non-breaking addition |
| Lock file | Garden plan | Record of plantings |
| Transaction | Grafting session | Surgical operation |

### Recommendation

The hybrid terminology (horticultural core + software operations) is a strength. It's memorable while remaining accessible. Consider:

1. Keep core terms (graft, dependency, change)
2. Use horticultural language in documentation ("propagate improvements")
3. Optionally add aliases for those who prefer the metaphor
4. Ensure core UX uses familiar software terms for accessibility

---

## Implementation Prioritization

### Phase 1: Foundation (Near-term)
- [ ] Transaction logging infrastructure
- [ ] Structured upgrade-plan output
- [ ] MCP server skeleton
- [ ] Enhanced CLI status/changes output

### Phase 2: Interfaces (Medium-term)
- [ ] HTTP API server
- [ ] Web UI MVP (dependency browser, transaction log)
- [ ] GitHub App integration
- [ ] Python SDK

### Phase 3: Ecosystem (Longer-term)
- [ ] Plugin architecture
- [ ] Domain-specific editor framework
- [ ] Component registry
- [ ] Advanced agent tooling

---

## Open Questions for Future Sessions

1. **Transaction storage**: Git notes vs. file-based vs. external?
2. **Plugin sandboxing**: How to safely run third-party plugins?
3. **Multi-repo workflows**: How do workspace/monorepos fit?
4. **Offline-first**: How much should work without network?
5. **Enterprise features**: SSO, audit logs, compliance reports?
6. **Performance at scale**: 100+ dependencies, 1000+ transactions?

---

## Sources

- [Graft Architecture](../docs/specifications/architecture.md)
- [Upgrade Mechanisms Brainstorming](./2026-01-01-upgrade-mechanisms.md)
- [Design Improvements Analysis](./2026-01-05-design-improvements-analysis.md)
- [Decision 0004: Atomic Upgrades](../docs/specifications/decisions/decision-0004-atomic-upgrades.md)
- [Core Operations Specification](../docs/specifications/graft/core-operations.md)
- [Meta Knowledge Base Conventions](../.graft/meta-knowledge-base/docs/meta.md)
