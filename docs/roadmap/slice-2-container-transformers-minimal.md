# Slice 2 — Container Transformers (Minimal)

**Status**: Planned
**Depends on**: Slice 1 (template rendering)
**Blocks**: Slice 3 (direct-edit + finalize/attest)

## Intent

Enable derivations to transform data using local container builds. Transformers run in containers with a simple file/stdin/environment interface. This completes the core derivation workflow by supporting both template rendering (Slice 1) and data transformation (Slice 2), making both example artifacts functional.

**Scope**: Minimal container support with Docker. Advanced features deferred to Slice 6.

## Problem Statement

The backlog artifact requires data transformation:

```yaml
# artifacts/backlog/graft.yaml
derivations:
  - id: normalize
    transformer: { ref: csv-tools, params: { mode: "jira->yaml" } }
    template: { source: inline, engine: none, content_type: application/json }
    inputs:
      materials:
        - { path: "../../sources/external/jira/latest/issues.json", rev: HEAD }
    outputs:
      - { path: "./backlog.yaml", schema: backlog }
```

This requires:
1. Reading JIRA JSON from materials
2. Running a transformer in a container
3. Writing transformed output

## Slice 2 Scope (Minimal Container Support)

### In Scope
- ✅ Local Dockerfile builds (one per artifact)
- ✅ Docker as the runtime (hardcoded)
- ✅ Basic container IO contract (files, env vars)
- ✅ Material loading and mounting
- ✅ Output validation
- ✅ Basic error messages
- ✅ Both examples working (sprint-brief + backlog)

### Deferred to Slice 6 (Production Features)
- ❌ Network isolation enforcement
- ❌ Build caching
- ❌ Run records and detailed logging
- ❌ Multiple runtime backends (Podman, Nerdctl)
- ❌ Determinism enforcement
- ❌ Image digest tracking
- ❌ Build args support

## Acceptance Criteria

1. **Container build + run**
   - Given artifact with `Dockerfile` and `transformer.build`
   - When `graft run artifacts/backlog/`
   - Then Docker builds image and runs container
   - All outputs written
   - Exit code 0

2. **Material loading**
   - Materials from `inputs.materials` mounted into container
   - Paths accessible to container process
   - Missing materials → exit code 1

3. **Output validation**
   - All declared outputs must exist after container runs
   - Missing outputs → exit code 1 with clear error

4. **Both examples functional**
   - `graft run artifacts/sprint-brief/` → template rendering (Slice 1)
   - `graft run artifacts/backlog/` → container transformation (Slice 2)
   - Both complete successfully

5. **Docker not available**
   - If Docker not installed/running
   - Exit code 1 with helpful message: "Docker is required for transformer derivations"

6. **Build failures**
   - If Dockerfile build fails
   - Exit code 1 with Docker build output

## Contracts

### 1. graft.yaml (minimal)

```yaml
derivations:
  - id: normalize
    transformer:
      build:
        image: "graft-backlog:local"  # Required: local image tag
        context: "."                   # Optional: default is artifact directory
    inputs:
      materials:
        - { path: "../../sources/external/jira/latest/issues.json" }
    outputs:
      - { path: "./backlog.yaml" }
```

**Rules**:
- `transformer.build.image` is required
- Dockerfile assumed at `<context>/Dockerfile`
- `transformer.ref` (remote refs) not supported in Slice 2

### 2. Container IO Contract (Minimal)

**Environment Variables**:
- `GRAFT_ARTIFACT_DIR` — absolute path to artifact directory (e.g., `/workspace`)
- `GRAFT_PARAMS` — JSON string of `transformer.params` (or `{}`)
- `GRAFT_OUTPUTS` — JSON array of output paths that must be written
- `GRAFT_MATERIALS` — JSON array of material paths available

**Filesystem**:
- Artifact directory mounted at `/workspace` (read-write)
- Materials accessible relative to artifact directory
- Container must write to paths in `GRAFT_OUTPUTS`

**Exit Code**:
- 0 = success (outputs written)
- Non-zero = failure (captured in error message)

**Simplifications for Slice 2**:
- No stdin/stdout contract (use files only)
- No network isolation (runs with default Docker networking)
- No determinism enforcement (warning only)

### 3. Dockerfile Convention

Place `Dockerfile` in artifact directory:

```
artifacts/backlog/
├── Dockerfile
├── graft.yaml
└── backlog.yaml (output)
```

Example minimal Dockerfile:

