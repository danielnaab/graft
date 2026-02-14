"""State query service.

Service functions for executing and caching state queries.

## Architecture: Layer 1 (State Infrastructure)

This module implements the core state query infrastructure that works with ANY domain.
It provides:
- Query execution (subprocess commands)
- Commit-based caching
- Temporal queries (git worktree)
- Cache invalidation

This layer has NO domain-specific knowledge. It treats all state queries as:
    command → JSON output → cache by commit hash

For integration with domain analytics libraries (Layer 2/3), see:
    /docs/architecture/state-queries-layered-architecture.md
"""

import hashlib
import json
import shutil
import subprocess
import tempfile
from datetime import UTC, datetime
from pathlib import Path

from graft.domain.state import StateQuery, StateResult
from graft.services.dependency_context import DependencyContext


def get_cache_path(
    ctx: DependencyContext,
    workspace_name: str,
    repo_name: str,
    query_name: str,
    commit_hash: str,
) -> Path:
    """Get cache file path for a state query result.

    Args:
        ctx: Dependency context
        workspace_name: Workspace name for grouping
        repo_name: Repository name
        query_name: State query name
        commit_hash: Git commit hash

    Returns:
        Path to cache file

    Example:
        >>> from graft.adapters.filesystem import RealFileSystem
        >>> ctx = DependencyContext(filesystem=RealFileSystem(), deps_directory="..")
        >>> path = get_cache_path(ctx, "my-workspace", "my-repo", "coverage", "abc123")
        >>> str(path)
        '~/.cache/graft/{workspace-hash}/my-repo/state/coverage/abc123.json'
    """
    # Compute workspace hash to avoid path conflicts
    workspace_hash = hashlib.sha256(workspace_name.encode()).hexdigest()[:16]

    # Build cache path
    cache_root = Path.home() / ".cache" / "graft" / workspace_hash / repo_name / "state"
    query_dir = cache_root / query_name
    cache_file = query_dir / f"{commit_hash}.json"

    return cache_file


def read_cached_state(
    ctx: DependencyContext,
    workspace_name: str,
    repo_name: str,
    query_name: str,
    commit_hash: str,
) -> StateResult | None:
    """Read cached state result if it exists.

    Args:
        ctx: Dependency context
        workspace_name: Workspace name
        repo_name: Repository name
        query_name: State query name
        commit_hash: Git commit hash

    Returns:
        Cached StateResult if exists and valid, None otherwise

    Example:
        >>> ctx = DependencyContext(filesystem=RealFileSystem(), deps_directory="..")
        >>> result = read_cached_state(ctx, "workspace", "repo", "coverage", "abc123")
        >>> result is None or isinstance(result, StateResult)
        True
    """
    cache_path = get_cache_path(ctx, workspace_name, repo_name, query_name, commit_hash)

    if not cache_path.exists():
        return None

    try:
        with open(cache_path) as f:
            data = json.load(f)
        return StateResult.from_cache_file(data)
    except (json.JSONDecodeError, KeyError, ValueError) as e:
        # Cache file is corrupted - log warning and delete it
        import sys

        print(
            f"Warning: Corrupted cache for {query_name} at commit {commit_hash[:7]}: {e}",
            file=sys.stderr,
        )

        # Delete corrupted cache file
        try:
            cache_path.unlink()
        except OSError:
            pass

        return None


def write_cached_state(
    ctx: DependencyContext,
    workspace_name: str,
    repo_name: str,
    result: StateResult,
) -> None:
    """Write state result to cache.

    Args:
        ctx: Dependency context
        workspace_name: Workspace name
        repo_name: Repository name
        result: State result to cache

    Example:
        >>> from datetime import datetime
        >>> ctx = DependencyContext(filesystem=RealFileSystem(), deps_directory="..")
        >>> result = StateResult(
        ...     query_name="coverage",
        ...     commit_hash="abc123",
        ...     data={"percent_covered": 85.0},
        ...     timestamp=datetime.now(),
        ...     command="pytest --cov",
        ...     deterministic=True,
        ... )
        >>> write_cached_state(ctx, "workspace", "repo", result)
    """
    cache_path = get_cache_path(
        ctx, workspace_name, repo_name, result.query_name, result.commit_hash
    )

    # Ensure directory exists
    cache_path.parent.mkdir(parents=True, exist_ok=True)

    # Write cache file
    with open(cache_path, "w") as f:
        json.dump(result.to_cache_file(), f, indent=2)


