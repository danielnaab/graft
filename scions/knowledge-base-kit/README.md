# knowledge-base-kit

Conventions for evolving knowledge bases with human and agent collaboration, and the checker that enforces
them. Formerly meta-knowledge-base (the conventions, v1.x) and knowledge-base-integrity (the checker, v0.x),
merged in v2.0.0 so that a rule and its enforcement change together.

## What this system standardizes

This system does **not** mandate document taxonomies. It standardizes:

- **Entrypoints**: where humans and agents begin
- **Authority**: what counts as canonical truth
- **Provenance**: how to ground important claims in sources
- **Lifecycle**: draft → working → stable → deprecated
- **Strategy**: how to choose an organization approach and evolve it with evidence
- **Linking**: how to keep browser navigation intuitive
- **Temporal layers**: organizing ephemeral, tracking, and durable knowledge, plus optional raw
  sources and historical records
- **KB dependencies**: managing cross-repository knowledge relationships
- **Evolution triggers**: evidence-based approach to adding complexity
- **Generated content**: when and how to automate documentation
- **Multi-agent coordination**: patterns for specialized agent collaboration

## Core Policies

Essential patterns for all knowledge bases:

- [Authority](docs/policies/authority.md) - What counts as canonical truth
- [Provenance](docs/policies/provenance.md) - Grounding claims in sources
- [Lifecycle](docs/policies/lifecycle.md) - Document status progression
- [Linking](docs/policies/linking.md) - Navigation and references
- [Intent-revealing structure](docs/policies/intent-revealing-structure.md) - File and directory naming
- [Write boundaries](docs/policies/writes.md) - Safe agent editing zones
- [Style](docs/policies/style.md) - Plain language and structure

## Extended Policies

For evolving and complex knowledge bases:

- [Temporal layers](docs/policies/temporal-layers.md) - Organizing ephemeral, tracking, durable, raw, and historical knowledge
- [Generated content](docs/policies/generated-content.md) - When to automate documentation

## Organization Strategies

Choose your approach:

- [Knowledge organization strategies](docs/strategies/README.md)
  - Handbook (curated onboarding)
  - Atomic notes + maps (research/exploration)
  - Source-first synthesis (auditability)
  - Metadata + generated views (scale)

## Playbooks

Step-by-step guidance:

- [Choose a strategy](docs/playbooks/choose-strategy.md) - Initial organization decisions
- [Evolve a knowledge base](docs/playbooks/evolve-kb.md) - Evidence-based evolution
- [Evolve a UI](docs/playbooks/evolve-ui.md) - Safe UI introduction
- [Manage KB dependencies](docs/playbooks/manage-kb-dependencies.md) - Cross-repository coordination
- [Track spec-impl relationship](docs/playbooks/track-spec-impl-relationship.md) - Specification-to-implementation tracking
- [Agent workflow](docs/playbooks/agent-workflow.md) - Plan → Patch → Verify pattern
- [Multi-agent coordination](docs/playbooks/multi-agent-coordination.md) - Specialized agent collaboration

## Quick Start

### For a new knowledge base

1. Create `knowledge-base.yaml` with entrypoints
2. Add `README.md` and optional `AGENTS.md`
3. Evolve based on pain (see [Evolve KB playbook](docs/playbooks/evolve-kb.md))

### For an evolving knowledge base

1. Review [Evolution triggers](docs/playbooks/evolve-kb.md#evidence-based-evolution-triggers)
2. Add what you need based on observed pain
3. Start with low-cost moves (entrypoints, lifecycle markers)
4. Iterate based on evidence

### For multi-repository projects

1. Separate specs from implementation if needed (see [KB dependencies](docs/playbooks/manage-kb-dependencies.md))
2. Declare relationship types in knowledge-base.yaml
3. Establish sync policies
4. Track drift explicitly

### For agent-augmented development

1. Define human and agent entrypoints
2. Set write boundaries
3. Use Plan → Patch → Verify pattern (see [Agent workflow](docs/playbooks/agent-workflow.md))
4. Consider multi-agent patterns if you have role separation (see [Multi-agent coordination](docs/playbooks/multi-agent-coordination.md))

## Philosophy

**Minimal intervention:** Only standardize operations, not structure

**Evidence-driven:** Evolve based on observed pain, not speculation

**Support evolution:** Knowledge bases that grow and adapt over time

**Agent-friendly:** Clear entrypoints, boundaries, and coordination patterns

**Human-centered:** Git is source of truth, UIs are views

## Examples

See [examples/](examples/) for complete knowledge bases demonstrating each strategy.

## The checker

Three tools in `src/` (Python standard library only) check a knowledge base against these conventions:

| Tool | Checks |
|------|--------|
| `link-validator` | Internal Markdown links and their heading anchors exist |
| `kb-linter` | Frontmatter `status` against the [lifecycle policy](docs/policies/lifecycle.md), provenance (a `## Sources` section or `sources` frontmatter), OKF conformance |
| `backlink-scanner` | `# spec:` annotations point at real specs, and specs have implementations ([living specifications](https://github.com/danielnaab/living-specifications)) |

How to configure and run them: [docs/tools.md](docs/tools.md). Each tool's behavior is specified in
[specs/](https://github.com/danielnaab/knowledge-base-kit/tree/main/specs).

## Using this in your repository

Graft it ([graft](https://github.com/danielnaab/graft/tree/v2)): the export (README, CHANGELOG, `docs/`,
`adapters/`, `examples/` and the checker's `src/`) lands at a pinned tag in your scions directory.

```nix
inputs.knowledge-base-kit = { url = "github:danielnaab/knowledge-base-kit/v2.1.0"; flake = false; };
```

Then add a `knowledge-base.yaml` at your repo root. `apiVersion: kb/v1` is the only value; nothing reads it yet.

```yaml
apiVersion: kb/v1
name: your-project

entrypoints:
  human: README.md
  agent: AGENTS.md

imports:
  - kind: local
    path: scions/knowledge-base-kit
    relationship: foundational
```

To run the checker, read it from the same input: `PYTHONPATH=${inputs.knowledge-base-kit}/src python3 -m
link_validator .` (and `kb_linter`, `backlink_scanner`). [notebook-kit](https://github.com/danielnaab/notebook-kit)
wires this up as `just check-kb`. Don't add a second input for the same repo (a `git+https` one, say): Nix shares
one fetch cache entry per commit between them, so one of the two gets the wrong tree.

## Architecture

```
living-specifications (spec-writing methodology)
        ↓ the backlink scanner implements its `# spec:` annotation format
knowledge-base-kit (this repo: conventions, and the checker that enforces them)
        ↓
your knowledge base (follows the conventions, runs the checker)
```

The kit grafts living-specifications into `scions/` (not exported). living-specifications follows these
conventions as a knowledge base: a conformance edge, not a design dependency (see
[Managing KB dependencies](docs/playbooks/manage-kb-dependencies.md#dependencies-are-between-exports)).

## Developing the checker

```bash
uv sync
uv run pytest
uv run ruff check src tests
uv run link-validator && uv run kb-linter && uv run backlink-scanner   # dogfood on this repo
```

## Navigation

- [Strategies](docs/strategies/README.md)
- [Policies](docs/policies/README.md)
- [Playbooks](docs/playbooks/README.md)
- [Examples](examples/README.md)
- [Adapters](adapters/claude-code/README.md)
- [The checker](docs/tools.md)
