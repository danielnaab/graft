"""Shared fixtures and helpers for container transformer tests.

This module provides minimal abstractions to reduce boilerplate while
keeping test intent clear and explicit.
"""
import subprocess
import sys
import json
import os
from pathlib import Path
from dataclasses import dataclass
from typing import Optional


# ============================================================================
# Helpers: Reduce Boilerplate Without Hiding Logic
# ============================================================================

def run_graft_run(artifact_dir: Path, **kwargs) -> subprocess.CompletedProcess:
    """Run `graft run` command against an artifact directory.

    Args:
        artifact_dir: Path to artifact directory
        **kwargs: Additional arguments to subprocess.run (e.g., cwd)

    Returns:
        CompletedProcess with returncode, stdout, stderr
    """
    # Set PYTHONPATH to include src/ so graft module can be imported
    env = kwargs.pop("env", None) or os.environ.copy()
    src_path = str(Path(__file__).parent.parent / "src")
    if "PYTHONPATH" in env:
        env["PYTHONPATH"] = f"{src_path}:{env['PYTHONPATH']}"
    else:
        env["PYTHONPATH"] = src_path

    return subprocess.run(
        [sys.executable, "-m", "graft.cli", "run", str(artifact_dir)],
        capture_output=True,
        text=True,
        env=env,
        **kwargs
    )


def create_minimal_dockerfile(base_image: str = "python:3.11-slim",
                              pip_packages: list[str] | None = None) -> str:
    """Generate standard Dockerfile for container transformers.

    Args:
        base_image: Docker base image
        pip_packages: Python packages to install via pip

    Returns:
        Dockerfile contents as string
    """
    lines = [
        f"FROM {base_image}",
        "WORKDIR /workspace"
    ]

    if pip_packages:
        pkg_list = " ".join(pip_packages)
        lines.append(f"RUN pip install --no-cache-dir {pkg_list}")

    lines.extend([
        "COPY transform.py /transform.py",
        'CMD ["python", "/transform.py"]'
    ])

    return "\n".join(lines) + "\n"


# ============================================================================
# Artifact Builder: Fluent API for Test Artifact Construction
# ============================================================================

@dataclass
class ArtifactBuilder:
    """Builder for test artifacts. Makes the setup pattern explicit.

    Usage:
        artifact = (ArtifactBuilder(tmp_path, "my-test")
            .with_graft_yaml(graft_config)
            .with_dockerfile(packages=["pyyaml"])
            .with_transform_script(script_content)
            .with_material("data.json", {"items": [1, 2, 3]})
            .build())
    """

    tmp_path: Path
    artifact_name: str
    _graft_yaml: Optional[str] = None
    _dockerfile: Optional[str] = None
    _transform_script: Optional[str] = None
    _materials: dict[str, str] = None  # filename -> content

    def __post_init__(self):
        if self._materials is None:
            self._materials = {}

    def with_graft_yaml(self, content: str) -> "ArtifactBuilder":
        """Set graft.yaml content."""
        self._graft_yaml = content
        return self

    def with_dockerfile(self, content: str | None = None,
                       packages: list[str] | None = None) -> "ArtifactBuilder":
        """Set Dockerfile content (explicit) or generate standard one."""
        if content is not None:
            self._dockerfile = content
        else:
            self._dockerfile = create_minimal_dockerfile(pip_packages=packages)
        return self

    def with_transform_script(self, content: str) -> "ArtifactBuilder":
        """Set transform.py content."""
        self._transform_script = content
        return self

    def with_material(self, filename: str, content: dict | str) -> "ArtifactBuilder":
        """Add a material file.

        Args:
            filename: Name of material file
            content: Dict (will be JSON encoded) or string
        """
        if isinstance(content, dict):
            self._materials[filename] = json.dumps(content)
        else:
            self._materials[filename] = content
        return self

    def build(self) -> Path:
        """Build the artifact directory structure.

        Returns:
            Path to created artifact directory
        """
        artifact_dir = self.tmp_path / self.artifact_name
        artifact_dir.mkdir()

        # Write required files
        if self._graft_yaml:
            (artifact_dir / "graft.yaml").write_text(self._graft_yaml)

        if self._dockerfile:
            (artifact_dir / "Dockerfile").write_text(self._dockerfile)

        if self._transform_script:
            (artifact_dir / "transform.py").write_text(self._transform_script)

        # Write materials
        for filename, content in self._materials.items():
            (artifact_dir / filename).write_text(content)

        return artifact_dir


# ============================================================================
# Common Transform Scripts: Reusable Building Blocks
# ============================================================================

def transform_count_items() -> str:
    """Transform that counts items in a JSON material.

    Reads first material, counts items, writes YAML output.
    """
    return """
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
"""


def transform_with_params() -> str:
    """Transform that uses GRAFT_PARAMS."""
    return """
import json
import os

params = json.loads(os.getenv("GRAFT_PARAMS", "{}"))
outputs = json.loads(os.getenv("GRAFT_OUTPUTS", "[]"))

result = {
    "value": params["multiplier"] * 2,
    "label": params["prefix"] + "_20"
}

with open(outputs[0], "w") as f:
    json.dump(result, f)
"""


def transform_multi_output() -> str:
    """Transform that writes multiple outputs."""
    return """
import json
import os
from pathlib import Path

outputs = json.loads(os.getenv("GRAFT_OUTPUTS", "[]"))

for output in outputs:
    output_path = Path(output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(f"Content for {output_path.name}")
"""


def transform_failing() -> str:
    """Transform that always fails."""
    return """
import sys
print("Something went wrong!", file=sys.stderr)
sys.exit(1)
"""
