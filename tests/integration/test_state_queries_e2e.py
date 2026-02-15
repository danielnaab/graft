"""End-to-end integration tests for state queries.

Tests the complete state query workflow including:
- Command execution via subprocess
- Cache read/write cycles
- Temporal queries with git worktrees
- CLI commands (query, list, invalidate)
- Error handling and edge cases
"""

import json
import subprocess
import tempfile
import time
from pathlib import Path

import pytest


@pytest.fixture
def git_repo_with_state_queries():
    """Create a git repository with graft.yaml containing state queries."""
    with tempfile.TemporaryDirectory() as tmpdir:
        repo_dir = Path(tmpdir)

        # Initialize git repository
        subprocess.run(["git", "init"], cwd=repo_dir, check=True, capture_output=True)
        subprocess.run(
            ["git", "config", "user.email", "test@example.com"],
            cwd=repo_dir,
            check=True,
            capture_output=True,
        )
        subprocess.run(
            ["git", "config", "user.name", "Test User"],
            cwd=repo_dir,
            check=True,
            capture_output=True,
        )

        # Create graft.yaml with state queries
        graft_yaml = repo_dir / "graft.yaml"
        graft_yaml.write_text("""apiVersion: graft/v0

metadata:
  name: test-repo

state:
  simple-query:
    run: 'echo ''{"status": "ok", "value": 42}'''
    cache:
      deterministic: true

  timestamp-query:
    run: 'date +''{"timestamp": "%s"}'''
    cache:
      deterministic: false

  failing-query:
    run: exit 1
    cache:
      deterministic: true

  invalid-json-query:
    run: echo "not json"
    cache:
      deterministic: true

  slow-query:
    run: 'sleep 2 && echo ''{"result": "slow"}'''
    cache:
      deterministic: true
    timeout: 5

  very-slow-query:
    run: sleep 10
    cache:
      deterministic: true
    timeout: 1
""")

        # Create initial commit
        subprocess.run(["git", "add", "."], cwd=repo_dir, check=True, capture_output=True)
        subprocess.run(
            ["git", "commit", "-m", "Initial commit"],
            cwd=repo_dir,
            check=True,
            capture_output=True,
        )

        # Create second commit with modified query
        graft_yaml.write_text(graft_yaml.read_text() + """
  new-query:
    run: 'echo ''{"new": true}'''
    cache:
      deterministic: true
""")
        subprocess.run(["git", "add", "."], cwd=repo_dir, check=True, capture_output=True)
        subprocess.run(
            ["git", "commit", "-m", "Add new query"],
            cwd=repo_dir,
            check=True,
            capture_output=True,
        )

        yield repo_dir


