---
title: "Brainstorming: User Interface Architecture for Graft"
date: 2026-01-07
status: working
participants: ["human", "agent"]
tags: [brainstorming, ui, architecture, web, interfaces, governance]
---

# Brainstorming: User Interface Architecture for Graft

## Context

This document explores how browser-based user interfaces could be built on top of Graft. The goal is to identify conceptual qualities, architectural patterns, and ecosystem components that would enable rich UI experiences while maintaining Graft's core principles.

**This is exploratory brainstorming** - no specification changes should result from this document until ideas are validated and refined.

**Guiding Principles** (from [architecture](../docs/specifications/architecture.md)):
1. Git-Native
2. Explicit Over Implicit
3. Minimal Primitives
4. Separation of Concerns
5. Atomic Operations
6. Composability

---

## Use Cases Under Consideration

The following use cases were identified as potential targets for browser-based UIs:

### UC1: Command Execution Visibility
**View lots of graft command executions**

Users need to see what commands have run, are running, or will run. This includes historical browsing, real-time monitoring, and output inspection.

### UC2: External Resource Connectivity
**Handy links to relevant external resources**

Examples: Coder Task workspaces, git pull requests, issues/tickets, source code, upstream graft dependencies. The UI should be a hub connecting Graft operations to the broader development ecosystem.

### UC3: Command Invocation
**Support invoking graft commands through the UI**

This requires a runtime environment - the UI itself isn't the execution environment. The UI would dispatch to some backend executor and stream results back.

### UC4: Domain-Specific Review Workflows
**PR-like workflows with domain-specific UIs**

Examples:
- View config schemas with nice UX
- Show diffs between one version and another
- Edit UI for structured data (forms, visual editors)
- Code diffs with syntax highlighting
- Embedded lightweight source code viewers

### UC5: Policy Enforcement
**Understand and enforce policy requirements on graft commands**

Examples: approval workflows (require human sign-off), sandboxing requirements (certain commands must run in isolated environments), resource limits, time windows.

### UC6: Audit Trail Interface
**Link command outputs to an audit log-like interface**

Who ran what, when, why, with what result. Compliance-oriented viewing, searchable history, exportable records.

---

## Part 1: Key Qualities and Conceptual Themes

Analyzing these use cases reveals several cross-cutting conceptual qualities that a Graft UI system should embody:

### Quality 1: Observability

**Definition**: The ability to understand what is happening, has happened, and will happen within Graft-managed systems.

**Manifestations across use cases**:
- UC1: Command execution history, status dashboards, output viewing
- UC6: Audit trails, compliance reporting

**Sub-qualities**:
| Sub-quality | Description |
|-------------|-------------|
| **Temporal depth** | See not just current state but history and trends |
| **Granularity control** | Zoom from high-level overview to detailed output |
| **Real-time awareness** | Live updates as things change |
| **Searchability** | Find specific events across large datasets |
| **Correlation** | Link related events (e.g., "this upgrade triggered these tests") |

**Design implications**:
- Need structured, queryable execution records (not just logs)
- Need event streaming infrastructure for real-time updates
- Need filtering/faceting across time, status, actor, type

---

### Quality 2: Connectivity

**Definition**: The ability to link Graft operations to the broader development ecosystem and vice versa.

**Manifestations across use cases**:
- UC2: Links to PRs, issues, workspaces, source code
- UC4: Embedded source viewers, diff linkage

**Sub-qualities**:
| Sub-quality | Description |
|-------------|-------------|
| **Outbound linking** | From Graft UI → external systems (GitHub, Coder, etc.) |
| **Inbound linking** | From external systems → Graft UI (deep links) |
| **Bidirectional context** | External tools can embed Graft status/actions |
| **Identity federation** | Consistent user identity across systems |

**Design implications**:
- Need stable, shareable URLs for all UI states
- Need a linking protocol/convention for external resources
- Need integration points (webhooks, embeds, APIs)
- Consider: URL schemes, deep link registry, embed widgets

---

### Quality 3: Actionability

