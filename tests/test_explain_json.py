"""Tests for 'graft explain' command with JSON output.

Slice 0 acceptance criteria:
- Contract: `explain --json` returns `{ artifact, graft, policy?, inputs?, derivations[] }`.
- Errors: missing `graft.yaml` → CLI exit code != 0 with helpful message.
"""

import json
from pathlib import Path


def test_explain_json_success(tmp_repo, run_graft):
    """Test that 'explain --json' returns expected structure for valid artifact."""
    artifact_path = tmp_repo / "artifacts" / "sprint-brief"

    result = run_graft("explain", str(artifact_path), "--json", cwd=tmp_repo)
    result.assert_success()

    # Validate JSON structure matches contract
    data = result.json()
    assert data["graft"] == "sprint-brief"
    assert "derivations" in data
    assert isinstance(data["derivations"], list)
    assert len(data["derivations"]) > 0


def test_explain_missing_graft_yaml(empty_repo, run_graft):
    """Test that missing graft.yaml produces non-zero exit with helpful message."""
    artifact_path = empty_repo / "artifacts" / "missing"
    artifact_path.mkdir(parents=True)

    result = run_graft("explain", str(artifact_path), "--json", cwd=empty_repo)
    result.assert_failure()

    # Should have helpful error message
    assert "graft.yaml" in result.stderr or "graft.yaml" in result.stdout


def test_explain_json_has_required_fields(tmp_repo, run_graft):
    """Test that JSON output contains all required contract fields."""
    artifact_path = tmp_repo / "artifacts" / "sprint-brief"

    result = run_graft("explain", str(artifact_path), "--json", cwd=tmp_repo)

    data = result.json()

    # Required fields per contract
    assert "artifact" in data
    assert "graft" in data
    assert "derivations" in data

    # derivations should be a non-empty list
    assert isinstance(data["derivations"], list)
    assert len(data["derivations"]) > 0


def test_explain_derivations_are_objects_not_strings(tmp_repo, run_graft):
    """Test that derivations array contains full objects, not just ID strings.

    CRITICAL: Per contract, derivations should be objects with structure:
    { id, transformer, outputs, template?, policy? }
    """
    artifact_path = tmp_repo / "artifacts" / "sprint-brief"

    result = run_graft("explain", str(artifact_path), "--json", cwd=tmp_repo)
    data = result.json()

    derivations = data["derivations"]
    assert len(derivations) > 0, "Should have at least one derivation"

    # Each derivation must be a dict/object, not a string
    first_deriv = derivations[0]
    assert isinstance(first_deriv, dict), (
        f"Derivation should be object, got {type(first_deriv).__name__}: {first_deriv}"
    )

    # Required fields in derivation object
    assert "id" in first_deriv, "Derivation must have 'id' field"
    assert first_deriv["id"] == "brief", "Expected derivation id 'brief'"

    assert "transformer" in first_deriv, "Derivation must have 'transformer' field"
    assert isinstance(first_deriv["transformer"], dict), "transformer should be object"

    assert "outputs" in first_deriv, "Derivation must have 'outputs' field"
    assert isinstance(first_deriv["outputs"], list), "outputs should be array"
    assert len(first_deriv["outputs"]) > 0, "Should have at least one output"

    # Validate output structure
    output = first_deriv["outputs"][0]
    assert isinstance(output, dict), "Output should be object"
    assert "path" in output, "Output must have 'path' field"


def test_explain_derivation_includes_template_when_present(tmp_repo, run_graft):
    """Test that derivation includes template field when present in graft.yaml."""
    artifact_path = tmp_repo / "artifacts" / "sprint-brief"

    result = run_graft("explain", str(artifact_path), "--json", cwd=tmp_repo)
    data = result.json()

    first_deriv = data["derivations"][0]

    # sprint-brief has a template, so it should be in the output
    assert "template" in first_deriv, "Derivation should include template when present"
    template = first_deriv["template"]
    assert isinstance(template, dict), "template should be object"
    assert "file" in template, "template should have 'file' field"
    assert "engine" in template, "template should have 'engine' field"


def test_explain_optional_policy_field(tmp_repo, run_graft):
    """Test that policy field is optional in output per contract (policy?)."""
    artifact_path = tmp_repo / "artifacts" / "sprint-brief"

    result = run_graft("explain", str(artifact_path), "--json", cwd=tmp_repo)
    data = result.json()

    # policy is optional (marked with ? in contract), but if present should be dict
    if "policy" in data:
        assert isinstance(data["policy"], dict), "policy should be object when present"
        # Validate policy structure
        assert "deterministic" in data["policy"]
        assert "network" in data["policy"]
        assert "attest" in data["policy"]


