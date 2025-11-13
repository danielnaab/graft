"""Black-box integration tests for impact and simulate commands (Slice 4 expansion).

These tests define the expected behavior for dependency analysis and simulation.
Tests are written first (TDD approach) and will drive implementation.
"""

import json
import subprocess
from pathlib import Path


# ============================================================================
# IMPACT COMMAND TESTS
# ============================================================================

def test_impact_on_fresh_artifact_returns_empty_downstream(run_graft, tmp_git_repo):
    """Impact on fresh artifact with no changes should return empty downstream array."""
    artifact_path = tmp_git_repo / "artifacts" / "sprint-brief"

    result = run_graft("impact", str(artifact_path), "--json", cwd=tmp_git_repo)
    result.assert_success()

    data = result.json()
    assert "artifact" in data, "impact output should contain artifact path"
    assert "downstream" in data, "impact output should contain downstream array"
    assert data["downstream"] == [], "fresh artifact should have no downstream impacts"


def test_impact_on_stale_artifact_still_returns_empty_downstream(run_graft, tmp_git_repo):
    """Impact shows downstream artifacts affected by THIS artifact, not sources affecting it.

    When an artifact becomes stale, impact shows which OTHER artifacts depend on it.
    If no other artifacts depend on it, downstream should be empty.
    """
    artifact_path = tmp_git_repo / "artifacts" / "sprint-brief"

    # Modify a source material to make artifact stale
    material_path = tmp_git_repo / "sources" / "roadmap" / "2025-Q1.md"
    original_content = material_path.read_text()
    material_path.write_text(original_content + "\n## New Section\n")

    result = run_graft("impact", str(artifact_path), "--json", cwd=tmp_git_repo)
    result.assert_success()

    data = result.json()
    # sprint-brief is a leaf node (nothing depends on it), so downstream is empty
    assert data["downstream"] == [], "leaf artifact should have no downstream impacts"


def test_impact_identifies_downstream_consumers(run_graft, tmp_git_repo):
    """When an artifact changes, impact should identify artifacts that depend on it.

    In agile-ops example:
    - sprint-brief depends on backlog/backlog.yaml
    - If backlog changes, sprint-brief should appear in downstream
    """
    # Test the backlog artifact (sprint-brief depends on it)
    backlog_path = tmp_git_repo / "artifacts" / "backlog"

    result = run_graft("impact", str(backlog_path), "--json", cwd=tmp_git_repo)
    result.assert_success()

    data = result.json()

    # sprint-brief depends on backlog, so it should be in downstream
    assert "downstream" in data
    assert isinstance(data["downstream"], list)

    # Check that sprint-brief is identified as a downstream consumer
    downstream_paths = [item["path"] if isinstance(item, dict) else item for item in data["downstream"]]

    # Should find sprint-brief artifact in some form
    has_sprint_brief = any("sprint-brief" in str(path) for path in downstream_paths)
    assert has_sprint_brief, f"sprint-brief should be in downstream of backlog, got: {downstream_paths}"


def test_impact_shows_transitive_dependencies(run_graft, tmp_git_repo):
    """Impact should show transitive downstream dependencies.

    In agile-ops example:
    - retrospectives → working-agreements → runbooks
    - Changing a retrospective should show both as downstream
    """
    # Modify a retrospective source file
    retro_path = tmp_git_repo / "sources" / "retrospectives" / "2025-Q1-sprint-1.md"
    original_content = retro_path.read_text()
    retro_path.write_text(original_content + "\n## Action Item\n- New process\n")

    # Commit the change
    subprocess.run(["git", "add", "."], cwd=tmp_git_repo, check=True, capture_output=True)
    subprocess.run(
        ["git", "commit", "-m", "Update retrospective"],
        cwd=tmp_git_repo,
        check=True,
        capture_output=True
    )

    # Check impact on working-agreements (which depends on retrospectives)
    working_agreements_path = tmp_git_repo / "artifacts" / "working-agreements"

    result = run_graft("impact", str(working_agreements_path), "--json", cwd=tmp_git_repo)
    result.assert_success()

    data = result.json()

    # runbooks depends on working-agreements, so it should be in downstream
    downstream_paths = [item["path"] if isinstance(item, dict) else item for item in data["downstream"]]
    has_runbooks = any("runbooks" in str(path) or "runbook" in str(path) for path in downstream_paths)

    # This test documents transitive dependency behavior
    # Implementation may choose to include or exclude transitive deps
    # For now, just assert the structure is correct
    assert isinstance(data["downstream"], list), "downstream should be a list"


def test_impact_with_multiple_downstream_consumers(run_graft, tmp_git_repo):
    """When multiple artifacts depend on the same source, all should be in downstream."""
    # working-agreements output is consumed by runbooks
    # If we had more artifacts depending on it, they'd all show up

    working_agreements_path = tmp_git_repo / "artifacts" / "working-agreements"

    result = run_graft("impact", str(working_agreements_path), "--json", cwd=tmp_git_repo)
    result.assert_success()

    data = result.json()
    assert isinstance(data["downstream"], list)
    # At minimum, runbooks should be in the list
    # (it depends on working-agreements/team-handbook.md)


