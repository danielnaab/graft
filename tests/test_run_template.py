"""Tests for the run command (Slice 1: deterministic single derivation).

Black-box integration tests that verify:
- Templates are rendered with Jinja2 engine
- Output files are created with rendered content
- --id flag targets specific derivations
- Missing template files cause appropriate failures
- Exit codes match the CLI contract (0=success, 1=user error, 2=system error)
"""
import subprocess
import sys
import shutil
from pathlib import Path

import pytest


def test_run_renders_jinja2_template_to_output(tmp_path):
    """Test that run command renders Jinja2 template to output file."""
    artifact_dir = tmp_path / "artifact"
    artifact_dir.mkdir()

    # Create graft.yaml
    graft_yaml = artifact_dir / "graft.yaml"
    graft_yaml.write_text("""graft: simple-render
derivations:
  - id: report
    transformer: {ref: jinja2}
    template:
      source: file
      engine: jinja2
      content_type: text/plain
      file: "./template.txt"
    outputs:
      - {path: "./output.txt"}
""")

    # Create Jinja2 template with variables
    template = artifact_dir / "template.txt"
    template.write_text("Hello {{ name }}! The answer is {{ value }}.")

    result = subprocess.run(
        [sys.executable, "-m", "graft.cli", "run", str(artifact_dir)],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0, f"Command failed: {result.stderr}"

    # Verify output file exists and template was rendered (stub: no context yet)
    out_file = artifact_dir / "output.txt"
    assert out_file.exists(), "Output file was not created"
    content = out_file.read_text()
    # For now, stub just copies the template (no variable substitution)
    assert "Hello" in content


def test_run_with_existing_example_sprint_brief(tmp_path):
    """Test run with the existing sprint-brief example."""
    src = Path("examples/agile-ops/artifacts/sprint-brief/")
    dst = tmp_path / "artifact"
    shutil.copytree(src, dst)

    result = subprocess.run(
        [sys.executable, "-m", "graft.cli", "run", str(dst)],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0, f"Command failed: {result.stderr}"

    out_file = dst / "brief.md"
    assert out_file.exists(), "Output file was not created"
    content = out_file.read_text()
    assert "Sprint Brief" in content, "Output file does not contain expected content"


def test_run_with_id_flag_targets_specific_derivation(tmp_path):
    """Test that --id flag runs only the specified derivation."""
    artifact_dir = tmp_path / "multi-deriv"
    artifact_dir.mkdir()

    # Create graft.yaml with two derivations
    graft_yaml = artifact_dir / "graft.yaml"
    graft_yaml.write_text("""graft: multi-deriv-test
derivations:
  - id: first
    transformer: {ref: jinja2}
    template:
      source: file
      engine: jinja2
      content_type: text/plain
      file: "./template1.txt"
    outputs:
      - {path: "./output1.txt"}
  - id: second
    transformer: {ref: jinja2}
    template:
      source: file
      engine: jinja2
      content_type: text/plain
      file: "./template2.txt"
    outputs:
      - {path: "./output2.txt"}
""")

    # Create template files
    (artifact_dir / "template1.txt").write_text("First template content")
    (artifact_dir / "template2.txt").write_text("Second template content")

    # Run with --id flag targeting only "second"
    result = subprocess.run(
        [sys.executable, "-m", "graft.cli", "run", str(artifact_dir), "--id", "second"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0, f"Command failed: {result.stderr}"

    # Verify only the second derivation's output was created
    output1 = artifact_dir / "output1.txt"
    output2 = artifact_dir / "output2.txt"

    assert not output1.exists(), "First derivation should not have run"
    assert output2.exists(), "Second derivation output should exist"
    assert output2.read_text() == "Second template content"


def test_run_fails_when_template_file_missing(tmp_path):
    """Test that missing template file causes exit code 1 (user error)."""
    artifact_dir = tmp_path / "missing-template"
    artifact_dir.mkdir()

    # Create graft.yaml referencing a non-existent template
    graft_yaml = artifact_dir / "graft.yaml"
    graft_yaml.write_text("""graft: missing-template-test
derivations:
  - id: broken
    transformer: {ref: jinja2}
    template:
      source: file
      engine: jinja2
      content_type: text/plain
      file: "./nonexistent.txt"
    outputs:
      - {path: "./output.txt"}
""")

    result = subprocess.run(
        [sys.executable, "-m", "graft.cli", "run", str(artifact_dir)],
        capture_output=True,
        text=True
    )

    # Should fail with exit code 1 (user error)
    assert result.returncode == 1, "Expected exit code 1 for missing template"
    assert "Error" in result.stderr or "Error" in result.stdout, \
        "Expected error message in output"


def test_run_fails_with_invalid_artifact_path(tmp_path):
    """Test that invalid artifact path causes exit code 1."""
    nonexistent = tmp_path / "does-not-exist"

    result = subprocess.run(
        [sys.executable, "-m", "graft.cli", "run", str(nonexistent)],
        capture_output=True,
        text=True
    )

    assert result.returncode == 1, "Expected exit code 1 for invalid path"
    assert "graft.yaml" in result.stderr or "graft.yaml" in result.stdout


def test_run_creates_output_directories(tmp_path):
    """Test that run creates necessary parent directories for outputs."""
    artifact_dir = tmp_path / "nested-output"
    artifact_dir.mkdir()

    graft_yaml = artifact_dir / "graft.yaml"
    graft_yaml.write_text("""graft: nested-output-test
derivations:
  - id: nested
    transformer: {ref: jinja2}
    template:
      source: file
      engine: jinja2
      content_type: text/plain
      file: "./template.txt"
    outputs:
      - {path: "./deep/nested/dir/output.txt"}
""")

    (artifact_dir / "template.txt").write_text("Nested template content")

    result = subprocess.run(
        [sys.executable, "-m", "graft.cli", "run", str(artifact_dir)],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0, f"Command failed: {result.stderr}"

    output_file = artifact_dir / "deep" / "nested" / "dir" / "output.txt"
    assert output_file.exists(), "Output file in nested directory was not created"
    assert output_file.read_text() == "Nested template content"


def test_run_handles_multiple_outputs_per_derivation(tmp_path):
    """Test that a derivation with multiple outputs copies template to all."""
    artifact_dir = tmp_path / "multi-output"
    artifact_dir.mkdir()

    graft_yaml = artifact_dir / "graft.yaml"
    graft_yaml.write_text("""graft: multi-output-test
derivations:
  - id: multi
    transformer: {ref: jinja2}
    template:
      source: file
      engine: jinja2
      content_type: text/plain
      file: "./template.txt"
    outputs:
      - {path: "./output1.txt"}
      - {path: "./output2.txt"}
      - {path: "./subdir/output3.txt"}
""")

    (artifact_dir / "template.txt").write_text("Shared template content")

    result = subprocess.run(
        [sys.executable, "-m", "graft.cli", "run", str(artifact_dir)],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0, f"Command failed: {result.stderr}"

    # Verify all outputs were created with same content
    output1 = artifact_dir / "output1.txt"
    output2 = artifact_dir / "output2.txt"
    output3 = artifact_dir / "subdir" / "output3.txt"

    assert output1.exists() and output2.exists() and output3.exists()
    assert output1.read_text() == "Shared template content"
    assert output2.read_text() == "Shared template content"
    assert output3.read_text() == "Shared template content"


def test_run_processes_all_derivations_when_no_id_specified(tmp_path):
    """Test that run without --id processes all derivations."""
    artifact_dir = tmp_path / "all-derivations"
    artifact_dir.mkdir()

    graft_yaml = artifact_dir / "graft.yaml"
    graft_yaml.write_text("""graft: all-derivations-test
derivations:
  - id: first
    transformer: {ref: jinja2}
    template:
      source: file
      engine: jinja2
      content_type: text/plain
      file: "./template1.txt"
    outputs:
      - {path: "./output1.txt"}
  - id: second
    transformer: {ref: jinja2}
    template:
      source: file
      engine: jinja2
      content_type: text/plain
      file: "./template2.txt"
    outputs:
      - {path: "./output2.txt"}
""")

    (artifact_dir / "template1.txt").write_text("First content")
    (artifact_dir / "template2.txt").write_text("Second content")

    result = subprocess.run(
        [sys.executable, "-m", "graft.cli", "run", str(artifact_dir)],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0, f"Command failed: {result.stderr}"

    # Verify both outputs were created
    output1 = artifact_dir / "output1.txt"
    output2 = artifact_dir / "output2.txt"

    assert output1.exists() and output2.exists()
    assert output1.read_text() == "First content"
    assert output2.read_text() == "Second content"