```dockerfile
FROM python:3.11-slim
WORKDIR /workspace
COPY transform.py /transform.py
CMD ["python", "/transform.py"]
```

## Architecture

### Adapter Layer

**New: Docker Adapter** (`src/graft/adapters/docker.py`):

```python
class ContainerPort(Protocol):
    """Port for executing containers."""

    def build_image(
        self,
        dockerfile_path: Path,
        image_tag: str,
        context_path: Path
    ) -> None:
        """Build Docker image from Dockerfile."""
        ...

    def run_container(
        self,
        image_tag: str,
        working_dir: Path,
        env_vars: dict[str, str]
    ) -> tuple[int, str, str]:
        """Run container and return (exit_code, stdout, stderr)."""
        ...

class DockerAdapter:
    """Docker container execution."""

    def build_image(
        self,
        dockerfile_path: Path,
        image_tag: str,
        context_path: Path
    ) -> None:
        """Build Docker image using docker build."""
        import subprocess

        result = subprocess.run(
            ["docker", "build", "-t", image_tag, "-f", str(dockerfile_path), str(context_path)],
            capture_output=True,
            text=True
        )

        if result.returncode != 0:
            raise BuildError(f"Docker build failed: {result.stderr}")

    def run_container(
        self,
        image_tag: str,
        working_dir: Path,
        env_vars: dict[str, str]
    ) -> tuple[int, str, str]:
        """Run container with artifact directory mounted."""
        import subprocess

        # Build docker run command
        cmd = [
            "docker", "run", "--rm",
            "-v", f"{working_dir.absolute()}:/workspace",
            "-w", "/workspace"
        ]

        # Add environment variables
        for key, value in env_vars.items():
            cmd.extend(["-e", f"{key}={value}"])

        cmd.append(image_tag)

        result = subprocess.run(cmd, capture_output=True, text=True)

        return result.returncode, result.stdout, result.stderr
```

**New: Material Loader** (`src/graft/adapters/materials.py`):

```python
class MaterialPort(Protocol):
    """Port for loading materials."""

    def load_materials(
        self,
        artifact_path: Path,
        materials: list[Material]
    ) -> list[Path]:
        """Load materials and return their absolute paths."""
        ...

class LocalMaterialLoader:
    """Load materials from local filesystem."""

    def __init__(self, filesystem: FileSystemPort):
        self.filesystem = filesystem

    def load_materials(
        self,
        artifact_path: Path,
        materials: list[Material]
    ) -> list[Path]:
        """Resolve material paths relative to artifact."""
        material_paths = []

        for material in materials:
            material_path = artifact_path / material.path

            if not self.filesystem.exists(material_path):
                raise MaterialNotFoundError(f"Material not found: {material.path}")

            material_paths.append(material_path.absolute())

        return material_paths
```

### Service Layer

**Update RunService** (`src/graft/services/run.py`):

```python
class RunService:
    """Service for running artifact derivations."""

    def __init__(
        self,
        config_adapter: ConfigAdapter,
        filesystem: FileSystemPort,
        material_loader: MaterialPort,
        container_adapter: ContainerPort
    ):
        self.config_adapter = config_adapter
        self.filesystem = filesystem
        self.material_loader = material_loader
        self.container_adapter = container_adapter

    def _execute_derivation(
        self,
        artifact: Artifact,
        derivation: Derivation
    ) -> list[str]:
        """Execute derivation (template OR container)."""

        # Case 1: Template-based (Slice 1)
        if derivation.template and derivation.template.file:
            return self._execute_template_derivation(artifact, derivation)

        # Case 2: Container-based (Slice 2)
        if derivation.transformer and derivation.transformer.build:
            return self._execute_container_derivation(artifact, derivation)

        # Case 3: Neither (skip)
        return []

    def _execute_container_derivation(
        self,
        artifact: Artifact,
        derivation: Derivation
    ) -> list[str]:
        """Execute container-based transformation."""

        build_spec = derivation.transformer.build

        # 1. Build Docker image
        dockerfile_path = artifact.path / build_spec.context / "Dockerfile"
        context_path = artifact.path / build_spec.context

        if not self.filesystem.exists(dockerfile_path):
            raise FileNotFoundError(f"Dockerfile not found: {dockerfile_path}")

        self.container_adapter.build_image(
            dockerfile_path=dockerfile_path,
            image_tag=build_spec.image,
            context_path=context_path
        )

        # 2. Load materials
        material_paths = []
        if artifact.config.inputs and artifact.config.inputs.materials:
            material_paths = self.material_loader.load_materials(
                artifact.path,
                artifact.config.inputs.materials
            )

        # 3. Prepare environment
        env_vars = {
            "GRAFT_ARTIFACT_DIR": "/workspace",
            "GRAFT_PARAMS": json.dumps(derivation.transformer.params or {}),
            "GRAFT_OUTPUTS": json.dumps([
                f"/workspace/{output.path}" for output in derivation.outputs
            ]),
            "GRAFT_MATERIALS": json.dumps([
                f"/workspace/{m.path}" for m in artifact.config.inputs.materials
            ]) if artifact.config.inputs and artifact.config.inputs.materials else "[]"
        }

        # 4. Run container
        exit_code, stdout, stderr = self.container_adapter.run_container(
            image_tag=build_spec.image,
            working_dir=artifact.path,
            env_vars=env_vars
        )

        if exit_code != 0:
            raise TransformerExecutionError(
                f"Container execution failed (exit {exit_code}): {stderr}"
            )

        # 5. Validate outputs
        output_paths = []
        for output in derivation.outputs:
            output_path = artifact.path / output.path
            if not self.filesystem.exists(output_path):
                raise OutputMissingError(f"Output not created: {output.path}")
            output_paths.append(output.path)

        return output_paths
```

