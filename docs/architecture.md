# Architecture

This document describes Graft's technical architecture: how components fit together, layering, and design decisions.

## Overview

Graft is built on a layered architecture following domain-driven design principles:

```
┌─────────────────────────────────────┐
│         CLI Layer (Typer)           │  Presentation
│   (graft run, finalize, status...)  │
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│        Services Layer               │  Use Cases
│  (RunService, FinalizeService...)   │
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│       Adapters Layer                │  External Interfaces
│  (FileSystem, Docker, DVC, Git...)  │
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│        Domain Layer                 │  Core Entities
│  (Artifact, Derivation, Material...)│
└─────────────────────────────────────┘
```

This separation ensures:
- **Testability** — Black-box CLI tests, no internal imports needed
- **Maintainability** — Clear boundaries, explicit dependencies
- **Flexibility** — Swap implementations (Docker → Podman, DVC → other orchestrators)

## Core Components

### Domain Layer

**Location:** `src/graft/domain/`

**Purpose:** Core business entities and value objects. Immutable, pure Python.

**Key types:**
- `Artifact` — A graft's identity and configuration
- `Material` — Input dependency (path, revision, content)
- `Derivation` — Transformation specification
- `Policy` — Constraints (deterministic, attest, direct_edit)
- `Provenance` — Complete audit record
- `SyncPolicy` — DVC orchestrator behavior

**Characteristics:**
- Immutable dataclasses (`@dataclass(frozen=True)`)
- No external dependencies
- Rich domain model, not anemic

**Example:**
```python
@dataclass(frozen=True)
class Material:
    path: str
    rev: str
    content_sha256: str
    size: int
```

### Adapters Layer

**Location:** `src/graft/adapters/`

**Purpose:** Interfaces to external systems using Protocol types.

**Key adapters:**
- **`LocalFileSystem`** — Read/write files, compute hashes
- **`ConfigAdapter`** — Parse graft.yaml, graft.config.yaml
- **`DockerAdapter`** — Build images, run containers
- **`LocalMaterialLoader`** — Load materials (git-aware)
- **`DVCAdapter`** — Generate dvc.yaml, detect drift
- **`GitAdapter`** (future) — Git operations

**Protocol-based design:**
```python
class FileSystemPort(Protocol):
    def read_file(self, path: Path) -> str: ...
    def write_file(self, path: Path, content: str) -> None: ...
    def compute_hash(self, path: Path) -> str: ...
```

**Benefits:**
- Testable via mocks
- Swappable implementations
- Clear contracts

### Services Layer

**Location:** `src/graft/services/`

**Purpose:** Use cases with explicit dependency injection.

**Key services:**
- **`ExplainService`** — Parse and return artifact configuration
- **`RunService`** — Execute derivations (templates, containers)
- **`StatusService`** — Detect dirty artifacts, compute impact
- **`FinalizeService`** — Create provenance, record attestation
- **`OrchestratorService`** — Manage DVC autosync

**Service pattern:**
```python
class RunService:
    def __init__(
        self,
        config_adapter: ConfigAdapter,
        fs: LocalFileSystem,
        material_loader: LocalMaterialLoader,
        container_adapter: DockerAdapter
    ):
        self.config_adapter = config_adapter
        self.fs = fs
        self.material_loader = material_loader
        self.container_adapter = container_adapter

    def run(self, artifact_path: Path, derivation_id: Optional[str]) -> RunResult:
        # Load configuration
        # Load materials
        # Evaluate template
        # Execute transformer
        # Return result
        ...
```

**Dependency injection at CLI startup:**
```python
# In cli.py
fs = LocalFileSystem()
config_adapter = ConfigAdapter(fs)
material_loader = LocalMaterialLoader(fs)
container_adapter = DockerAdapter()
run_service = RunService(config_adapter, fs, material_loader, container_adapter)
```

**Benefits:**
- Explicit dependencies
- Testable in isolation
- Business logic separated from infrastructure

