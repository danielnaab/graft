"""Black-box integration tests for status and finalize commands (Slice 3)."""

import json
import subprocess
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


def test_status_detects_fresh_artifact(run_graft, tmp_git_repo):
    """Test status shows 'fresh' when input materials haven't changed."""
    artifact_path = tmp_git_repo / "artifacts" / "sprint-brief"

    # Check status without modifying any materials
    result = run_graft("status", str(artifact_path), "--json", cwd=tmp_git_repo)
    result.assert_success()

    data = result.json()
    assert data["change_origin"] == "fresh", "status should be 'fresh' when materials unchanged"


def test_status_detects_stale_artifact_after_material_change(run_graft, tmp_git_repo):
    """Test status shows 'stale' when an input material has been modified."""
    artifact_path = tmp_git_repo / "artifacts" / "sprint-brief"

    # First verify it's fresh
    result = run_graft("status", str(artifact_path), "--json", cwd=tmp_git_repo)
    result.assert_success()
    assert result.json()["change_origin"] == "fresh"

    # Modify one of the input materials
    material_path = tmp_git_repo / "sources" / "roadmap" / "2025-Q1.md"
    original_content = material_path.read_text()
    material_path.write_text(original_content + "\n## New Section\n- New requirement\n")

    # Check status again - should now be stale
    result = run_graft("status", str(artifact_path), "--json", cwd=tmp_git_repo)
    result.assert_success()

    data = result.json()
    assert data["change_origin"] == "stale", "status should be 'stale' after material modification"


def test_status_returns_fresh_after_commit(run_graft, tmp_git_repo):
    """Test status returns fresh after changes are committed."""
    artifact_path = tmp_git_repo / "artifacts" / "sprint-brief"

    # Modify a material
    material_path = tmp_git_repo / "sources" / "roadmap" / "2025-Q1.md"
    original_content = material_path.read_text()
    material_path.write_text(original_content + "\n## New Content\n")

    # Should be stale now
    result = run_graft("status", str(artifact_path), "--json", cwd=tmp_git_repo)
    result.assert_success()
    assert result.json()["change_origin"] == "stale"

    # Commit the change
    subprocess.run(["git", "add", "."], cwd=tmp_git_repo, check=True)
    subprocess.run(
        ["git", "commit", "-m", "Update roadmap"],
        cwd=tmp_git_repo,
        check=True,
        capture_output=True
    )

    # After commit, HEAD now points to the new commit, so artifact is fresh again
    # (since the material's rev: HEAD references the latest commit)
    result = run_graft("status", str(artifact_path), "--json", cwd=tmp_git_repo)
    result.assert_success()

    data = result.json()
    assert data["change_origin"] == "fresh", "status should be 'fresh' after committing material change"