### Domain Layer

**Update Transformer entity** (`src/graft/domain/entities.py`):

```python
@dataclass(frozen=True)
class TransformerBuild:
    """Container build specification."""
    image: str
    context: str = "."

@dataclass(frozen=True)
class Transformer:
    """Transformer specification."""
    build: TransformerBuild | None = None
    params: dict[str, Any] = field(default_factory=dict)
```

### CLI Layer

**Update dependency injection** (`src/graft/cli.py`):

```python
from .adapters.docker import DockerAdapter
from .adapters.materials import LocalMaterialLoader

# Initialize adapters
fs = LocalFileSystem()
config_adapter = ConfigAdapter(fs)
material_loader = LocalMaterialLoader(fs)
container_adapter = DockerAdapter()

# Initialize services
run_service = RunService(config_adapter, fs, material_loader, container_adapter)
```

**Update error handling**:

```python
except BuildError as e:
    typer.echo(f"Error: Docker build failed: {e}", err=True)
    raise typer.Exit(code=1)
except TransformerExecutionError as e:
    typer.echo(f"Error: Transformer execution failed: {e}", err=True)
    raise typer.Exit(code=1)
except OutputMissingError as e:
    typer.echo(f"Error: {e}", err=True)
    raise typer.Exit(code=1)
```

## Example: Backlog Artifact

**artifacts/backlog/Dockerfile**:

```dockerfile
FROM python:3.11-slim

WORKDIR /workspace

# Install dependencies
RUN pip install --no-cache-dir pyyaml

# Copy transformer script
COPY transform.py /transform.py

# Run transformer
CMD ["python", "/transform.py"]
```

**artifacts/backlog/transform.py**:

```python
import json
import yaml
import os

# Read environment
artifact_dir = os.getenv("GRAFT_ARTIFACT_DIR", "/workspace")
params = json.loads(os.getenv("GRAFT_PARAMS", "{}"))
outputs = json.loads(os.getenv("GRAFT_OUTPUTS", "[]"))
materials = json.loads(os.getenv("GRAFT_MATERIALS", "[]"))

# Load JIRA JSON
jira_path = None
for material in materials:
    if "jira" in material.lower() and material.endswith(".json"):
        jira_path = material
        break

if not jira_path:
    print("Error: No JIRA material found")
    exit(1)

with open(jira_path, "r") as f:
    jira_data = json.load(f)

# Transform to backlog format
backlog = {"items": []}
for issue in jira_data.get("issues", []):
    backlog["items"].append({
        "id": issue.get("key"),
        "title": issue["fields"].get("summary"),
        "status": issue["fields"].get("status", {}).get("name")
    })

# Write output
output_path = outputs[0]  # First output
with open(output_path, "w") as f:
    yaml.dump(backlog, f, default_flow_style=False)

print(f"Transformed {len(backlog['items'])} items to {output_path}")
```

**artifacts/backlog/graft.yaml**:

```yaml
graft: backlog
inputs:
  materials:
    - { path: "../../sources/external/jira/latest/issues.json", rev: HEAD }
derivations:
  - id: normalize
    transformer:
      build:
        image: "graft-backlog:local"
        context: "."
      params:
        mode: "jira->yaml"
    outputs:
      - { path: "./backlog.yaml", schema: backlog }
    policy:
      deterministic: true
      attest: required
```

