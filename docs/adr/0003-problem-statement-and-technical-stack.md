# 3. Problem statement and technical stack

Date: 2025-11-12

## Status

Accepted

## Context

Software development and data workflows increasingly involve both automated transformations and human judgment. Organizations face several challenges:

1. **Automated outputs need human refinement** — Generated documents, reports, and derived data often need manual review, correction, or context addition
2. **Provenance is critical but missing** — Compliance, research, and collaboration require knowing exactly what inputs produced what outputs, who reviewed them, and when
3. **AI agents need trust boundaries** — As AI capabilities grow, organizations need infrastructure to deploy agents with appropriate oversight
4. **Workflows span organizational boundaries** — Teams want to compose data workflows like they compose code (dependencies, versioning, sharing)
5. **Traditional tools don't fit** — Build tools treat outputs as ephemeral; data pipelines lack human-in-the-loop support; wikis don't track provenance

The core problem: **How do we build file-first data workflows where outputs are living documents that humans and agents collaborate on, with complete auditability?**

Key requirements:
- Outputs must be git-versioned files (not cached artifacts)
- Full provenance tracking (what was read, written, who decided, when)
- Policy enforcement (deterministic, attestation, direct edit controls)
- Support for automated (containers), guided (templates), and manual transformations
- Human and AI agent parity (both are first-class actors)
- Composability via git references (remote materials, workflow supply chains)

## Decision

We will build **Graft**: a git-backed build tool for file-first data workflows with provenance and policy enforcement.

### Technology Stack

**Python 3.14+**
- Supports rapid iteration
- Mature ecosystem for data tools
- Excellent typing support (mypy)
- Cross-platform
- Well-suited for CLI tools
- Good Docker SDK support
- Dataclasses provide clean domain modeling

**uv for package management**
- Fast, modern Python package manager
- Better dependency resolution than pip
- Good developer experience
- Standardizing around modern tooling

**DVC for orchestration**
- Mature, well-tested DAG execution
- Handles incremental builds, parallelization, caching
- Already used by data teams
- Lets us focus on unique value (provenance + policy)
- Clear separation: DVC executes, Graft tracks

**Typer for CLI**
- Modern, type-safe CLI framework
- Automatic help generation
- Good error handling
- JSON output support
- Clean command structure

**Docker for container transformers**
- Industry standard for isolated execution
- Reproducible builds via image digests
- Cross-platform
- Existing ecosystem of images
- Security via isolation

**Git as the ledger**
- Universal version control
- Proven collaboration model (PRs, review)
- Built-in diff, blame, history
- No additional infrastructure needed
- Already in every development workflow

### What We're NOT Using

**Airflow/Dagster** — Too heavyweight, designed for terabyte-scale ETL, not file-first workflows

**Bazel/Buck** — Focused on code compilation, not data/document workflows

**Custom orchestrator** — DVC provides this; building our own would be reinventing wheels

**Database for provenance** — Files in git are simpler, more portable, git-native

**Custom container runtime** — Docker is ubiquitous; support for alternatives (Podman) can be added via adapters

## Consequences

**Positive:**

- **Rapid iteration** — Python enables fast development cycles
- **Python ecosystem** — Rich libraries for YAML, JSON, templating, Docker, git operations
- **Type safety** — mypy catches errors at development time
- **Modern tooling** — uv provides fast, reliable dependency management
- **Proven orchestration** — DVC handles complex DAG execution, parallelization
- **Standard containers** — Docker provides reproducibility, isolation
- **Git-native** — Fits existing developer workflows, no new infrastructure
- **Cross-platform** — Works on Linux, macOS, Windows

**Negative:**

- **Python requirement** — Users must have Python 3.14+ (but common in data/ML workflows)
- **Docker requirement** — For container transformers, but optional for template-only workflows
- **DVC learning curve** — Some users may need to learn DVC concepts (mitigated: Graft CLI is primary interface)
- **Not optimized for massive scale** — This is for 10-100 artifacts, not terabyte-scale pipelines

**Neutral:**

- **Young stack** — Python 3.14 is recent (but backwards compatible to extent possible)
- **uv adoption** — uv is newer than pip/poetry (but gaining traction)
- **DVC dependency** — We're coupled to DVC's evolution (but stable, mature project)

## Implementation Notes

- Target Python 3.14 for modern features, but maintain compatibility where practical
- Use `pyproject.toml` for package configuration (PEP 518)
- Type hints everywhere, enforce with mypy
- Black-box testing via subprocess (CLI as contract)
- Protocol-based adapters for swappable implementations
- Immutable domain objects (`@dataclass(frozen=True)`)
- Result objects with `.to_dict()` for JSON serialization

This stack enables us to deliver on Graft's mission: **auditable, file-first data workflows where humans and agents collaborate with confidence**.
