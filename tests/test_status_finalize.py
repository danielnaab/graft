"""Black-box integration tests for status and finalize commands (Slice 3)."""

import json
from pathlib import Path


def test_status_and_finalize(run_graft, tmp_repo, assert_json_file):
    """Test status command returns change_origin and downstream, finalize writes provenance."""
    # Use sprint-brief artifact
    artifact_path = tmp_repo / "artifacts" / "sprint-brief"

    # Direct edit an output file to simulate a change
    output_file = artifact_path / "brief.md"
    original_content = output_file.read_text()
    output_file.write_text(original_content + "\n- Added note\n", encoding="utf-8")

    # Test status command
    result = run_graft("status", str(artifact_path), "--json", cwd=tmp_repo)
    result.assert_success()

    data = result.json()
    assert "change_origin" in data, "status output should contain change_origin"
    assert "downstream" in data, "status output should contain downstream array"
    assert isinstance(data["downstream"], list), "downstream should be an array"

    # Test finalize command
    fin_result = run_graft(
        "finalize", str(artifact_path),
        "--agent", "Claude Code",
        "--model", "claude-3.7",
        cwd=tmp_repo
    )
    fin_result.assert_success()

    # Verify provenance file was created
    prov_file = artifact_path / ".graft" / "provenance" / "finalize.json"
    assert prov_file.exists(), "finalize should create provenance file"

    prov_data = assert_json_file(prov_file)
    assert "agent" in prov_data, "provenance should contain agent info"
    assert prov_data["agent"]["name"] == "Claude Code", "agent name should match"
    assert prov_data["agent"]["model"] == "claude-3.7", "model should match"
    assert "finalized_at" in prov_data, "provenance should contain timestamp"