### CLI Layer

**Location:** `src/graft/cli.py`

**Purpose:** Thin presentation layer using Typer.

**Responsibilities:**
- Parse command-line arguments
- Call appropriate service methods
- Format output (human-readable or JSON)
- Handle errors, exit codes

**Example:**
```python
@app.command()
def run(
    artifact: str,
    id: Optional[str] = None,
    sync: Optional[str] = None
):
    try:
        artifact_path = _artifact_path(artifact)
        result = run_service.run(artifact_path, derivation_id=id)

        if result.success:
            typer.echo(f"Run complete: {result.output_path}")
        else:
            typer.echo(f"Error: {result.error}", err=True)
            raise typer.Exit(code=1)
    except Exception as e:
        typer.echo(f"Error: {e}", err=True)
        raise typer.Exit(code=2)
```

**CLI is the contract** — All tests invoke via subprocess, asserting on JSON output and exit codes.

## Key Design Decisions

### File-First Model

**Decision:** Outputs are committed files, not cached artifacts.

**Rationale:**
- Git provides versioning, diff, review
- PRs are natural review mechanism
- Files can be directly edited
- No separate cache management
- Provenance lives alongside outputs

**Trade-offs:**
- Git repo size grows (mitigate with DVC for large files)
- Can't "rebuild from scratch" without git history
- Conflicts require manual resolution (but so do any file conflicts)

**Benefit:** Normal editing stays normal. Files are the source of truth.

### DVC for Orchestration

**Decision:** Use DVC to manage dependency DAG, not custom orchestrator.

**Rationale:**
- DVC is mature, well-tested
- Handles parallelization, caching, incremental builds
- Integrates with existing workflows
- Community support, documentation
- We can focus on provenance/policy, not DAG execution

**Integration:**
- Graft generates `dvc.yaml` from `graft.yaml` files
- One DVC stage per derivation
- DVC calls `graft run <artifact> --id <derivation>`
- Graft records provenance, DVC manages execution

**Trade-offs:**
- Adds DVC as dependency
- Users might need to learn DVC concepts (mitigated: Graft commands are primary interface)
- DVC opinions (dvc.yaml format, .dvc directory structure)

**Benefit:** Proven orchestration without building our own.

### Protocol-Based Adapters

**Decision:** Use Protocol types for all external dependencies.

**Rationale:**
- Easy to mock for testing
- Swappable implementations (Docker → Podman, DVC → Airflow)
- Clear contracts
- Python typing support

**Example:**
```python
class ContainerPort(Protocol):
    def build_image(self, dockerfile: Path, image: str) -> str: ...
    def run_container(self, image: str, env: dict, volumes: dict) -> RunResult: ...
```

Allows testing services without Docker:
```python
class MockContainer:
    def build_image(self, dockerfile, image): return "mock-digest"
    def run_container(self, image, env, volumes): return RunResult(success=True)

# In tests
service = RunService(..., container_adapter=MockContainer())
```

**Benefit:** Testable architecture without complex mocking frameworks.

### Immutable Domain Objects

**Decision:** All domain entities are immutable dataclasses.

**Rationale:**
- Prevents accidental mutation bugs
- Thread-safe (if needed in future)
- Clear data flow (input → transformation → output)
- Easier to reason about

**Implementation:**
```python
@dataclass(frozen=True)
class Provenance:
    artifact: str
    derivation_id: str
    prepared_at: str
    finalized_at: str
    materials: Tuple[Material, ...]  # Immutable collection
    ...
```

**Benefit:** Fewer bugs, clearer semantics.

### Black-Box Testing

**Decision:** All tests invoke `graft` CLI via subprocess, no internal imports.

**Rationale:**
- Tests the actual contract (CLI interface)
- Forces us to keep CLI stable
- Prevents coupling tests to implementation
- JSON output is testable API
- Exit codes verify error handling