**Definition**: The ability to take meaningful actions through the UI, not just view information.

**Manifestations across use cases**:
- UC3: Invoking graft commands
- UC4: Editing structured data
- UC5: Approval workflows (approve/reject/defer)

**Sub-qualities**:
| Sub-quality | Description |
|-------------|-------------|
| **Command dispatch** | Send commands to execution environments |
| **Structured editing** | Modify data through forms/visual tools |
| **Workflow participation** | Approve, reject, escalate, comment |
| **Bulk operations** | Act on multiple items at once |
| **Undo/rollback** | Recover from mistakes |

**Design implications**:
- UI is not the execution runtime - needs backend executor
- Need optimistic UI patterns with rollback
- Need command queuing and status tracking
- Consider: WebSocket connections, job queues, execution tokens

---

### Quality 4: Context Richness

**Definition**: The ability to present information in domain-appropriate ways that maximize comprehension and reduce cognitive load.

**Manifestations across use cases**:
- UC4: Schema-aware editing, semantic diffs, domain editors
- UC1: Structured output rendering (not just text dumps)

**Sub-qualities**:
| Sub-quality | Description |
|-------------|-------------|
| **Schema awareness** | UI knows the structure of data it displays |
| **Semantic diffing** | Show meaningful changes, not just text diffs |
| **Domain adaptation** | Different UIs for different data types |
| **Progressive disclosure** | Start simple, drill down for detail |
| **Visualization** | Graphs, diagrams, timelines where appropriate |

**Design implications**:
- Need plugin/extension architecture for domain UIs
- Need schema registry or discovery mechanism
- Need diff algorithms beyond textual (structural, semantic)
- Consider: JSON Schema, custom renderers, visual DSLs

---

### Quality 5: Governance

**Definition**: The ability to control, constrain, and audit operations according to organizational policies.

**Manifestations across use cases**:
- UC5: Approval workflows, sandboxing, policy enforcement
- UC6: Audit logging, compliance reporting

**Sub-qualities**:
| Sub-quality | Description |
|-------------|-------------|
| **Policy specification** | Declaratively define what's allowed/required |
| **Pre-flight checks** | Validate before execution |
| **Approval routing** | Direct requests to appropriate approvers |
| **Execution constraints** | Enforce sandboxing, resource limits |
| **Audit completeness** | Capture all relevant events immutably |
| **Non-repudiation** | Prove who did what (signatures, attestations) |

**Design implications**:
- Need policy engine (or integration point)
- Need approval state machine
- Need execution environment abstraction (local, sandboxed, remote)
- Consider: OPA/Rego, workflow engines, cryptographic signing

---

### Quality 6: Composability

**Definition**: The ability to build different UI experiences from shared primitives and allow extension without modification.

**Manifestations across use cases**:
- UC4: Domain-specific editors plugged into common shell
- All: Common patterns reused across different views

**Sub-qualities**:
| Sub-quality | Description |
|-------------|-------------|
| **Plugin architecture** | Add new UI components without changing core |
| **Widget embedding** | Embed Graft UI pieces in external apps |
| **API-first design** | UI built on same APIs available to others |
| **Theming/skinning** | Adapt appearance to organizational branding |
| **Configuration-driven** | Behavior adjustable without code changes |

**Design implications**:
- Need well-defined extension points
- Need component library/design system
- Need public API that powers the UI
- Consider: Micro-frontends, Web Components, iframe embedding

---

## Part 2: Expanded Behaviors and Related Capabilities

Beyond the explicit use cases, these related behaviors emerge from the qualities above:

### Execution & Control

| Behavior | Description | Related Quality |
|----------|-------------|-----------------|
| **Streaming output** | Real-time command output as it happens | Observability, Actionability |
| **Cancellation** | Stop running commands | Actionability |
| **Retry with modifications** | Re-run failed commands with tweaks | Actionability |
| **Scheduling** | Queue commands for future execution | Actionability, Governance |
| **Dependency visualization** | See upgrade paths and blockers | Observability, Context |

