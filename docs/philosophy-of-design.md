# Philosophy of Design

This document explains the principles guiding Graft's design: why certain decisions were made, what trade-offs were accepted, and what values drive the project.

## Core Principles

### 1. Normal Editing Stays Normal

**Principle:** Users should be able to edit outputs directly using their normal tools (text editors, IDEs). Graft doesn't dictate special formats or require proprietary editors.

**Why this matters:**
- Outputs are just files—markdown, YAML, JSON, whatever makes sense
- Edit with VSCode, vim, Claude Code, or any tool
- No learning curve for editing
- No lock-in to Graft-specific tooling

**Implications:**
- `direct_edit: true` policy enables this workflow
- Git provides version control, diff, review
- PRs are the review mechanism (no custom UI needed)
- Conflicts resolved like any file conflicts

**Anti-pattern:** Tools that generate code you can't edit, or lock content in databases/proprietary formats.

**Graft approach:** Files are the source of truth. Edit them. Graft tracks what you did.

---

### 2. File-First, Not Cache-First

**Principle:** Outputs are committed files in git, not ephemeral build artifacts cached somewhere.

**Why this matters:**
- Outputs are living documents that evolve
- History matters (when did this change, who changed it, why)
- Reviewability via git diff, git blame
- Outputs can be materials for other grafts
- No separate cache management (git is the cache)

**Trade-offs accepted:**
- Git repo grows (mitigate with DVC for large files, .gitignore for build artifacts)
- Can't "clean and rebuild" like traditional builds (but you can re-run and see what changed)
- Merge conflicts require resolution (but meaningful conflicts should be reviewed anyway)

**Implications:**
- Every finalize creates a git commit (atomic: outputs + provenance)
- Provenance lives in `.graft/provenance/`, committed alongside outputs
- DVC manages execution but outputs stay in git

**Anti-pattern:** Tools where "outputs" are in a cache and source of truth is unclear.

**Graft approach:** Git is the ledger. Outputs are first-class versioned content.

---

### 3. Provenance, Not Just Execution

**Principle:** Knowing *what ran* isn't enough. We need to know *what was read, what was written, who decided, when, under what policy*.

**Why this matters:**
- Compliance requires audit trails
- Research requires reproducibility
- Collaboration requires attribution
- Debugging requires understanding "how did we get here?"

**Provenance captures:**
- **Read set** — Materials (exact hashes, git refs)
- **Write set** — Outputs (exact hashes)
- **Transformation** — Container digest, template hashes
- **Attribution** — Who finalized, when, role
- **Policy** — What constraints governed the transformation

**Implications:**
- Finalize is a required step (not automatic)
- Provenance is structured JSON, machine-readable
- Can reconstruct exactly what happened
- Can verify outputs match provenance (detect tampering)

**Anti-pattern:** Tools where you run a script and hope you remember what happened.

**Graft approach:** Every transformation leaves a complete audit trail.

---

### 4. Humans and Agents Are First-Class

**Principle:** Both humans and AI agents are valid participants in workflows, with equal support for attribution and policy enforcement.

**Why this matters:**
- AI agents are increasingly capable (and will only improve)
- But they need oversight, attribution, trust boundaries
- Infrastructure should support both, not treat agents as second-class

**Graft supports:**
- Agent attribution (`--agent`, `--role`)
- Policy-based trust boundaries (some grafts allow auto-merge, others require review)
- PR-based review for agent changes (just like human changes)
- Provenance distinguishes human vs. agent contributions

**Implications:**
- `attest: required` captures who made decisions
- Policy varies by artifact (low-stakes: agents can auto-finalize; high-stakes: human review required)
- AI agents call `graft explain --json` to understand context, `graft finalize` to record work

**Anti-pattern:** Tools built only for humans or only for automation, not both.

**Graft approach:** Flexible policy lets you choose the right boundary for each artifact.

---

### 5. Determinism First, Exceptions Explicit