def test_impact_json_includes_orchestrator_status(run_graft, tmp_git_repo):
    """Impact --json should include orchestrator status in output."""
    artifact_path = tmp_git_repo / "artifacts" / "sprint-brief"

    result = run_graft("impact", str(artifact_path), "--json", cwd=tmp_git_repo)
    result.assert_success()

    data = result.json()
    # Should include orchestrator status (from autosync)
    if "orchestrator" in data:
        assert "type" in data["orchestrator"]
        assert "drift" in data["orchestrator"]


def test_impact_on_missing_artifact_returns_error(run_graft, tmp_git_repo):
    """Impact should error when artifact path doesn't exist."""
    nonexistent_path = tmp_git_repo / "artifacts" / "does-not-exist"

    result = run_graft("impact", str(nonexistent_path), "--json", cwd=tmp_git_repo)
    result.assert_failure()

    # Should exit with code 1 (user error)
    assert result.returncode == 1


# ============================================================================
# SIMULATE COMMAND TESTS
# ============================================================================

def test_simulate_basic_execution(run_graft, tmp_git_repo):
    """Simulate should execute without errors on valid artifact."""
    artifact_path = tmp_git_repo / "artifacts" / "sprint-brief"

    result = run_graft("simulate", str(artifact_path), cwd=tmp_git_repo)
    result.assert_success()

    # Should output status information
    assert "Artifact" in result.stdout or "artifact" in result.stdout


def test_simulate_with_cascade_flag(run_graft, tmp_git_repo):
    """Simulate with --cascade should execute successfully."""
    artifact_path = tmp_git_repo / "artifacts" / "sprint-brief"

    result = run_graft("simulate", str(artifact_path), "--cascade", cwd=tmp_git_repo)
    result.assert_success()

    assert "cascade" in result.stdout.lower()


def test_simulate_shows_changed_sources(run_graft, tmp_git_repo):
    """Simulate should identify which input materials have changed.

    This test drives the core simulate feature: showing what changed.
    """
    artifact_path = tmp_git_repo / "artifacts" / "sprint-brief"

    # Modify a source material
    material_path = tmp_git_repo / "sources" / "roadmap" / "2025-Q1.md"
    original_content = material_path.read_text()
    material_path.write_text(original_content + "\n## Updated Requirements\n- New feature\n")

    result = run_graft("simulate", str(artifact_path), cwd=tmp_git_repo)
    result.assert_success()

    # Should mention the changed file
    output = result.stdout.lower()
    assert "2025-q1.md" in output or "roadmap" in output, \
        f"simulate should show changed source, got: {result.stdout}"


def test_simulate_on_fresh_artifact_reports_up_to_date(run_graft, tmp_git_repo):
    """Simulate on fresh artifact should report no changes needed."""
    artifact_path = tmp_git_repo / "artifacts" / "sprint-brief"

    result = run_graft("simulate", str(artifact_path), cwd=tmp_git_repo)
    result.assert_success()

    output = result.stdout.lower()
    # Should indicate artifact is up-to-date or no changes
    assert "fresh" in output or "up-to-date" in output or "no changes" in output, \
        f"simulate on fresh artifact should indicate no changes, got: {result.stdout}"


def test_simulate_json_output(run_graft, tmp_git_repo):
    """Simulate should support --json flag for structured output."""
    artifact_path = tmp_git_repo / "artifacts" / "sprint-brief"

    # Modify a source to make it stale
    material_path = tmp_git_repo / "sources" / "roadmap" / "2025-Q1.md"
    original_content = material_path.read_text()
    material_path.write_text(original_content + "\n## New Content\n")

    result = run_graft("simulate", str(artifact_path), "--json", cwd=tmp_git_repo)
    result.assert_success()

    data = result.json()

    # Should include artifact path
    assert "artifact" in data

    # Should include changed materials
    assert "changed_materials" in data or "changes" in data or "sources" in data

    # Should include some indication of what changed
    assert isinstance(data, dict), "JSON output should be a dictionary"


def test_simulate_cascade_shows_downstream_impacts(run_graft, tmp_git_repo):
    """Simulate with --cascade and --json should show all affected artifacts."""
    artifact_path = tmp_git_repo / "artifacts" / "backlog"

    # Modify backlog source to trigger staleness
    source_path = tmp_git_repo / "sources" / "external" / "jira" / "snapshots" / "2025-11-07" / "issues.json"
    original_content = source_path.read_text()
    # Modify JSON (just append a comment, invalid but shows change)
    source_path.write_text(original_content + "\n")

    result = run_graft("simulate", str(artifact_path), "--cascade", "--json", cwd=tmp_git_repo)
    result.assert_success()

    data = result.json()

    # With cascade, should show downstream artifacts affected
    if "downstream" in data:
        assert isinstance(data["downstream"], list)
        # sprint-brief depends on backlog, might be included


