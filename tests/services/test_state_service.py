"""Tests for state service functions."""

import json
import subprocess
import tempfile
from pathlib import Path

import pytest

from graft.domain.state import StateCache, StateQuery
from graft.services.dependency_context import DependencyContext
from graft.services.state_service import (
    execute_state_query,
    execute_temporal_query,
    get_cache_path,
    get_state,
    invalidate_cached_state,
    read_cached_state,
    write_cached_state,
)


class TestCachePathGeneration:
    """Tests for cache path generation."""

    def test_get_cache_path_structure(self, dependency_context: DependencyContext):
        """Test cache path follows expected structure."""
        path = get_cache_path(
            dependency_context,
            workspace_name="test-workspace",
            repo_name="test-repo",
            query_name="coverage",
            commit_hash="abc123def456",
        )

        # Should be in home/.cache/graft/<workspace-hash>/repo/state/query/commit.json
        assert ".cache/graft" in str(path)
        assert "test-repo/state/coverage/abc123def456.json" in str(path)

    def test_get_cache_path_workspace_hash_consistency(self, dependency_context: DependencyContext):
        """Test same workspace name produces same hash."""
        path1 = get_cache_path(
            dependency_context,
            workspace_name="my-workspace",
            repo_name="repo",
            query_name="test",
            commit_hash="abc123",
        )
        path2 = get_cache_path(
            dependency_context,
            workspace_name="my-workspace",
            repo_name="repo",
            query_name="test",
            commit_hash="abc123",
        )

        assert path1 == path2


class TestCacheOperations:
    """Tests for cache read/write operations."""

    def test_write_and_read_cache(self, dependency_context: DependencyContext):
        """Test writing and reading cached state."""
        from datetime import UTC, datetime

        from graft.domain.state import StateResult

        # Create test result
        result = StateResult(
            query_name="test-query",
            commit_hash="abc123def",
            data={"test": "value", "number": 42},
            timestamp=datetime.now(UTC),
            command="echo test",
            deterministic=True,
            cached=False,
        )

        # Write to cache
        write_cached_state(
            dependency_context,
            workspace_name="test-workspace",
            repo_name="test-repo",
            result=result,
        )

        # Read from cache
        cached = read_cached_state(
            dependency_context,
            workspace_name="test-workspace",
            repo_name="test-repo",
            query_name="test-query",
            commit_hash="abc123def",
        )

        assert cached is not None
        assert cached.query_name == "test-query"
        assert cached.commit_hash == "abc123def"
        assert cached.data == {"test": "value", "number": 42}
        assert cached.command == "echo test"
        assert cached.deterministic is True
        assert cached.cached is True  # Loaded from cache

    def test_read_cache_nonexistent(self, dependency_context: DependencyContext):
        """Test reading nonexistent cache returns None."""
        cached = read_cached_state(
            dependency_context,
            workspace_name="test",
            repo_name="test",
            query_name="nonexistent",
            commit_hash="abc123",
        )

        assert cached is None

    def test_invalidate_specific_query(self, dependency_context: DependencyContext):
        """Test invalidating cache for specific query."""
        from datetime import UTC, datetime

        from graft.domain.state import StateResult

        # Create and cache two queries
        result1 = StateResult(
            query_name="query1",
            commit_hash="abc123",
            data={"test": 1},
            timestamp=datetime.now(UTC),
            command="echo 1",
            deterministic=True,
        )
        result2 = StateResult(
            query_name="query2",
            commit_hash="abc123",
            data={"test": 2},
            timestamp=datetime.now(UTC),
            command="echo 2",
            deterministic=True,
        )

        write_cached_state(dependency_context, "workspace", "repo", result1)
        write_cached_state(dependency_context, "workspace", "repo", result2)

        # Invalidate one query
        count = invalidate_cached_state(dependency_context, "workspace", "repo", "query1")

        assert count >= 1

        # query1 should be gone
        cached1 = read_cached_state(dependency_context, "workspace", "repo", "query1", "abc123")
        assert cached1 is None

        # query2 should still exist
        cached2 = read_cached_state(dependency_context, "workspace", "repo", "query2", "abc123")
        assert cached2 is not None

    def test_invalidate_all_queries(self, dependency_context: DependencyContext):
        """Test invalidating all cached state."""
        from datetime import UTC, datetime

        from graft.domain.state import StateResult

        # Create and cache queries
        result1 = StateResult(
            query_name="query1",
            commit_hash="abc123",
            data={"test": 1},
            timestamp=datetime.now(UTC),
            command="echo 1",
            deterministic=True,
        )
        result2 = StateResult(
            query_name="query2",
            commit_hash="def456",
            data={"test": 2},
            timestamp=datetime.now(UTC),
            command="echo 2",
            deterministic=True,
        )

        write_cached_state(dependency_context, "workspace", "repo", result1)
        write_cached_state(dependency_context, "workspace", "repo", result2)

        # Invalidate all
        count = invalidate_cached_state(dependency_context, "workspace", "repo", None)

        assert count >= 1

        # Both should be gone
        cached1 = read_cached_state(dependency_context, "workspace", "repo", "query1", "abc123")
        cached2 = read_cached_state(dependency_context, "workspace", "repo", "query2", "def456")
        assert cached1 is None
        assert cached2 is None


