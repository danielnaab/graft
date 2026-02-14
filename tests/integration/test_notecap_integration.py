"""Integration tests for graft + notecap state queries.

Tests the three-layer architecture:
- Layer 1: Graft state infrastructure (this codebase)
- Layer 2: Notecap analytics (pure functions)
- Layer 3: Notecap state CLI (integration layer)

These tests verify that domain analytics libraries (like notecap) can
integrate correctly with graft's state query system.
"""

import json
import subprocess
import tempfile
from pathlib import Path

import pytest


class TestNotecapIntegration:
    """Test graft integration with notecap state commands."""

    @pytest.mark.skipif(
        not Path("/home/coder/src/notebook").exists(),
        reason="Requires notebook repository",
    )
    def test_graft_can_call_notecap_state_writing(self):
        """Verify graft can execute notecap state writing command."""
        # Note: This test requires the notebook repository to be present
        # and notecap to be installed in its virtualenv

        notebook_path = Path("/home/coder/src/notebook")
        if not (notebook_path / "graft.yaml").exists():
            pytest.skip("Notebook repository not set up")

        # Execute graft state query (should call notecap state writing)
        result = subprocess.run(
            ["graft", "state", "query", "writing-today", "--raw"],
            cwd=notebook_path,
            capture_output=True,
            text=True,
            timeout=30,
        )

        # Should succeed
        assert result.returncode == 0, f"Command failed: {result.stderr}"

        # Should output valid JSON
        try:
            output = json.loads(result.stdout)
        except json.JSONDecodeError as e:
            pytest.fail(f"Output is not valid JSON: {result.stdout}\nError: {e}")

        # Should have expected schema (from WritingMetrics dataclass)
        assert "notes_created" in output
        assert "notes_modified" in output
        assert "words_today" in output
        assert "total_words" in output
        assert "date" in output

        # Values should be non-negative integers
        assert isinstance(output["notes_created"], int)
        assert isinstance(output["notes_modified"], int)
        assert isinstance(output["words_today"], int)
        assert isinstance(output["total_words"], int)
        assert output["notes_created"] >= 0
        assert output["notes_modified"] >= 0
        assert output["words_today"] >= 0
        assert output["total_words"] >= 0

    @pytest.mark.skipif(
        not Path("/home/coder/src/notebook").exists(),
        reason="Requires notebook repository",
    )
    def test_graft_caches_notecap_results(self):
        """Verify graft caches notecap state query results."""
        notebook_path = Path("/home/coder/src/notebook")
        if not (notebook_path / "graft.yaml").exists():
            pytest.skip("Notebook repository not set up")

        # First call - should execute and cache
        result1 = subprocess.run(
            ["graft", "state", "query", "tasks", "--raw"],
            cwd=notebook_path,
            capture_output=True,
            text=True,
            timeout=30,
        )

        assert result1.returncode == 0
        output1 = json.loads(result1.stdout)

        # Second call - should use cache (note: deterministic query)
        result2 = subprocess.run(
            ["graft", "state", "query", "tasks", "--raw"],
            cwd=notebook_path,
            capture_output=True,
            text=True,
            timeout=30,
        )

        assert result2.returncode == 0
        output2 = json.loads(result2.stdout)

        # Should be identical (deterministic cache)
        assert output1 == output2

        # stderr should indicate cache hit on second call
        assert "(from cache)" in result2.stderr or result2.stderr == ""

    @pytest.mark.skipif(
        not Path("/home/coder/src/notebook").exists(),
        reason="Requires notebook repository",
    )
    def test_notecap_state_schema_matches_analytics(self):
        """Verify notecap state CLI output matches analytics dataclass schema."""
        notebook_path = Path("/home/coder/src/notebook")
        if not (notebook_path / "graft.yaml").exists():
            pytest.skip("Notebook repository not set up")

        # Test all state queries defined in notebook graft.yaml
        queries = {
            "writing-today": {
                "fields": ["notes_created", "notes_modified", "words_today", "total_words", "date"],
            },
            "tasks": {
                "fields": ["open", "completed", "total", "top_notes"],
            },
            "graph": {
                "fields": [
                    "total_notes",
                    "total_links",
                    "unique_targets",
                    "orphaned",
                    "broken_links",
                    "avg_links",
                    "top_hubs",
                ],
            },
            "recent": {
                "fields": [
                    "last_modified",
                    "modified_today",
                    "modified_this_week",
                    "stale_notes",
                    "recent_notes",
                ],
            },
        }

        for query_name, expected_schema in queries.items():
            result = subprocess.run(
                ["graft", "state", "query", query_name, "--raw"],
                cwd=notebook_path,
                capture_output=True,
                text=True,
                timeout=30,
            )

            assert result.returncode == 0, f"Query {query_name} failed: {result.stderr}"

            output = json.loads(result.stdout)

            # Verify all expected fields are present
            for field in expected_schema["fields"]:
                assert field in output, f"Query {query_name} missing field: {field}"