### Collaboration

| Behavior | Description | Related Quality |
|----------|-------------|-----------------|
| **Review workflows** | Request/provide feedback on changes | Governance, Actionability |
| **Comments & annotations** | Discuss specific items | Connectivity, Context |
| **Notifications** | Alert relevant parties | Connectivity |
| **Mentions & assignments** | Direct attention to people | Connectivity, Governance |
| **Shared views** | Collaborative dashboards | Observability, Composability |

### Search & Discovery

| Behavior | Description | Related Quality |
|----------|-------------|-----------------|
| **Full-text search** | Find executions by output content | Observability |
| **Faceted filtering** | Narrow by status, time, actor, type | Observability |
| **Saved queries** | Reuse common searches | Composability |
| **Comparison views** | Side-by-side state comparison | Context |
| **Time travel** | View state at a point in time | Observability |

### Accessibility & UX

| Behavior | Description | Related Quality |
|----------|-------------|-----------------|
| **Keyboard navigation** | Full keyboard operability | Actionability |
| **Mobile-responsive** | Usable on phones/tablets | Composability |
| **Offline capability** | Read access when disconnected | Observability |
| **Dark mode** | Respect system preferences | Composability |
| **Internationalization** | Multiple languages | Composability |

### Integration

| Behavior | Description | Related Quality |
|----------|-------------|-----------------|
| **Webhooks** | Notify external systems of events | Connectivity |
| **API access** | Programmatic control | Composability, Connectivity |
| **SSO/SAML/OIDC** | Enterprise authentication | Governance |
| **Embeddable widgets** | Graft status in other UIs | Connectivity, Composability |
| **CLI parity** | Everything doable via UI also doable via CLI | Composability |

---

## Part 3: Architectural Choices for Graft Core

To support these UI capabilities, Graft core may need to evolve. These are exploratory ideas for what primitives or changes might help:

### 3.1 Execution Records as First-Class Entities

**Current state**: Commands run and produce output, but there's no standardized record.

**Potential evolution**: Formalize "Execution" as a first-class entity:

```yaml
# Conceptual: stored in .graft/executions/ or as git notes
execution:
  id: "exec-2026-01-07-xyz789"
  command: "meta-kb:migrate-v2"

  context:
    repository: "/path/to/repo"
    git_ref: "abc123"
    working_tree_clean: true
    invoker: "user@example.com"
    invocation_source: "web-ui"  # or "cli", "ci", "api"

  timing:
    queued_at: "2026-01-07T10:00:00Z"
    started_at: "2026-01-07T10:00:05Z"
    completed_at: "2026-01-07T10:02:30Z"

  input:
    args: ["--dry-run"]
    env:
      GRAFT_VERBOSE: "true"

  output:
    exit_code: 0
    stdout_ref: "blob:abc123"  # or inline for small outputs
    stderr_ref: "blob:def456"
    artifacts:
      - path: "report.json"
        ref: "blob:ghi789"

  outcome: "success"  # success, failure, cancelled, timeout

  links:
    triggered_by: "pr:org/repo#123"
    related_to: ["issue:ORG-456", "workspace:coder/task-789"]
```

**Benefits**:
- UIs can query/display execution history
- Enables audit trails
- Links provide connectivity to external resources
- Structured data enables rich filtering/search

**Open questions**:
- Storage location? (`.graft/executions/`, git notes, external DB)
- Retention policy?
- Performance at scale (1000s of executions)?

---

### 3.2 Linking Infrastructure

**Current state**: No standard way to reference external resources.

**Potential evolution**: Define a link specification:

```yaml
# In graft.yaml or per-execution metadata
links:
  # URN-style identifiers
  upstream_dep: "graft:org/meta-kb@v2.0.0"
  source_pr: "github:org/repo/pull/123"
  tracking_issue: "jira:ORG-456"
  workspace: "coder:workspace/task-789"
  docs: "https://docs.example.com/migration-v2"

# Link type registry (extensible)
link_types:
  github:
    pattern: "github:{org}/{repo}/{type}/{id}"
    url_template: "https://github.com/{org}/{repo}/{type}/{id}"
  jira:
    pattern: "jira:{key}"
    url_template: "https://jira.example.com/browse/{key}"
  coder:
    pattern: "coder:{workspace_path}"
    url_template: "https://coder.example.com/{workspace_path}"
```