class TestStateQueryExecution:
    """Test basic state query execution."""

    def test_execute_simple_query(self, git_repo_with_state_queries):
        """Should execute query and return JSON result."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "state", "query", "simple-query"],
            cwd=git_repo_with_state_queries,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0
        output = json.loads(result.stdout)
        assert output["data"]["status"] == "ok"
        assert output["data"]["value"] == 42
        assert output["metadata"]["query_name"] == "simple-query"
        assert output["metadata"]["deterministic"] is True

    def test_execute_query_with_raw_output(self, git_repo_with_state_queries):
        """Should output only data with --raw flag."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "state", "query", "simple-query", "--raw"],
            cwd=git_repo_with_state_queries,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0
        output = json.loads(result.stdout)
        assert "status" in output
        assert "value" in output
        assert "metadata" not in output

    def test_execute_failing_query(self, git_repo_with_state_queries):
        """Should handle command failure gracefully."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "state", "query", "failing-query"],
            cwd=git_repo_with_state_queries,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 1
        assert "failed" in result.stdout.lower() or "error" in result.stdout.lower()

    def test_execute_invalid_json_query(self, git_repo_with_state_queries):
        """Should fail with clear error for invalid JSON."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "state", "query", "invalid-json-query"],
            cwd=git_repo_with_state_queries,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 1
        assert "json" in result.stdout.lower()

    def test_execute_nonexistent_query(self, git_repo_with_state_queries):
        """Should fail with helpful error for missing query."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "state", "query", "nonexistent"],
            cwd=git_repo_with_state_queries,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 1
        assert "not found" in result.stdout.lower()
        assert "simple-query" in result.stdout  # Should list available queries


class TestStateCaching:
    """Test caching behavior."""

    def test_query_caches_result(self, git_repo_with_state_queries):
        """Should cache deterministic query results."""
        # Clear any existing cache first
        subprocess.run(
            ["uv", "run", "python", "-m", "graft", "state", "invalidate", "--all"],
            cwd=git_repo_with_state_queries,
            capture_output=True,
        )

        # First execution - not cached
        result1 = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "state", "query", "simple-query"],
            cwd=git_repo_with_state_queries,
            capture_output=True,
            text=True,
        )
        assert result1.returncode == 0
        assert "(from cache)" not in result1.stderr

        # Second execution - should use cache
        result2 = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "state", "query", "simple-query"],
            cwd=git_repo_with_state_queries,
            capture_output=True,
            text=True,
        )
        assert result2.returncode == 0
        assert "(from cache)" in result2.stderr

        # Results should be identical
        output1 = json.loads(result1.stdout)
        output2 = json.loads(result2.stdout)
        assert output1["data"] == output2["data"]

    def test_refresh_invalidates_cache(self, git_repo_with_state_queries):
        """Should re-execute query with --refresh flag."""
        # Execute and cache
        subprocess.run(
            ["uv", "run", "python", "-m", "graft", "state", "query", "timestamp-query"],
            cwd=git_repo_with_state_queries,
            capture_output=True,
            text=True,
        )

        time.sleep(0.1)  # Ensure timestamp difference

        # Refresh should execute again
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "state", "query", "timestamp-query", "--refresh"],
            cwd=git_repo_with_state_queries,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0
        assert "(from cache)" not in result.stderr

    def test_invalidate_specific_query_cache(self, git_repo_with_state_queries):
        """Should invalidate cache for specific query."""
        # Execute and cache
        subprocess.run(
            ["uv", "run", "python", "-m", "graft", "state", "query", "simple-query"],
            cwd=git_repo_with_state_queries,
            capture_output=True,
        )

        # Invalidate cache
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "state", "invalidate", "simple-query"],
            cwd=git_repo_with_state_queries,
            capture_output=True,
            text=True,
        )
        assert result.returncode == 0

        # Next execution should not use cache
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "state", "query", "simple-query"],
            cwd=git_repo_with_state_queries,
            capture_output=True,
            text=True,
        )
        assert "(from cache)" not in result.stderr

    def test_invalidate_all_caches(self, git_repo_with_state_queries):
        """Should invalidate all query caches."""
        # Execute and cache multiple queries
        subprocess.run(
            ["uv", "run", "python", "-m", "graft", "state", "query", "simple-query"],
            cwd=git_repo_with_state_queries,
            capture_output=True,
        )
        subprocess.run(
            ["uv", "run", "python", "-m", "graft", "state", "query", "timestamp-query"],
            cwd=git_repo_with_state_queries,
            capture_output=True,
        )

        # Invalidate all
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "state", "invalidate", "--all"],
            cwd=git_repo_with_state_queries,
            capture_output=True,
            text=True,
        )
        assert result.returncode == 0

        # Both queries should not use cache
        result1 = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "state", "query", "simple-query"],
            cwd=git_repo_with_state_queries,
            capture_output=True,
            text=True,
        )
        result2 = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "state", "query", "timestamp-query"],
            cwd=git_repo_with_state_queries,
            capture_output=True,
            text=True,
        )
        assert "(from cache)" not in result1.stderr
        assert "(from cache)" not in result2.stderr


class TestStateList:
    """Test listing state queries."""

    def test_list_queries(self, git_repo_with_state_queries):
        """Should list all defined queries."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "state", "list"],
            cwd=git_repo_with_state_queries,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0
        assert "simple-query" in result.stdout
        assert "timestamp-query" in result.stdout
        assert "failing-query" in result.stdout

    def test_list_shows_cache_status(self, git_repo_with_state_queries):
        """Should show cache status for queries."""
        # Execute one query to cache it
        subprocess.run(
            ["uv", "run", "python", "-m", "graft", "state", "query", "simple-query"],
            cwd=git_repo_with_state_queries,
            capture_output=True,
        )

        # List should show cache status
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "state", "list"],
            cwd=git_repo_with_state_queries,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0
        assert "simple-query" in result.stdout
        # Should show cached status for simple-query
        output_lines = result.stdout.split("\n")
        simple_query_section = False
        for line in output_lines:
            if "simple-query" in line:
                simple_query_section = True
            if simple_query_section and "Cached" in line:
                assert "Yes" in line
                break