**Principle:** Transformations should be deterministic by default. Non-deterministic workflows are allowed but must be explicit.

**Why this matters:**
- Reproducibility requires determinism
- Caching requires determinism
- Debugging requires predictability
- But some workflows (LLM calls, API fetches) are inherently non-deterministic

**Default:** `policy.deterministic: true`

**When to use `false`:**
- LLM-based synthesis (model may vary)
- API calls to external services
- Time-dependent transformations

**Implications:**
- Container transformers should produce identical outputs given identical inputs
- Templates are deterministic (Jinja2 is pure)
- Non-deterministic workflows still get provenance (record what happened, even if can't replay)

**Anti-pattern:** Pipelines where "rebuild" gives different results and nobody knows why.

**Graft approach:** Determinism is the default. Exceptions are marked and understood.

---

### 6. Policy as Configuration, Not Code

**Principle:** Trust boundaries, review requirements, and constraints should be declarative (YAML), not buried in code.

**Why this matters:**
- Readable by non-programmers
- Auditable (git diff shows policy changes)
- Enforceable by tooling
- Clear expectations

**Policy options:**
- `deterministic: true/false` — Is transformation reproducible?
- `attest: required/optional` — Must someone sign off?
- `direct_edit: true/false` — Can outputs be manually edited?

**Implications:**
- Each artifact declares its own policy
- Policy can vary (sprint briefs allow direct edit, generated code doesn't)
- Policy violations detected by `graft finalize` (fails if constraints not met)

**Anti-pattern:** Security/compliance requirements hidden in undocumented scripts.

**Graft approach:** Policy is explicit, visible, enforceable configuration.

---

### 7. Separation of Concerns: Graft vs. DVC vs. Git

**Principle:** Each tool does what it's best at. Don't reinvent wheels.

**Graft's responsibility:**
- Provenance tracking
- Policy enforcement
- Attribution and attestation
- Template evaluation
- Container orchestration (build, run)

**DVC's responsibility:**
- Dependency DAG execution
- Incremental builds (only run what changed)
- Parallelization
- Caching (for deterministic stages)

**Git's responsibility:**
- Version control
- Diff, blame, history
- Branching, merging
- Collaboration (PRs, review)

**Implications:**
- Graft generates `dvc.yaml` but doesn't execute the DAG (DVC does)
- Outputs live in git, provenance lives in git
- Users can use DVC directly for advanced orchestration (`dvc repro`, `dvc dag`)
- Git hooks can enforce completeness (don't commit dirty artifacts)

**Anti-pattern:** Reinvent version control, orchestration, or collaboration in a custom tool.

**Graft approach:** Compose with mature tools, focus on unique value (provenance + policy).

---

### 8. CLI as Contract

**Principle:** The CLI is the public API. Everything testable via CLI, outputs structured JSON.

**Why this matters:**
- Stability (breaking CLI is breaking users)
- Testability (black-box tests via subprocess)
- Integrability (scripts, CI, agents call `graft --json`)
- Documentation (CLI help, reference docs)

**Implications:**
- All commands support `--json` for machine-readable output
- Exit codes are meaningful (0 = success, 1 = user error, 2 = system error)
- Tests invoke CLI, not internal APIs
- Internal refactoring doesn't break users (as long as CLI contract stable)

**Anti-pattern:** Tools where "real" API is internal and CLI is an afterthought.

**Graft approach:** CLI is the contract. Treat it as public API.

---

### 9. Composability Across Boundaries

**Principle:** Workflows should compose across organizational and repository boundaries via git references.

**Why this matters:**
- Organizations share data workflows like they share code (libraries, packages)
- Upstream providers publish grafts, downstream consumers extend them
- Full provenance across the supply chain
- Git is the transport layer (URLs + refs)

**Graft enables:**
- Remote material references: `https://github.com/org/repo/raw/v1.0/data.json`
- Version pinning: `rev: v1.0` (reproducible builds)
- Upstream updates: change ref, re-run, review changes
- Provenance across boundaries: record exact upstream version used

**Implications:**
- Materials can be local or remote
- Grafts can reference outputs from other repos
- Dependency graphs span organizations
- Versioning (git tags) enables stable APIs

**Anti-pattern:** Data workflows that can't be shared or composed.

**Graft approach:** Git-native workflow supply chains.

---

### 10. Auditability Over Automation

**Principle:** When there's a trade-off between "fully automated" and "fully auditable," choose auditability.

**Why this matters:**
- Trust requires transparency
- Compliance requires proof
- Debugging requires understanding
- Automation without auditability is risky

**Graft prioritizes:**
- Complete provenance over minimal metadata
- Explicit finalize over auto-commit
- Structured JSON over ad-hoc logs
- Attribution over anonymity

**Implications:**
- Provenance files are verbose (full hashes, paths, metadata)
- Finalize is a required step (captures decision point)
- Exit codes signal success/failure (no silent failures)
- Errors are explicit (user error vs. system error distinguished)

**Anti-pattern:** "Fast and loose" automation with no audit trail.

**Graft approach:** Automation with accountability.

---

## Design Trade-Offs

### We Chose: File-First → Accepted: Git Repo Growth

Committed outputs mean repo size grows over time. Mitigations:
- DVC can track large files (store in remote, not git)
- `.gitignore` for truly ephemeral artifacts
- Git history is valuable (shows evolution)

### We Chose: Provenance Verbosity → Accepted: Large JSON Files

Provenance captures everything. Provenance files can be large. Mitigations:
- JSON is compressible
- Future: optional compression or binary format
- Verbosity enables complete audit trails

### We Chose: DVC Integration → Accepted: DVC Learning Curve

Users might need to understand DVC concepts. Mitigations:
- Graft commands are primary interface (DVC is under the hood)
- Documentation explains DVC integration
- Users can ignore DVC for simple workflows

### We Chose: Black-Box Testing → Accepted: Slower Tests

Subprocess tests are slower than unit tests. Mitigations:
- Focus on important workflows, not exhaustive coverage
- Parallel test execution
- Trade-off worth it for stability (CLI contract is what matters)

---

## Non-Goals

**What Graft is NOT trying to do:**

**Not a general-purpose build tool** — Use Make, Bazel, etc. for compiling code. Graft is for file-first data workflows.

**Not a data pipeline orchestrator at scale** — Use Airflow, Dagster for terabyte-scale ETL. Graft is for medium-scale (10-100 artifacts).

**Not a real-time system** — Graft is batch-oriented (run when materials change). Not for streaming or query-time generation.

**Not a content management system** — Use Notion, Confluence for wiki-style content. Graft is for derived content with provenance.

**Not a secret manager** — Use proper secret management (Vault, AWS Secrets). Graft doesn't handle credentials.

---

## Inspirations

**DVC** — Dependency tracking, orchestration, caching for data pipelines.

**Nix** — Deterministic builds, immutability, reproducibility.

**Git** — Version control, collaboration, ledger-based history.

**Jupyter** — Literate workflows, mixing code and outputs.

**Grafting (horticulture)** — Binding together materials from different origins to create something new that grows.

---

## Future Evolution

These principles guide current design but aren't dogmatic. As use cases evolve, so might the principles. Potential future shifts:

**LLM-first workflows** — As LLM integration becomes core, may need new primitives (prompt management, caching, cost tracking).

**Signed provenance** — Cryptographic signatures for tamper-evident audit trails (not just JSON files).

**Distributed execution** — Run transformers on remote workers (cloud functions, K8s) while preserving provenance.

**Real-time features** — Query-time generation or streaming updates (without sacrificing auditability).

The philosophy adapts to serve users, but core values (auditability, file-first, normal editing) remain.

---

This philosophy guides Graft's evolution while staying true to its mission: **auditable, file-first data workflows where humans and agents collaborate with confidence**.

Next: See [Architecture](architecture.md) for how these principles are implemented.