## Testing Strategy

**tests/test_container_transformers.py**:

```python
import subprocess
import sys
import json
import yaml
from pathlib import Path

def test_container_transformer_builds_and_runs(tmp_path):
    """Test that container transformer builds image and produces output."""
    artifact_dir = tmp_path / "container-artifact"
    artifact_dir.mkdir()

    # Create graft.yaml
    graft_yaml = artifact_dir / "graft.yaml"
    graft_yaml.write_text("""graft: test-container
inputs:
  materials:
    - { path: "./data.json", rev: HEAD }
derivations:
  - id: transform
    transformer:
      build:
        image: "graft-test:local"
        context: "."
    outputs:
      - { path: "./output.yaml" }
""")

    # Create Dockerfile
    dockerfile = artifact_dir / "Dockerfile"
    dockerfile.write_text("""FROM python:3.11-slim
WORKDIR /workspace
COPY transform.py /transform.py
CMD ["python", "/transform.py"]
""")

    # Create transformer script
    transform_script = artifact_dir / "transform.py"
    transform_script.write_text("""
import json
import yaml
import os

outputs = json.loads(os.getenv("GRAFT_OUTPUTS", "[]"))
materials = json.loads(os.getenv("GRAFT_MATERIALS", "[]"))

# Read material
with open(materials[0], "r") as f:
    data = json.load(f)

# Transform
result = {"count": len(data.get("items", []))}

# Write output
with open(outputs[0], "w") as f:
    yaml.dump(result, f)
""")

    # Create input material
    data_json = artifact_dir / "data.json"
    data_json.write_text(json.dumps({"items": ["a", "b", "c"]}))

    # Run graft
    result = subprocess.run(
        [sys.executable, "-m", "graft.cli", "run", str(artifact_dir)],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0, f"Command failed: {result.stderr}"

    # Verify output
    output_file = artifact_dir / "output.yaml"
    assert output_file.exists()

    output_data = yaml.safe_load(output_file.read_text())
    assert output_data["count"] == 3

def test_backlog_artifact_with_container(tmp_path):
    """Test real backlog artifact with container transformation."""
    # Copy backlog artifact
    src = Path("examples/agile-ops/artifacts/backlog/")
    dst = tmp_path / "backlog"
    shutil.copytree(src, dst)

    # Copy materials
    materials_src = Path("examples/agile-ops/sources/")
    materials_dst = tmp_path / "sources"
    shutil.copytree(materials_src, materials_dst)

    result = subprocess.run(
        [sys.executable, "-m", "graft.cli", "run", str(dst)],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0

    output = dst / "backlog.yaml"
    assert output.exists()
```

## Error Messages

New exception types:

```python
class BuildError(Exception):
    """Raised when Docker build fails."""
    pass

class TransformerExecutionError(Exception):
    """Raised when container execution fails."""
    pass

class OutputMissingError(Exception):
    """Raised when declared output not created."""
    pass

class MaterialNotFoundError(FileNotFoundError):
    """Raised when material file not found."""
    pass
```

All map to exit code 1 (user errors).

## What's Deferred to Slice 6

Slice 6 will add production-grade features:
- Network isolation enforcement (`--network=none`)
- Build caching (skip rebuild when unchanged)
- Run records (`.graft/runs/<id>/<timestamp>.json`)
- Detailed logging (`.graft/logs/<id>.log`)
- Multiple backends (Podman, Nerdctl)
- Build args support
- Image digest tracking
- Determinism enforcement

## Dependencies

**No new Python dependencies needed** (Docker must be installed):
- Docker CLI (external requirement)
- subprocess (stdlib)
- json (stdlib)

## Implementation Estimate

- ~300 lines of new code
- 2 new adapters (Docker, MaterialLoader)
- Update RunService (~100 lines)
- Update domain entities (~30 lines)
- New tests (~200 lines)
- Example Dockerfile + transform script (~50 lines)

**Total: ~680 lines of code** (vs. ~2000+ for full Slice 6)

## Summary

**Slice 2 delivers**:
- Container-based transformers (minimal)
- Both examples functional (sprint-brief + backlog)
- Docker as default transformer runtime
- Simple file/env interface
- Architecture validated

**Slice 6 will add**:
- Production features (caching, logging, etc.)
- Multiple backends
- Advanced policies
- Complex error handling