class TestTemporalQueries:
    """Test temporal query execution at historical commits."""

    def test_query_at_previous_commit(self, git_repo_with_state_queries):
        """Should execute query at previous commit using worktree."""
        # Get HEAD~1 commit hash
        result = subprocess.run(
            ["git", "rev-parse", "HEAD~1"],
            cwd=git_repo_with_state_queries,
            capture_output=True,
            text=True,
            check=True,
        )
        prev_commit = result.stdout.strip()

        # Query at previous commit (before new-query was added)
        result = subprocess.run(
            [
                "uv",
                "run",
                "python",
                "-m",
                "graft",
                "state",
                "query",
                "simple-query",
                "--commit",
                prev_commit,
            ],
            cwd=git_repo_with_state_queries,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0
        output = json.loads(result.stdout)
        assert output["data"]["status"] == "ok"
        assert output["metadata"]["commit_hash"] == prev_commit

    def test_query_at_head_notation(self, git_repo_with_state_queries):
        """Should support HEAD~N notation."""
        result = subprocess.run(
            [
                "uv",
                "run",
                "python",
                "-m",
                "graft",
                "state",
                "query",
                "simple-query",
                "--commit",
                "HEAD~1",
            ],
            cwd=git_repo_with_state_queries,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0
        output = json.loads(result.stdout)
        assert output["data"]["status"] == "ok"

    def test_temporal_query_with_dirty_tree(self, git_repo_with_state_queries):
        """Should fail temporal query with uncommitted changes."""
        # Create uncommitted change
        test_file = git_repo_with_state_queries / "test.txt"
        test_file.write_text("uncommitted change")

        result = subprocess.run(
            [
                "uv",
                "run",
                "python",
                "-m",
                "graft",
                "state",
                "query",
                "simple-query",
                "--commit",
                "HEAD~1",
            ],
            cwd=git_repo_with_state_queries,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 1
        assert "uncommitted changes" in result.stdout.lower()

    def test_temporal_query_caching(self, git_repo_with_state_queries):
        """Should cache temporal query results."""
        # Execute temporal query
        result1 = subprocess.run(
            [
                "uv",
                "run",
                "python",
                "-m",
                "graft",
                "state",
                "query",
                "simple-query",
                "--commit",
                "HEAD~1",
            ],
            cwd=git_repo_with_state_queries,
            capture_output=True,
            text=True,
        )
        assert result1.returncode == 0

        # Second execution should use cache
        result2 = subprocess.run(
            [
                "uv",
                "run",
                "python",
                "-m",
                "graft",
                "state",
                "query",
                "simple-query",
                "--commit",
                "HEAD~1",
            ],
            cwd=git_repo_with_state_queries,
            capture_output=True,
            text=True,
        )
        assert result2.returncode == 0
        assert "(from cache)" in result2.stderr


class TestQueryTimeout:
    """Test query timeout behavior."""

    def test_slow_query_completes(self, git_repo_with_state_queries):
        """Should complete slow query within timeout."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "state", "query", "slow-query"],
            cwd=git_repo_with_state_queries,
            capture_output=True,
            text=True,
            timeout=10,  # Test timeout, not query timeout
        )

        assert result.returncode == 0
        output = json.loads(result.stdout)
        assert output["data"]["result"] == "slow"

    def test_very_slow_query_times_out(self, git_repo_with_state_queries):
        """Should timeout query that exceeds configured timeout."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "state", "query", "very-slow-query"],
            cwd=git_repo_with_state_queries,
            capture_output=True,
            text=True,
            timeout=5,  # Test timeout
        )

        assert result.returncode == 1
        # Should mention timeout in error
        assert "timeout" in result.stdout.lower() or "timed out" in result.stdout.lower()


class TestErrorHandling:
    """Test error handling and edge cases."""

    def test_query_without_graft_yaml(self):
        """Should fail gracefully without graft.yaml."""
        with tempfile.TemporaryDirectory() as tmpdir:
            result = subprocess.run(
                ["uv", "run", "python", "-m", "graft", "state", "query", "test"],
                cwd=tmpdir,
                capture_output=True,
                text=True,
            )

            assert result.returncode == 1
            assert "graft.yaml" in result.stdout.lower() or "not found" in result.stdout.lower()

    def test_list_without_state_section(self):
        """Should handle missing state section gracefully."""
        with tempfile.TemporaryDirectory() as tmpdir:
            repo_dir = Path(tmpdir)

            # Create graft.yaml without state section
            graft_yaml = repo_dir / "graft.yaml"
            graft_yaml.write_text("""apiVersion: graft/v0
metadata:
  name: test
""")

            result = subprocess.run(
                ["uv", "run", "python", "-m", "graft", "state", "list"],
                cwd=repo_dir,
                capture_output=True,
                text=True,
            )

            assert result.returncode == 0
            assert "No state queries" in result.stdout