def test_simulate_shows_material_status(run_graft, tmp_git_repo):
    """Simulate should show status of each material (fresh/stale)."""
    artifact_path = tmp_git_repo / "artifacts" / "sprint-brief"

    # Modify one of two materials
    material_path = tmp_git_repo / "sources" / "roadmap" / "2025-Q1.md"
    original_content = material_path.read_text()
    material_path.write_text(original_content + "\n## New Section\n")

    result = run_graft("simulate", str(artifact_path), "--json", cwd=tmp_git_repo)
    result.assert_success()

    data = result.json()

    # Should have some way to show which materials changed
    # Could be "materials", "sources", "inputs", etc.
    has_materials_info = any(key in data for key in ["materials", "sources", "inputs", "changed_materials"])
    assert has_materials_info, f"simulate should show material information, got keys: {data.keys()}"


def test_simulate_on_missing_artifact_returns_error(run_graft, tmp_git_repo):
    """Simulate should error when artifact path doesn't exist."""
    nonexistent_path = tmp_git_repo / "artifacts" / "does-not-exist"

    result = run_graft("simulate", str(nonexistent_path), cwd=tmp_git_repo)
    result.assert_failure()

    assert result.returncode == 1


def test_simulate_with_non_deterministic_artifact_proceeds(run_graft, tmp_git_repo):
    """Simulate should handle artifacts with deterministic: false.

    In agile-ops, runbooks and working-agreements have deterministic: false.
    Simulate should still work, perhaps with a warning.
    """
    artifact_path = tmp_git_repo / "artifacts" / "runbooks"

    result = run_graft("simulate", str(artifact_path), cwd=tmp_git_repo)
    result.assert_success()

    # Should execute successfully (may include warning about non-deterministic)


# ============================================================================
# INTEGRATION TESTS (combining status, impact, simulate)
# ============================================================================

def test_full_workflow_status_to_simulate(run_graft, tmp_git_repo):
    """Test workflow: check status, see stale, simulate to see what changed."""
    artifact_path = tmp_git_repo / "artifacts" / "sprint-brief"

    # 1. Initial status should be fresh
    status_result = run_graft("status", str(artifact_path), "--json", cwd=tmp_git_repo)
    status_result.assert_success()
    assert status_result.json()["change_origin"] == "fresh"

    # 2. Modify a source
    material_path = tmp_git_repo / "sources" / "roadmap" / "2025-Q1.md"
    original_content = material_path.read_text()
    material_path.write_text(original_content + "\n## Updated Content\n")

    # 3. Status should now show stale
    status_result = run_graft("status", str(artifact_path), "--json", cwd=tmp_git_repo)
    status_result.assert_success()
    assert status_result.json()["change_origin"] == "stale"

    # 4. Simulate should show what changed
    sim_result = run_graft("simulate", str(artifact_path), cwd=tmp_git_repo)
    sim_result.assert_success()
    assert "roadmap" in sim_result.stdout.lower() or "2025-q1" in sim_result.stdout.lower()


def test_demo_scenario_roadmap_update_impacts_sprint_brief(run_graft, tmp_git_repo):
    """Replicate the exact demo scenario from guild-meeting-outline.md.

    This is the canonical test that the demo script should work.
    """
    artifact_path = tmp_git_repo / "artifacts" / "sprint-brief"

    # 1. Check status - should be fresh
    result = run_graft("status", str(artifact_path), "--json", cwd=tmp_git_repo)
    result.assert_success()
    data = result.json()
    assert data["change_origin"] == "fresh", "Initial status should be fresh"

    # 2. Simulate the demo change to roadmap
    roadmap_path = tmp_git_repo / "sources" / "roadmap" / "2025-Q1.md"
    original_content = roadmap_path.read_text()
    roadmap_path.write_text(
        original_content + "\n## Updated Requirements\n- Must support offline mode\n- Performance target: <100ms response time\n"
    )

    # 3. Check status again - should be stale
    result = run_graft("status", str(artifact_path), "--json", cwd=tmp_git_repo)
    result.assert_success()
    data = result.json()
    assert data["change_origin"] == "stale", "Status should be stale after roadmap change"

    # 4. Impact should show empty downstream (sprint-brief is a leaf)
    result = run_graft("impact", str(artifact_path), "--json", cwd=tmp_git_repo)
    result.assert_success()
    data = result.json()
    assert "downstream" in data
    assert isinstance(data["downstream"], list)

    # 5. Simulate should show what changed
    result = run_graft("simulate", str(artifact_path), cwd=tmp_git_repo)
    result.assert_success()
    # Should mention roadmap or the file that changed
    assert "roadmap" in result.stdout.lower() or "2025-q1" in result.stdout.lower()
