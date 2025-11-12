# Graft

Graft binds together material from different origins—code, specifications, research, strategy—into artifacts that grow and adapt as your sources change.

Graft is a git-backed build tool for file-first data workflows. It uses DVC to track dependencies and orchestrate transformations—containers, templates, or manual edits by humans or agents—producing outputs that live as versioned files in your repository. Every transformation records full provenance: input hashes, who finalized, when, and under what policy. Outputs aren't ephemeral build artifacts—they're files you commit, edit, and evolve.

## Why Graft?

**File-first workflows** — Outputs are living files you edit directly, not disposable build artifacts. Generated, refined, and evolved over time.

**Full provenance** — Every artifact records what materials fed it, what transformation produced it, and who finalized it. Complete audit trails for compliance, research, and collaboration.

**Human and agent collaboration** — Transformations can be fully automated (containers, templates) or require manual work. Humans and AI agents are first-class participants with attribution and policy enforcement.

**Composable across boundaries** — Reference materials and workflows from remote repositories. Build workflow supply chains with git as the transport layer.

## Quick Example

An artifact that transforms ticket data into a weekly sprint brief:

```yaml
# artifacts/sprint-brief/graft.yaml
graft: sprint-brief
inputs:
  materials:
    - { path: "../../sources/tickets/sprint-2025-11W1.yaml", rev: HEAD }
derivations:
  - id: brief
    transformer: { ref: report-md, params: { title: "Sprint Brief" } }
    template:
      source: file
      engine: jinja2
      file: "./template.md"
    outputs:
      - { path: "./brief.md" }
    policy:
      deterministic: true
      attest: required
      direct_edit: true
```

Run the transformation:
```bash
graft run artifacts/sprint-brief/
```

The template generates `brief.md`. You edit it directly to add context and insights. When done:

```bash
graft finalize artifacts/sprint-brief/ --agent "Jane Doe"
```

Next week, when tickets change, Graft detects the drift. You propagate the updates, refine the brief, and finalize again. Full history in git. Full provenance in `.graft/provenance/`.

## Use Cases

**Auditable AI agency** — Deploy AI agents with confidence. Full attribution, policy enforcement, and human oversight.

**Composable workflows** — Build data workflow supply chains. Reference remote grafts, extend them, version them, share them.

**Living team knowledge** — Sprint briefs, retrospectives, runbooks that evolve with your work. Dependencies flow, reviews happen in PRs, nothing goes stale.

**Reproducible research** — Data pipelines with human judgment captured. Show reviewers exactly what ran, who decided what, when.

See [Use Cases](docs/use-cases.md) for detailed narratives.

## Documentation

- **[Concepts](docs/concepts.md)** — Core mental model: artifacts, materials, derivations, provenance, and the finalize transaction
- **[Tutorial](docs/tutorial.md)** — Hands-on walkthrough building your first graft
- **[Workflows](docs/workflows.md)** — Patterns for automated, manual, and hybrid workflows
- **[CLI Reference](docs/cli-reference.md)** — Command documentation
- **[graft.yaml Reference](docs/graft-yaml-reference.md)** — Configuration schema and examples
- **[Architecture](docs/architecture.md)** — Technical architecture and design
- **[Philosophy of Design](docs/philosophy-of-design.md)** — Design principles and rationale
- **[DVC Integration](docs/dvc-integration.md)** — How Graft uses DVC for orchestration
- **[FAQ](docs/faq.md)** — Common questions

## Installation

```bash
# Using pip
pip install graft

# Using uv
uv pip install graft
```

## Quick Start

Initialize a Graft project:
```bash
graft init
```

This creates `graft.config.yaml` with defaults. See the [Tutorial](docs/tutorial.md) for a complete walkthrough.

## Requirements

- Python 3.14+
- Git
- DVC (installed with Graft)
- Docker (for container-based transformers)

## Contributing

See [Implementation Strategy](docs/implementation-strategy.md) and [Testing Strategy](docs/testing-strategy.md) for development practices.

## License

MIT