def test_explain_optional_inputs_field(tmp_repo, run_graft):
    """Test that inputs field is optional in output per contract (inputs?)."""
    artifact_path = tmp_repo / "artifacts" / "sprint-brief"

    result = run_graft("explain", str(artifact_path), "--json", cwd=tmp_repo)
    data = result.json()

    # inputs is optional (marked with ? in contract), but if present should be dict
    if "inputs" in data:
        assert isinstance(data["inputs"], dict), "inputs should be object when present"
        # If materials exist, validate structure
        if "materials" in data["inputs"]:
            assert isinstance(data["inputs"]["materials"], list)
            for material in data["inputs"]["materials"]:
                assert "path" in material, "material must have 'path'"
                assert "rev" in material, "material must have 'rev'"


def test_explain_minimal_graft_yaml(empty_repo, run_graft):
    """Test explain with minimal graft.yaml (only required fields)."""
    artifact_path = empty_repo / "minimal-artifact"
    artifact_path.mkdir(parents=True)

    # Create minimal graft.yaml with only required fields
    graft_yaml = artifact_path / "graft.yaml"
    graft_yaml.write_text("""graft: minimal
derivations:
  - id: test-deriv
    transformer: {ref: test}
    outputs:
      - {path: output.txt}
""")

    result = run_graft("explain", str(artifact_path), "--json", cwd=empty_repo)
    result.assert_success()

    data = result.json()
    assert data["graft"] == "minimal"
    assert len(data["derivations"]) == 1

    # Optional fields may be absent or have default values
    # Both are acceptable per contract (policy?, inputs?)


def test_explain_malformed_yaml(empty_repo, run_graft):
    """Test that malformed YAML produces non-zero exit with helpful message."""
    artifact_path = empty_repo / "malformed"
    artifact_path.mkdir(parents=True)

    # Create invalid YAML
    graft_yaml = artifact_path / "graft.yaml"
    graft_yaml.write_text("graft: test\n  invalid: - indentation\n[")

    result = run_graft("explain", str(artifact_path), "--json", cwd=empty_repo)
    result.assert_failure()

    # Should have error message about YAML parsing
    error_output = result.stderr + result.stdout
    assert "yaml" in error_output.lower() or "parse" in error_output.lower()


def test_explain_missing_required_field(empty_repo, run_graft):
    """Test that graft.yaml missing required 'graft' field fails gracefully."""
    artifact_path = empty_repo / "incomplete"
    artifact_path.mkdir(parents=True)

    # Create graft.yaml missing required 'graft' field
    graft_yaml = artifact_path / "graft.yaml"
    graft_yaml.write_text("""derivations:
  - id: test
    transformer: {ref: test}
    outputs: []
""")

    result = run_graft("explain", str(artifact_path), "--json", cwd=empty_repo)
    result.assert_failure()

    # Should have error message about missing field
    error_output = result.stderr + result.stdout
    assert "graft" in error_output.lower()


def test_explain_without_json_flag_is_human_readable(tmp_repo, run_graft):
    """Test that explain without --json flag outputs human-readable format, not JSON.

    CRITICAL: Default output should NOT be JSON. Only --json flag should output JSON.
    """
    artifact_path = tmp_repo / "artifacts" / "sprint-brief"

    result = run_graft("explain", str(artifact_path), cwd=tmp_repo)
    result.assert_success()

    # Try to parse as JSON - should fail or at least not be pretty-printed JSON
    # Human-readable output should have descriptive text, not pure JSON
    output = result.stdout.strip()

    # Check if it's formatted as indented JSON (which it shouldn't be)
    try:
        parsed = json.loads(output)
        # If we can parse it as JSON, the test should document this as wrong
        # The output should be human-readable, not JSON
        assert False, (
            "Default explain output should be human-readable, not JSON. "
            f"Got valid JSON: {output[:100]}..."
        )
    except json.JSONDecodeError:
        # Good! It's not JSON, which is what we want for human-readable output
        pass


def test_explain_error_message_includes_path(empty_repo, run_graft):
    """Test that error message for missing graft.yaml includes the expected path."""
    artifact_path = empty_repo / "missing-config"
    artifact_path.mkdir(parents=True)

    result = run_graft("explain", str(artifact_path), "--json", cwd=empty_repo)
    result.assert_failure()

    # Error message should mention where it looked for graft.yaml
    error_output = result.stderr + result.stdout
    assert "graft.yaml" in error_output
    # Should mention the actual path it was looking for
    assert "missing-config" in error_output or str(artifact_path) in error_output
