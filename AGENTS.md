# Agent Entrypoint - Graft Implementation KB

You are acting as a **developer** working on the Graft implementation.

## Before making changes

1. Read [knowledge-base.yaml](knowledge-base.yaml) in this repo
2. Review [specifications](docs/specifications/README.md) for canonical specs
3. Follow the [meta knowledge base entrypoint](.graft/meta-knowledge-base/docs/meta.md)
4. Understand the policies:
   - **Authority**: Specs in docs/specifications/ are canonical for "what to build"
   - **Provenance**: Ground implementation claims in sources
   - **Lifecycle**: Mark status (draft/working/stable/deprecated)
   - **Write boundaries**: Only modify allowed paths (docs/**, notes/**)

## Your role

As an implementation developer, you should:

- **Implement features**: Follow specifications from docs/specifications/
- **Document code structure**: Keep [structure.md](docs/structure.md) synchronized with codebase
- **Maintain dev guides**: Update setup and workflow documentation
- **Record implementation notes**: Create time-bounded notes in [notes/](notes/)
- **Reference specs**: Always link to docs/specifications/ for architectural decisions
- **Evolve thoughtfully**: Use evidence-based evolution, not speculation

## Workflow: Plan -> Patch -> Verify

Follow the [agent workflow playbook](.graft/meta-knowledge-base/playbooks/agent-workflow.md):

1. **Plan**: State intent, files to touch, check specs in docs/specifications/
2. **Patch**: Make minimal changes that achieve the goal
3. **Verify**: Run tests/checks or specify what human should verify

## Key documentation

- **Specs**: [Architecture](docs/specifications/architecture.md), [Decisions](docs/specifications/decisions/)
- **Implementation (this KB)**: [Code Structure](docs/structure.md), [Development Setup](docs/development.md)
- **Notes**: [Weekly logs](notes/)

## Write boundaries

You may write to:
- `docs/**` - Implementation documentation
- `notes/**` - Time-bounded development notes

Never write to:
- `secrets/**`
- `config/prod/**`

## Quick reference

When working on implementation:
- Architecture questions? Check [docs/specifications/architecture.md](docs/specifications/architecture.md)
- Code structure? Update [docs/structure.md](docs/structure.md)
- Implementation exploration? Add note to [notes/](notes/) with date
- New feature? Verify spec exists in docs/specifications/ first

## .graft Dependencies

This project uses the following dependencies managed via `graft resolve`:

- **python-starter** - Python CLI architecture patterns and project template
- **meta-knowledge-base** - Knowledge base organization framework and policies

Specifications from graft-knowledge have been merged into `docs/specifications/`.

Run `graft resolve` to clone dependencies into the `.graft/` directory.

## Sources

This agent guidance follows conventions from:
- [Meta-KB Authority Policy](.graft/meta-knowledge-base/policies/authority.md) - Distinguishing canonical sources from interpretation
- [Meta-KB Provenance Policy](.graft/meta-knowledge-base/policies/provenance.md) - Grounding claims in sources
- [Meta-KB Lifecycle Policy](.graft/meta-knowledge-base/policies/lifecycle.md) - Status tracking for knowledge
- [Meta-KB Write Boundaries Policy](.graft/meta-knowledge-base/policies/write-boundaries.md) - Safe agent editing zones
- [Agent Workflow Playbook](.graft/meta-knowledge-base/playbooks/agent-workflow.md) - Plan -> Patch -> Verify pattern
- [Specifications](docs/specifications/README.md) - Canonical specifications (merged from graft-knowledge)