**Benefits**:
- Consistent linking across all Graft artifacts
- UIs can render rich link previews
- External systems can deep-link back
- Decoupled from specific hosting providers

---

### 3.3 Structured Output Specification

**Current state**: Command output is opaque text.

**Potential evolution**: Commands can declare structured output:

```yaml
commands:
  analyze-upgrade:
    run: "scripts/analyze.sh"
    output:
      format: "json"
      schema: "schemas/upgrade-analysis.json"
      # UI hint: render as expandable tree with risk highlighting
      ui_hints:
        renderer: "upgrade-analysis-view"

  migrate-v2:
    run: "npx jscodeshift ..."
    output:
      format: "ndjson"  # newline-delimited JSON for streaming
      schema: "schemas/migration-progress.json"
      ui_hints:
        renderer: "progress-stream"
        summary_field: "files_modified"
```

**Benefits**:
- UIs can render output meaningfully (not just `<pre>` blocks)
- Enables filtering, searching within output
- Supports streaming progress indicators
- Domain-specific renderers possible

---

### 3.4 Policy Specification Points

**Current state**: No policy mechanism.

**Potential evolution**: Declarative policy attachment:

```yaml
# In graft.yaml or separate policy file
policies:
  # Command-level policies
  commands:
    migrate-v2:
      requires_approval:
        from: ["security-team", "tech-lead"]
        min_approvers: 1
      execution_constraints:
        sandbox: required
        timeout: 600s
        network: deny
      audit:
        retention: 90d
        include_output: true

  # Global policies
  global:
    production_branches:
      patterns: ["main", "release/*"]
      require_approval: true
      notify: ["#deploys"]
```