class TestExecuteStateQuery:
    """Tests for execute_state_query function."""

    def test_execute_query_success(self, dependency_context: DependencyContext, tmp_path: Path):
        """Test successful query execution."""
        # Create test repository
        repo_path = tmp_path / "test-repo"
        repo_path.mkdir()

        # Create query that outputs valid JSON
        query = StateQuery(
            name="test",
            run='echo \'{"result": "success", "value": 42}\'',
            cache=StateCache(deterministic=True),
        )

        # Execute query
        result = execute_state_query(
            dependency_context,
            query,
            str(repo_path),
            "abc123def",
        )

        assert result.query_name == "test"
        assert result.commit_hash == "abc123def"
        assert result.data == {"result": "success", "value": 42}
        assert result.command == query.run
        assert result.deterministic is True
        assert result.cached is False

    def test_execute_query_with_timeout(self, dependency_context: DependencyContext, tmp_path: Path):
        """Test query respects timeout setting."""
        repo_path = tmp_path / "test-repo"
        repo_path.mkdir()

        # Create query with very short timeout
        query = StateQuery(
            name="slow",
            run="sleep 10",  # Will timeout
            cache=StateCache(deterministic=True),
            timeout=1,  # 1 second timeout
        )

        # Execute query - should timeout or be killed
        with pytest.raises((subprocess.CalledProcessError, subprocess.TimeoutExpired)):
            execute_state_query(dependency_context, query, str(repo_path), "abc123")

    def test_execute_query_invalid_json(self, dependency_context: DependencyContext, tmp_path: Path):
        """Test query with invalid JSON output fails."""
        repo_path = tmp_path / "test-repo"
        repo_path.mkdir()

        query = StateQuery(
            name="bad-json",
            run="echo 'not json'",
            cache=StateCache(deterministic=True),
        )

        with pytest.raises(json.JSONDecodeError):
            execute_state_query(dependency_context, query, str(repo_path), "abc123")

    def test_execute_query_non_object_json(self, dependency_context: DependencyContext, tmp_path: Path):
        """Test query outputting JSON array fails."""
        repo_path = tmp_path / "test-repo"
        repo_path.mkdir()

        query = StateQuery(
            name="array-output",
            run="echo '[1, 2, 3]'",
            cache=StateCache(deterministic=True),
        )

        with pytest.raises(ValueError, match="must output a JSON object"):
            execute_state_query(dependency_context, query, str(repo_path), "abc123")

    def test_execute_query_command_failure(self, dependency_context: DependencyContext, tmp_path: Path):
        """Test query with failing command raises error."""
        repo_path = tmp_path / "test-repo"
        repo_path.mkdir()

        query = StateQuery(
            name="failing",
            run="exit 1",
            cache=StateCache(deterministic=True),
        )

        with pytest.raises(subprocess.CalledProcessError):
            execute_state_query(dependency_context, query, str(repo_path), "abc123")


class TestTemporalQueryExecution:
    """Tests for temporal query execution with git worktree."""

    def test_execute_temporal_query_creates_worktree(
        self, dependency_context: DependencyContext, tmp_path: Path
    ):
        """Test temporal query creates and cleans up worktree."""
        # Setup fake repository
        repo_path = str(tmp_path / "repo")
        dependency_context.git.clone("https://example.com/repo.git", repo_path, "main")

        # Create query
        query = StateQuery(
            name="test",
            run='echo \'{"test": "value"}\'',
            cache=StateCache(deterministic=True),
        )

        commit = "abc123def456abc123def456abc123def456abc123de"

        # Execute temporal query
        result = execute_temporal_query(
            dependency_context,
            query,
            repo_path,
            commit,
        )

        # Verify worktree was added
        worktree_calls = dependency_context.git.get_add_worktree_calls()
        assert len(worktree_calls) == 1
        assert worktree_calls[0][0] == repo_path
        assert worktree_calls[0][2] == commit

        # Verify worktree was removed
        remove_calls = dependency_context.git.get_remove_worktree_calls()
        assert len(remove_calls) == 1
        assert remove_calls[0][0] == repo_path

        # Verify result
        assert result.query_name == "test"
        assert result.commit_hash == commit
        assert result.data == {"test": "value"}

    def test_execute_temporal_query_cleans_up_on_failure(
        self, dependency_context: DependencyContext, tmp_path: Path
    ):
        """Test temporal query cleans up worktree even on failure."""
        repo_path = str(tmp_path / "repo")
        dependency_context.git.clone("https://example.com/repo.git", repo_path, "main")

        # Create query that fails
        query = StateQuery(
            name="failing",
            run="exit 1",
            cache=StateCache(deterministic=True),
        )

        commit = "abc123def456abc123def456abc123def456abc123de"

        # Execute temporal query - should fail
        with pytest.raises(subprocess.CalledProcessError):
            execute_temporal_query(dependency_context, query, repo_path, commit)

        # Worktree should still be removed
        remove_calls = dependency_context.git.get_remove_worktree_calls()
        assert len(remove_calls) == 1