**Test structure:**
```python
def test_run_sprint_brief(tmp_path):
    # Copy fixture to tmp_path
    shutil.copytree("examples/agile-ops", tmp_path / "agile-ops")

    # Run graft CLI
    result = subprocess.run(
        ["graft", "run", "agile-ops/artifacts/sprint-brief/", "--json"],
        cwd=tmp_path,
        capture_output=True,
        text=True
    )

    # Assert exit code
    assert result.returncode == 0

    # Assert JSON output
    data = json.loads(result.stdout)
    assert data["artifact"] == "sprint-brief"
    assert "derivations" in data

    # Assert outputs exist
    assert (tmp_path / "agile-ops/artifacts/sprint-brief/brief.md").exists()
```

**Benefit:** Tests verify user experience, not implementation details.

## Data Flow

### Run Flow

```
1. User: graft run artifacts/sprint-brief/
           ↓
2. CLI: Parse args, call RunService.run()
           ↓
3. RunService:
   - ConfigAdapter.load_config() → Artifact
   - MaterialLoader.load_materials() → List[Material]
   - Evaluate template (if present)
   - ContainerAdapter.run() OR TemplateRenderer.render()
   - FileSystem.write_output()
   - Create run record in .graft/runs/
           ↓
4. CLI: Print result, exit 0
```

### Finalize Flow

```
1. User: graft finalize artifacts/sprint-brief/ --agent "Jane"
           ↓
2. CLI: Parse args, call FinalizeService.finalize()
           ↓
3. FinalizeService:
   - Load latest run record
   - Verify outputs exist
   - Compute output hashes
   - Create Provenance object:
     - Materials (paths, hashes, git refs)
     - Template (source, evaluated hash)
     - Transformer (image digest, params)
     - Outputs (paths, hashes)
     - Attestation (agent, role, timestamp)
     - Policy (deterministic, attest, direct_edit)
   - FileSystem.write_json(provenance)
           ↓
4. CLI: Print "Finalized", exit 0
```

### Status Flow

```
1. User: graft status artifacts/sprint-brief/
           ↓
2. CLI: Call StatusService.status()
           ↓
3. StatusService:
   - Load config, provenance
   - Compare material hashes (current vs. recorded)
   - Detect if materials changed
   - Compute downstream impact (what depends on this artifact)
   - Return StatusResult
           ↓
4. CLI: Print status (dirty/clean, what changed, downstream impact)
```

## Extension Points

### Adding New Transformers

To add transformer types (e.g., LLM-based synthesis):

1. **Define protocol** in `adapters/`:
```python
class LLMPort(Protocol):
    def generate(self, prompt: str, model: str) -> str: ...
```

2. **Implement adapter**:
```python
class ClaudeAdapter:
    def generate(self, prompt, model):
        # Call Anthropic API
        ...
```

3. **Update RunService**:
```python
class RunService:
    def __init__(self, ..., llm_adapter: LLMPort):
        self.llm = llm_adapter

    def run(self, ...):
        if derivation.transformer.type == "llm":
            result = self.llm.generate(...)
```

4. **Update graft.yaml schema**:
```yaml
transformer:
  llm:
    model: claude-sonnet-4
    max_tokens: 8000
```

### Adding New Orchestrators

To support orchestrators beyond DVC (e.g., Airflow):

1. **Define protocol**:
```python
class OrchestratorPort(Protocol):
    def scaffold(self, artifacts: List[Artifact]) -> ScaffoldResult: ...
    def detect_drift(self) -> DriftResult: ...
```

2. **Implement adapter**:
```python
class AirflowAdapter:
    def scaffold(self, artifacts):
        # Generate airflow DAG Python file
        ...
```

3. **Update config schema**:
```yaml
orchestrator:
  type: airflow  # or dvc
  dag_file: dags/graft_pipeline.py
```

Protocol-based design makes this straightforward.

### Adding New Commands

To add CLI commands:

1. **Create service method** (if needed)
2. **Add CLI command** in `cli.py`:
```python
@app.command()
def mycommand(artifact: str):
    result = my_service.do_something(artifact)
    print_json(result.to_dict())
```

3. **Update tests**:
```python
def test_mycommand():
    result = subprocess.run(["graft", "mycommand", "artifacts/foo/", "--json"], ...)
    assert result.returncode == 0
```

## Directory Structure

```
graft/
├── src/graft/
│   ├── __init__.py
│   ├── cli.py                    # CLI layer (Typer commands)
│   ├── utils.py                  # Shared utilities
│   ├── domain/                   # Domain entities
│   │   ├── artifact.py
│   │   ├── material.py
│   │   ├── derivation.py
│   │   ├── policy.py
│   │   ├── provenance.py
│   │   └── orchestrator.py
│   ├── adapters/                 # External interfaces
│   │   ├── filesystem.py         # LocalFileSystem
│   │   ├── config.py             # ConfigAdapter
│   │   ├── docker.py             # DockerAdapter
│   │   ├── materials.py          # MaterialLoader
│   │   └── orchestrator.py       # DVCAdapter
│   └── services/                 # Use cases
│       ├── explain.py            # ExplainService
│       ├── run.py                # RunService
│       ├── status.py             # StatusService
│       ├── finalize.py           # FinalizeService
│       └── orchestrator.py       # OrchestratorService
├── tests/                        # Black-box subprocess tests
│   ├── conftest.py               # Pytest fixtures
│   ├── test_explain.py
│   ├── test_run.py
│   ├── test_status_finalize.py
│   └── ...
├── examples/                     # Reference artifacts
│   └── agile-ops/
│       └── artifacts/
│           ├── sprint-brief/
│           └── backlog/
├── docs/                         # Documentation
└── pyproject.toml
```

## Key Invariants

**Artifact uniqueness** — Artifact names are unique within a repository.

**Provenance atomicity** — Outputs + provenance committed together in one git commit.

**Policy enforcement** — Finalize fails if policy violated (missing attestation, outputs don't match, etc.).

**Immutable provenance** — Once finalized, provenance is not modified (new finalize = new provenance record).

**Git as ledger** — All changes flow through git; no side-channel state.

## Performance Considerations

**Material loading** — Materials are loaded once per run, hashed, and validated. For large files, this can be slow. Future: cache material hashes.

**Template evaluation** — Jinja2 is fast for small templates. For complex templates with many materials, consider optimization.

**Container builds** — Docker builds can be slow. Use layer caching, multi-stage builds, and avoid rebuilding unnecessarily.

**Provenance serialization** — JSON is human-readable but verbose. For high-frequency workflows, consider compressed or binary formats.

**DVC overhead** — DVC adds orchestration overhead. For single-artifact workflows, direct `graft run` may be faster than `dvc repro`.

## Security Considerations

**Container isolation** — Containers run with mounted materials and output directories. Transformers can read/write within those bounds. Future: stricter security (read-only materials, limited network).

**Provenance integrity** — Provenance files are plain JSON, can be tampered. Future: cryptographic signatures for attestation.

**Remote materials** — Fetching from remote URLs requires trust. Pin to specific revisions, verify hashes.

**Secrets** — Never commit secrets to git. Use environment variables, git-ignored config files, or secret management systems.

## Future Architecture Enhancements

**Distributed execution** — Run transformers on remote workers (Kubernetes, cloud functions).

**LLM integration** — First-class support for LLM-based transformations with prompt management, caching, cost tracking.

**Signed provenance** — GPG or other signing for tamper-evident audit trails.

**Event streaming** — Publish events (finalize, material change) to message queues for downstream automation.

**Plugins** — Allow third-party transformer types, adapters, custom commands.

---

This architecture provides a solid foundation for Graft's file-first, provenance-tracked workflows while remaining flexible for future enhancements.

Next: See [Philosophy of Design](philosophy-of-design.md) for the principles guiding these decisions.