def invalidate_cached_state(
    ctx: DependencyContext,
    workspace_name: str,
    repo_name: str,
    query_name: str | None = None,
) -> int:
    """Invalidate cached state for a query or all queries.

    Args:
        ctx: Dependency context
        workspace_name: Workspace name
        repo_name: Repository name
        query_name: Specific query to invalidate, or None for all

    Returns:
        Number of cache files deleted

    Example:
        >>> ctx = DependencyContext(filesystem=RealFileSystem(), deps_directory="..")
        >>> count = invalidate_cached_state(ctx, "workspace", "repo", "coverage")
        >>> count >= 0
        True
    """
    workspace_hash = hashlib.sha256(workspace_name.encode()).hexdigest()[:16]
    cache_root = Path.home() / ".cache" / "graft" / workspace_hash / repo_name / "state"

    if query_name is None:
        # Delete entire state cache directory
        if cache_root.exists():
            import shutil

            shutil.rmtree(cache_root)
            # Count files (approximate)
            return 1  # We don't track exact count
        return 0
    else:
        # Delete specific query cache
        query_dir = cache_root / query_name
        if query_dir.exists():
            count = sum(1 for _ in query_dir.glob("*.json"))
            import shutil

            shutil.rmtree(query_dir)
            return count
        return 0


def execute_state_query(
    ctx: DependencyContext,
    query: StateQuery,
    repo_path: str,
    commit_hash: str,
) -> StateResult:
    """Execute a state query and return the result.

    Security Model:
    ---------------
    This function uses shell=True for command execution, which allows shell
    features like pipes, redirects, and variable expansion. This is safe because:

    1. Commands come from user's own graft.yaml config file
    2. Users version-control and review their own commands
    3. Similar trust model to Makefile, package.json, .bashrc
    4. No remote or untrusted input is executed

    Users are responsible for the commands they define in their graft.yaml.
    This is equivalent to running arbitrary shell commands from their terminal.

    Args:
        ctx: Dependency context
        query: State query to execute
        repo_path: Path to repository
        commit_hash: Git commit hash

    Returns:
        State result with parsed JSON data

    Raises:
        subprocess.CalledProcessError: If command fails
        json.JSONDecodeError: If output is not valid JSON
        ValueError: If output is not a JSON object

    Example:
        >>> ctx = DependencyContext(filesystem=RealFileSystem(), deps_directory="..")
        >>> from graft.domain.state import StateQuery, StateCache
        >>> query = StateQuery(
        ...     name="test",
        ...     run="echo '{\"result\": \"ok\"}'",
        ...     cache=StateCache(deterministic=True),
        ... )
        >>> result = execute_state_query(ctx, query, "/tmp/repo", "abc123")
        >>> result.data
        {'result': 'ok'}
    """
    # Execute command in repo directory
    # SECURITY: Commands from user's graft.yaml (trusted source).
    # This is the same model as make, npm run, or shell aliases.
    timeout_seconds = query.timeout if query.timeout is not None else 300  # Default 5 minutes

    try:
        proc = subprocess.run(
            query.run,
            shell=True,
            cwd=repo_path,
            capture_output=True,
            text=True,
            timeout=timeout_seconds,
            check=False,  # We'll handle errors manually
        )
    except subprocess.TimeoutExpired as e:
        error_msg = f"State query '{query.name}' timed out after {timeout_seconds}s"
        raise subprocess.CalledProcessError(
            -1, query.run, output="", stderr=error_msg
        ) from e

    # Check exit code
    if proc.returncode != 0:
        error_msg = f"State query '{query.name}' failed with exit code {proc.returncode}"
        if proc.stderr:
            error_msg += f"\nstderr: {proc.stderr}"
        if proc.stdout:
            error_msg += f"\nstdout: {proc.stdout}"
        raise subprocess.CalledProcessError(
            proc.returncode, query.run, output=proc.stdout, stderr=proc.stderr
        )

    # Parse JSON output
    try:
        data = json.loads(proc.stdout)
    except json.JSONDecodeError as e:
        error_msg = f"State query '{query.name}' output is not valid JSON: {e}"
        if proc.stdout:
            error_msg += f"\nOutput was: {proc.stdout[:500]}"
        raise json.JSONDecodeError(
            error_msg, proc.stdout, e.pos
        ) from e

    # Validate it's a JSON object (dict)
    if not isinstance(data, dict):
        raise ValueError(
            f"State query '{query.name}' must output a JSON object, got {type(data).__name__}"
        )

    # Create result
    return StateResult(
        query_name=query.name,
        commit_hash=commit_hash,
        data=data,
        timestamp=datetime.now(UTC),
        command=query.run,
        deterministic=query.cache.deterministic,
        cached=False,
    )