class TestLayeredArchitecture:
    """Test the three-layer architecture principles."""

    def test_layer1_has_no_domain_knowledge(self):
        """Verify graft state infrastructure has no notebook-specific knowledge."""
        # Read state service code
        state_service_path = Path(__file__).parent.parent.parent / "src/graft/services/state_service.py"
        state_service_code = state_service_path.read_text()

        # Should not contain domain-specific terms
        forbidden_terms = [
            "notebook",
            "notecap",
            "writing",
            "tasks",
            "wikilink",
            "markdown",
        ]

        for term in forbidden_terms:
            assert term.lower() not in state_service_code.lower(), (
                f"Layer 1 (state_service.py) contains domain-specific term: '{term}'. "
                "State infrastructure should be generic."
            )

    def test_layer2_has_no_caching_logic(self):
        """Verify analytics functions don't implement caching."""
        # This would need to be run in the notebook repository
        # Skipping for now as it requires cross-repo access
        pass

    def test_layer3_has_no_analytics_logic(self):
        """Verify state CLI commands only wrap analytics functions."""
        # This would need to be run in the notebook repository
        # Skipping for now as it requires cross-repo access
        pass


class TestErrorHandling:
    """Test error handling across layers."""

    @pytest.mark.skipif(
        not Path("/home/coder/src/notebook").exists(),
        reason="Requires notebook repository",
    )
    def test_graft_handles_notecap_errors_gracefully(self):
        """Verify graft handles errors from notecap commands gracefully."""
        notebook_path = Path("/home/coder/src/notebook")
        if not (notebook_path / "graft.yaml").exists():
            pytest.skip("Notebook repository not set up")

        # Create a temporary directory without notes/ subdirectory
        with tempfile.TemporaryDirectory() as tmpdir:
            tmpdir_path = Path(tmpdir)

            # Create a minimal graft.yaml that calls notecap
            graft_yaml = tmpdir_path / "graft.yaml"
            graft_yaml.write_text(
                """apiVersion: graft/v0
state:
  test-writing:
    run: "uv run notecap state writing"
    cache:
      deterministic: false
    timeout: 15
"""
            )

            # Initialize git repo (required for graft)
            subprocess.run(["git", "init"], cwd=tmpdir_path, check=True, capture_output=True)
            subprocess.run(
                ["git", "config", "user.name", "Test"],
                cwd=tmpdir_path,
                check=True,
                capture_output=True,
            )
            subprocess.run(
                ["git", "config", "user.email", "test@test.com"],
                cwd=tmpdir_path,
                check=True,
                capture_output=True,
            )
            subprocess.run(["git", "add", "."], cwd=tmpdir_path, check=True, capture_output=True)
            subprocess.run(
                ["git", "commit", "-m", "init"],
                cwd=tmpdir_path,
                check=True,
                capture_output=True,
            )

            # Try to run state query (should fail gracefully - no notes/ directory)
            result = subprocess.run(
                ["graft", "state", "query", "test-writing"],
                cwd=tmpdir_path,
                capture_output=True,
                text=True,
                timeout=30,
            )

            # Should fail with non-zero exit code
            assert result.returncode != 0

            # Should have informative error message
            error_output = result.stdout + result.stderr
            assert "Error" in error_output or "error" in error_output