class TestGetState:
    """Tests for get_state function with caching."""

    def test_get_state_uses_cache(self, dependency_context: DependencyContext, tmp_path: Path):
        """Test get_state returns cached result when available."""
        from datetime import UTC, datetime

        from graft.domain.state import StateResult

        repo_path = tmp_path / "repo"
        repo_path.mkdir()

        # Pre-cache a result
        cached_result = StateResult(
            query_name="test",
            commit_hash="abc123",
            data={"cached": True},
            timestamp=datetime.now(UTC),
            command="echo cached",
            deterministic=True,
            cached=True,
        )
        write_cached_state(dependency_context, "workspace", "repo", cached_result)

        # Create query (won't be executed if cache works)
        query = StateQuery(
            name="test",
            run="exit 1",  # Would fail if executed
            cache=StateCache(deterministic=True),
        )

        # Get state - should use cache
        result = get_state(
            dependency_context,
            query,
            workspace_name="workspace",
            repo_name="repo",
            repo_path=str(repo_path),
            commit_hash="abc123",
            refresh=False,
        )

        assert result.cached is True
        assert result.data == {"cached": True}

    def test_get_state_executes_when_not_cached(
        self, dependency_context: DependencyContext, tmp_path: Path
    ):
        """Test get_state executes query when cache miss."""
        repo_path = tmp_path / "repo"
        repo_path.mkdir()

        workspace_name = "workspace-uncached"
        repo_name = "repo"
        query_name = "uncached-test"
        commit_hash = "def456"

        # Clean up any existing cache before test
        invalidate_cached_state(dependency_context, workspace_name, repo_name, query_name)

        query = StateQuery(
            name=query_name,
            run='echo \'{"executed": true}\'',
            cache=StateCache(deterministic=True),
        )

        result = get_state(
            dependency_context,
            query,
            workspace_name=workspace_name,
            repo_name=repo_name,
            repo_path=str(repo_path),
            commit_hash=commit_hash,
            refresh=False,
        )

        assert result.cached is False
        assert result.data == {"executed": True}

        # Clean up cache after test
        invalidate_cached_state(dependency_context, workspace_name, repo_name, query_name)

    def test_get_state_refresh_ignores_cache(
        self, dependency_context: DependencyContext, tmp_path: Path
    ):
        """Test refresh flag forces execution even with cache."""
        from datetime import UTC, datetime

        from graft.domain.state import StateResult

        repo_path = tmp_path / "repo"
        repo_path.mkdir()

        # Pre-cache old result
        old_result = StateResult(
            query_name="test",
            commit_hash="abc123",
            data={"old": True},
            timestamp=datetime.now(UTC),
            command="echo old",
            deterministic=True,
        )
        write_cached_state(dependency_context, "workspace", "repo", old_result)

        # Create query with new output
        query = StateQuery(
            name="test",
            run='echo \'{"new": true}\'',
            cache=StateCache(deterministic=True),
        )

        # Get state with refresh=True
        result = get_state(
            dependency_context,
            query,
            workspace_name="workspace",
            repo_name="repo",
            repo_path=str(repo_path),
            commit_hash="abc123",
            refresh=True,
        )

        assert result.data == {"new": True}

    def test_get_state_uses_worktree_for_historical(
        self, dependency_context: DependencyContext, tmp_path: Path
    ):
        """Test get_state uses worktree when requested."""
        repo_path = str(tmp_path / "repo")
        dependency_context.git.clone("https://example.com/repo.git", repo_path, "main")

        query = StateQuery(
            name="test",
            run='echo \'{"historical": true}\'',
            cache=StateCache(deterministic=True),
        )

        commit = "abc123def456abc123def456abc123def456abc123de"

        result = get_state(
            dependency_context,
            query,
            workspace_name="workspace",
            repo_name="repo",
            repo_path=repo_path,
            commit_hash=commit,
            refresh=False,
            use_worktree=True,
        )

        # Verify worktree was used
        worktree_calls = dependency_context.git.get_add_worktree_calls()
        assert len(worktree_calls) == 1

        assert result.data == {"historical": True}
