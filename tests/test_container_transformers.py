"""Black-box tests for container-based transformers (Slice 2).

These tests use helper functions and builders from conftest_container.py
to reduce boilerplate and make the test pattern more apparent:

1. Build artifact with ArtifactBuilder (fluent API)
2. Run graft with run_graft_run() helper
3. Assert on results

The helpers provide:
- ArtifactBuilder: Fluent API for creating test artifacts
- run_graft_run(): Wrapper for subprocess.run with graft CLI
- transform_*(): Reusable transform script patterns

This pattern reduces code ~50% while making test structure clearer.
"""
import subprocess
import sys
import json
import yaml
import shutil
from pathlib import Path
import pytest

from conftest_container import (
    ArtifactBuilder,
    run_graft_run,
    transform_count_items,
    transform_with_params,
    transform_multi_output,
    transform_failing,
)


def test_container_transformer_builds_and_runs(tmp_path):
    """Test that container transformer builds image and produces output."""
    # Arrange: Build test artifact
    artifact = (ArtifactBuilder(tmp_path, "container-artifact")
        .with_graft_yaml("""graft: test-container
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
        .with_dockerfile(packages=["pyyaml"])
        .with_transform_script(transform_count_items())
        .with_material("data.json", {"items": ["a", "b", "c"]})
        .build())

    # Act: Run graft
    result = run_graft_run(artifact)

    # Assert: Check success and output
    assert result.returncode == 0, f"Command failed: {result.stderr}"

    output_file = artifact / "output.yaml"
    assert output_file.exists(), "Output file was not created"

    output_data = yaml.safe_load(output_file.read_text())
    assert output_data["count"] == 3


def test_container_transformer_with_params(tmp_path):
    """Test that params are passed to container via environment."""
    artifact = (ArtifactBuilder(tmp_path, "params-test")
        .with_graft_yaml("""graft: params-test
derivations:
  - id: with-params
    transformer:
      build:
        image: "graft-params-test:local"
        context: "."
      params:
        multiplier: 10
        prefix: "result"
    outputs:
      - { path: "./output.json" }
""")
        .with_dockerfile()
        .with_transform_script(transform_with_params())
        .build())

    result = run_graft_run(artifact)

    assert result.returncode == 0, f"Command failed: {result.stderr}"

    # Verify params were used
    output_file = artifact / "output.json"
    output_data = json.loads(output_file.read_text())
    assert output_data["value"] == 20
    assert output_data["label"] == "result_20"


def test_container_transformer_missing_dockerfile(tmp_path):
    """Test that missing Dockerfile returns exit code 1."""
    artifact = (ArtifactBuilder(tmp_path, "no-dockerfile")
        .with_graft_yaml("""graft: no-dockerfile
derivations:
  - id: missing
    transformer:
      build:
        image: "graft-missing:local"
    outputs:
      - { path: "./output.txt" }
""")
        .build())  # Note: No dockerfile!

    result = run_graft_run(artifact)

    assert result.returncode == 1
    assert "Dockerfile not found" in result.stderr


def test_container_transformer_missing_material(tmp_path):
    """Test that missing material returns exit code 1."""
    artifact = (ArtifactBuilder(tmp_path, "missing-material")
        .with_graft_yaml("""graft: missing-material
inputs:
  materials:
    - { path: "./nonexistent.json", rev: HEAD }
derivations:
  - id: transform
    transformer:
      build:
        image: "graft-test:local"
    outputs:
      - { path: "./output.yaml" }
""")
        .with_dockerfile(content="FROM python:3.11-slim\nCMD ['echo', 'test']")
        .build())

    result = run_graft_run(artifact)

    assert result.returncode == 1
    assert "Material not found" in result.stderr


def test_container_transformer_missing_output(tmp_path):
    """Test that container not creating output returns exit code 1."""
    artifact = (ArtifactBuilder(tmp_path, "missing-output")
        .with_graft_yaml("""graft: missing-output
derivations:
  - id: no-output
    transformer:
      build:
        image: "graft-no-output:local"
    outputs:
      - { path: "./output.txt" }
""")
        .with_dockerfile(content="""FROM python:3.11-slim
WORKDIR /workspace
CMD ["echo", "I forgot to write the output"]
""")
        .build())

    result = run_graft_run(artifact)

    assert result.returncode == 1
    assert "Output not created" in result.stderr


def test_container_transformer_nonzero_exit(tmp_path):
    """Test that container exiting with non-zero returns exit code 1."""
    artifact = (ArtifactBuilder(tmp_path, "failing-container")
        .with_graft_yaml("""graft: failing-container
derivations:
  - id: fails
    transformer:
      build:
        image: "graft-fails:local"
    outputs:
      - { path: "./output.txt" }
""")
        .with_dockerfile()
        .with_transform_script(transform_failing())
        .build())

    result = run_graft_run(artifact)

    assert result.returncode == 1
    assert "Container execution failed" in result.stderr


def test_backlog_artifact_with_container(tmp_path):
    """Test real backlog artifact with container transformation."""
    # Copy entire project structure to maintain relative paths
    src = Path("examples/agile-ops/")
    dst = tmp_path / "agile-ops"
    shutil.copytree(src, dst)

    # Run from the backlog artifact directory
    backlog_dir = dst / "artifacts" / "backlog"

    result = subprocess.run(
        [sys.executable, "-m", "graft.cli", "run", str(backlog_dir)],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0, f"Command failed: {result.stderr}"

    output = backlog_dir / "backlog.yaml"
    assert output.exists(), "Backlog output was not created"

    # Verify output structure
    backlog_data = yaml.safe_load(output.read_text())
    assert "items" in backlog_data
    assert isinstance(backlog_data["items"], list)
    # Should have transformed the 3 JIRA issues
    assert len(backlog_data["items"]) == 3


def test_both_examples_work(tmp_path):
    """Test that both sprint-brief (template) and backlog (container) work."""
    # Copy entire example project
    src = Path("examples/agile-ops/")
    dst = tmp_path / "agile-ops"
    shutil.copytree(src, dst)

    # Test sprint-brief (template-based, Slice 1)
    sprint_brief = dst / "artifacts" / "sprint-brief"
    result1 = subprocess.run(
        [sys.executable, "-m", "graft.cli", "run", str(sprint_brief)],
        capture_output=True,
        text=True,
        cwd=dst
    )
    assert result1.returncode == 0, f"sprint-brief failed: {result1.stderr}"
    assert (sprint_brief / "brief.md").exists()

    # Test backlog (container-based, Slice 2)
    backlog = dst / "artifacts" / "backlog"
    result2 = subprocess.run(
        [sys.executable, "-m", "graft.cli", "run", str(backlog)],
        capture_output=True,
        text=True,
        cwd=dst
    )
    assert result2.returncode == 0, f"backlog failed: {result2.stderr}"
    assert (backlog / "backlog.yaml").exists()


def test_container_with_multiple_outputs(tmp_path):
    """Test container transformer creating multiple outputs."""
    artifact = (ArtifactBuilder(tmp_path, "multi-output")
        .with_graft_yaml("""graft: multi-output
derivations:
  - id: multi
    transformer:
      build:
        image: "graft-multi:local"
    outputs:
      - { path: "./output1.txt" }
      - { path: "./output2.txt" }
      - { path: "./subdir/output3.txt" }
""")
        .with_dockerfile()
        .with_transform_script(transform_multi_output())
        .build())

    result = run_graft_run(artifact)

    assert result.returncode == 0, f"Command failed: {result.stderr}"
    assert (artifact / "output1.txt").exists()
    assert (artifact / "output2.txt").exists()
    assert (artifact / "subdir" / "output3.txt").exists()