def execute_temporal_query(
    ctx: DependencyContext,
    query: StateQuery,
    repo_path: str,
    commit_hash: str,
) -> StateResult:
    """Execute a state query at a specific historical commit using git worktree.

    Creates a temporary worktree at the target commit, executes the query there,
    and cleans up the worktree afterward. This allows querying historical state
    without affecting the main working directory.

    Args:
        ctx: Dependency context
        query: State query to execute
        repo_path: Path to main repository
        commit_hash: Git commit hash to query

    Returns:
        State result from the historical commit

    Raises:
        subprocess.CalledProcessError: If command fails
        json.JSONDecodeError: If output is not valid JSON
        ValueError: If output structure is invalid or worktree operations fail

    Example:
        >>> ctx = DependencyContext(filesystem=RealFileSystem(), git=SubprocessGitOperations(), deps_directory="..")
        >>> from graft.domain.state import StateQuery, StateCache
        >>> query = StateQuery(
        ...     name="test",
        ...     run="echo '{\\\"result\\\": \\\"ok\\\"}'",
        ...     cache=StateCache(deterministic=True),
        ... )
        >>> result = execute_temporal_query(ctx, query, "/tmp/repo", "abc123...")
        >>> result.data
        {'result': 'ok'}
    """
    # Create temporary worktree directory
    worktree_path = tempfile.mkdtemp(prefix="graft-state-")

    try:
        # Add worktree at specified commit
        ctx.git.add_worktree(repo_path, worktree_path, commit_hash)

        # Execute query in worktree
        result = execute_state_query(ctx, query, worktree_path, commit_hash)

        return result

    finally:
        # Cleanup worktree (remove from git and delete directory)
        try:
            ctx.git.remove_worktree(repo_path, worktree_path)
        except ValueError as e:
            # Worktree removal failed, but continue with directory cleanup
            import sys

            print(
                f"Warning: Failed to remove worktree {worktree_path}: {e}",
                file=sys.stderr,
            )

        # Remove worktree directory
        try:
            shutil.rmtree(worktree_path, ignore_errors=True)
        except OSError as e:
            # Directory cleanup failed, but don't fail the query
            import sys

            print(
                f"Warning: Failed to remove worktree directory {worktree_path}: {e}",
                file=sys.stderr,
            )


def get_state(
    ctx: DependencyContext,
    query: StateQuery,
    workspace_name: str,
    repo_name: str,
    repo_path: str,
    commit_hash: str,
    refresh: bool = False,
    use_worktree: bool = False,
) -> StateResult:
    """Get state query result, using cache if available.

    Args:
        ctx: Dependency context
        query: State query to execute
        workspace_name: Workspace name for cache grouping
        repo_name: Repository name
        repo_path: Path to repository
        commit_hash: Git commit hash
        refresh: If True, invalidate cache and re-run
        use_worktree: If True, use git worktree for temporal execution

    Returns:
        State result (from cache or fresh execution)

    Raises:
        subprocess.CalledProcessError: If command fails
        json.JSONDecodeError: If output is not valid JSON
        ValueError: If output structure is invalid

    Example:
        >>> ctx = DependencyContext(filesystem=RealFileSystem(), deps_directory="..")
        >>> from graft.domain.state import StateQuery, StateCache
        >>> query = StateQuery(
        ...     name="test",
        ...     run="echo '{\"result\": \"ok\"}'",
        ...     cache=StateCache(deterministic=True),
        ... )
        >>> result = get_state(ctx, query, "workspace", "repo", "/tmp/repo", "abc123")
        >>> result.data
        {'result': 'ok'}
    """
    # Check cache (unless refresh requested)
    if not refresh:
        cached = read_cached_state(ctx, workspace_name, repo_name, query.name, commit_hash)
        if cached is not None:
            return cached

    # Execute query (temporal or current)
    if use_worktree:
        result = execute_temporal_query(ctx, query, repo_path, commit_hash)
    else:
        result = execute_state_query(ctx, query, repo_path, commit_hash)

    # Cache result if deterministic
    if result.deterministic:
        write_cached_state(ctx, workspace_name, repo_name, result)

    return result