**Benefits**:
- Governance rules live with the code
- UIs can render approval workflows
- Policy engine can be external (OPA, etc.) or built-in
- Auditable policy history (it's in git)

---

### 3.5 Event Emission

**Current state**: No event infrastructure.

**Potential evolution**: Graft emits events for key operations:

```yaml
# Event schema (conceptual)
events:
  - type: "command.started"
    execution_id: "exec-xyz"
    timestamp: "2026-01-07T10:00:00Z"
    payload:
      command: "meta-kb:migrate-v2"
      actor: "user@example.com"

  - type: "command.progress"
    execution_id: "exec-xyz"
    timestamp: "2026-01-07T10:00:30Z"
    payload:
      percent: 45
      message: "Processing src/ directory..."

  - type: "command.completed"
    execution_id: "exec-xyz"
    timestamp: "2026-01-07T10:02:30Z"
    payload:
      outcome: "success"
      duration_ms: 150000
```

**Delivery mechanisms**:
- Local: Unix socket, file watcher
- Remote: WebSocket, Server-Sent Events
- Integration: Webhooks, message queues

**Benefits**:
- Real-time UI updates
- External system integration
- Enables distributed/async UIs

---

### 3.6 Schema and Type Registry

**Current state**: No schema awareness.

**Potential evolution**: Register schemas for structured content:

```yaml
# Schema registry in graft.yaml
schemas:
  # Local schemas
  recipe:
    file: "schemas/recipe.json"
    ui_editor: "recipe-form-editor"
    diff_renderer: "recipe-semantic-diff"

  # Remote/shared schemas
  infrastructure:
    source: "git@github.com:org/infra-schemas#v1.0"
    path: "schemas/infra.json"

types:
  # File type associations
  "recipes/**/*.yaml": recipe
  "infrastructure/**/*.yaml": infrastructure
  "config.yaml": graft-config
```

**Benefits**:
- UIs know how to render/edit different file types
- Semantic diffing possible
- Validation during editing
- Plugin discovery (which editor for which type)

---

## Part 4: Ecosystem Repositories

To implement the UI vision while maintaining separation of concerns, consider these repositories:

### Core Infrastructure

| Repository | Purpose | Key Contents |
|------------|---------|--------------|
| **graft** | Core CLI tool | Commands, lock file, change tracking |
| **graft-api** | HTTP/WebSocket API server | REST endpoints, event streaming, auth |
| **graft-executor** | Sandboxed command execution | Container runtime, resource limits, isolation |

### UI Layer

| Repository | Purpose | Key Contents |
|------------|---------|--------------|
| **graft-ui** | Web application | React/Vue/Svelte app, routing, state |
| **graft-ui-components** | Shared component library | Design system, common widgets |
| **graft-ui-plugins** | Domain-specific UI plugins | Schema editors, custom renderers |

### Integration

| Repository | Purpose | Key Contents |
|------------|---------|--------------|
| **graft-github-app** | GitHub integration | PR comments, status checks, webhooks |
| **graft-gitlab-integration** | GitLab integration | MR integration, pipelines |
| **graft-coder-integration** | Coder workspace integration | Workspace links, environment setup |

### Governance & Compliance

| Repository | Purpose | Key Contents |
|------------|---------|--------------|
| **graft-policy-engine** | Policy evaluation | OPA integration, approval workflows |
| **graft-audit** | Audit log infrastructure | Storage, querying, compliance reports |

### SDK & Tooling

| Repository | Purpose | Key Contents |
|------------|---------|--------------|
| **graft-sdk-python** | Python SDK | Client library for API |
| **graft-sdk-js** | JavaScript/TypeScript SDK | Client library, React hooks |
| **graft-mcp-server** | Model Context Protocol server | AI assistant integration |
| **graft-vscode** | VS Code extension | Editor integration |

---

## Part 5: Architectural Patterns

### Pattern A: Layered Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Browser (graft-ui)                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │  Dashboard  │  │  Execution  │  │   Review    │          │
│  │    View     │  │   Browser   │  │  Workflow   │          │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘          │
│         └────────────────┴────────────────┘                  │
│                          │                                   │
│              ┌───────────▼───────────┐                       │
│              │   graft-ui-components │                       │
│              │   (shared widgets)    │                       │
│              └───────────┬───────────┘                       │
└─────────────────────────│───────────────────────────────────┘
                          │ HTTP/WebSocket
┌─────────────────────────▼───────────────────────────────────┐
│                    graft-api (server)                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │   REST API  │  │   Events    │  │    Auth     │          │
│  │  /commands  │  │  WebSocket  │  │  OAuth/OIDC │          │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘          │
│         └────────────────┴────────────────┘                  │
│                          │                                   │
│              ┌───────────▼───────────┐                       │
│              │    graft-executor     │                       │
│              │  (sandboxed runtime)  │                       │
│              └───────────┬───────────┘                       │
└─────────────────────────│───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                    Git Repository                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │ graft.yaml  │  │ graft.lock  │  │ .graft/     │          │
│  │             │  │             │  │ executions/ │          │
│  └─────────────┘  └─────────────┘  └─────────────┘          │
└─────────────────────────────────────────────────────────────┘
```

### Pattern B: Plugin Architecture for Domain UIs

```
┌─────────────────────────────────────────────────────────────┐
│                    Plugin Host (graft-ui)                    │
│  ┌─────────────────────────────────────────────────────────┐│
│  │                    Plugin Registry                       ││
│  │  ┌───────────┐  ┌───────────┐  ┌───────────┐            ││
│  │  │  recipe   │  │  infra    │  │  policy   │            ││
│  │  │  editor   │  │  diagram  │  │  editor   │            ││
│  │  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘            ││
│  └────────│──────────────│──────────────│──────────────────┘│
│           │              │              │                    │
│  ┌────────▼──────────────▼──────────────▼──────────────────┐│
│  │                  Plugin API Contract                     ││
│  │  - render(data, schema) → ReactNode                      ││
│  │  - edit(data, schema, onChange) → ReactNode              ││
│  │  - diff(before, after, schema) → ReactNode               ││
│  │  - validate(data, schema) → ValidationResult             ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

### Pattern C: Event-Driven Real-Time Updates

```
┌──────────┐    ┌──────────┐    ┌──────────┐
│  graft   │───▶│  Events  │───▶│  graft   │
│   CLI    │    │  (local) │    │   API    │
└──────────┘    └──────────┘    └────┬─────┘
                                     │
                    ┌────────────────┴────────────────┐
                    │         Event Router            │
                    └────────────────┬────────────────┘
                                     │
        ┌─────────────┬──────────────┼──────────────┬─────────────┐
        ▼             ▼              ▼              ▼             ▼
   ┌─────────┐  ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐
   │ Browser │  │ Webhook │   │  Slack  │   │  Audit  │   │  GitHub │
   │   UI    │  │ Delivery│   │  Bot    │   │   Log   │   │   App   │
   └─────────┘  └─────────┘   └─────────┘   └─────────┘   └─────────┘
```

---

## Part 6: Open Questions for Further Exploration

### Technical

1. **Execution storage**: Git objects vs. file system vs. external database?
2. **Real-time protocol**: WebSocket vs. Server-Sent Events vs. polling?
3. **Plugin sandboxing**: How to safely run third-party UI plugins?
4. **Offline support**: What subset works without network?
5. **Performance**: How to handle 10K+ executions, 100+ concurrent users?

### Product

1. **Self-hosted vs. SaaS**: Should there be a hosted offering?
2. **Multi-tenancy**: One UI instance per repo, per org, or global?
3. **Pricing/licensing**: Open source core + enterprise features?
4. **Migration path**: How do existing Graft users adopt the UI?

### Governance

1. **Policy language**: Custom DSL vs. OPA/Rego vs. simple YAML rules?
2. **Approval persistence**: Where do approvals live? (Git, external DB)
3. **Compliance certifications**: SOC2, FedRAMP, etc. implications?

### Integration

1. **IDE priority**: VS Code first, or web-first then IDE?
2. **Git hosting**: GitHub-first, or provider-agnostic from start?
3. **AI integration**: MCP server priority vs. native AI features?

---

## Part 7: Relationship to Existing Brainstorming

This document expands on ideas from [Evolution Brainstorming (2026-01-05)](./2026-01-05-evolution-brainstorming.md), specifically:

- **Section 2: Web UI for Graft Repositories** - We've deepened the architectural thinking
- **Section 5: Multiple Interfaces** - We've explored the UI layer in detail
- **Section 4: Structured Transactions** - Execution records build on transaction concepts
- **Section 7: Ecosystem of Composable Components** - Repository structure is now more concrete

**New contributions in this document**:
- Identified 6 key qualities (Observability, Connectivity, Actionability, Context Richness, Governance, Composability)
- Expanded behavioral capabilities beyond original use cases
- Detailed architectural choices for Graft core evolution
- Concrete ecosystem repository breakdown
- Architectural patterns (layered, plugin, event-driven)

---

## Sources

- [Graft Architecture](../docs/specifications/architecture.md)
- [Evolution Brainstorming (2026-01-05)](./2026-01-05-evolution-brainstorming.md)
- [Design Improvements Analysis](./2026-01-05-design-improvements-analysis.md)
- [Core Operations Specification](../docs/specifications/graft/core-operations.md)
- [Meta Knowledge Base: Temporal Layers](../.graft/meta-knowledge-base/docs/policies/temporal-layers.md)

---

## Next Steps (When Ready to Move Beyond Brainstorming)

1. **Validate use cases**: Confirm these match real user needs
2. **Prioritize qualities**: Which are must-have vs. nice-to-have?
3. **Prototype execution records**: Try storing a few, see what works
4. **Design API contract**: What endpoints does graft-api expose?
5. **Build minimal UI**: Command execution browser as MVP
6. **Iterate based on usage**: Let real use drive evolution
x