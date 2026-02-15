"""State query commands.

CLI commands for querying and caching repository state.
"""

import json
import subprocess
import sys
from datetime import UTC, datetime
from typing import Annotated

import typer

from graft.adapters.filesystem import RealFileSystem
from graft.adapters.git import SubprocessGitOperations
from graft.services.config_service import find_graft_yaml, parse_graft_yaml
from graft.services.dependency_context import DependencyContext
from graft.services.state_service import (
    execute_temporal_query,
    get_state,
    invalidate_cached_state,
    read_cached_state,
)

state_app = typer.Typer(help="Query and cache repository state")


@state_app.command("query")
def query_state(
    query_name: Annotated[str, typer.Argument(help="Name of state query to execute")],
    commit: Annotated[
        str | None,
        typer.Option("--commit", "-c", help="Git commit to query (default: HEAD)"),
    ] = None,
    refresh: Annotated[
        bool,
        typer.Option("--refresh", "-r", help="Invalidate cache and re-run query"),
    ] = False,
    raw: Annotated[
        bool,
        typer.Option("--raw", help="Output only the data (no metadata)"),
    ] = False,
    pretty: Annotated[
        bool,
        typer.Option("--pretty", "-p", help="Pretty-print JSON output"),
    ] = True,
) -> None:
    """Execute a state query and cache the result.

    Examples:
        graft state query coverage
        graft state query coverage --commit v1.0.0
        graft state query coverage --refresh
        graft state query coverage --raw | jq '.percent_covered'
    """
    # Setup context
    ctx = DependencyContext(
        filesystem=RealFileSystem(),
        git=SubprocessGitOperations(),
        deps_directory=".graft",
    )

    try:
        # Find and parse graft.yaml
        config_path = find_graft_yaml(ctx)
        config = parse_graft_yaml(ctx, config_path)

        # Check if state query exists
        if not config.has_state_query(query_name):
            typer.echo(f"Error: State query '{query_name}' not found")

            if config.state:
                typer.echo("\nAvailable state queries:")
                for name in config.state.keys():
                    typer.echo(f"  - {name}")
            else:
                typer.echo(
                    "\nNo state queries defined in graft.yaml"
                )
                typer.echo(
                    "Add a 'state:' section with query definitions."
                )

            raise typer.Exit(code=1)

        query = config.get_state_query(query_name)

        # Get current commit hash (or specified commit)
        target_commit = commit or "HEAD"

        try:
            commit_hash = ctx.git.resolve_ref(".", target_commit)
        except subprocess.CalledProcessError as e:
            typer.echo(
                f"Error: Failed to resolve commit '{target_commit}': {e}"
            )
            raise typer.Exit(code=1)

        # Check if working tree is clean for historical queries
        if target_commit != "HEAD" and not ctx.git.is_working_directory_clean("."):
            typer.echo("Error: Working directory has uncommitted changes")
            typer.echo("Commit or stash changes before querying historical state")
            typer.echo(f"  Or use: graft state query {query_name} --commit HEAD")
            raise typer.Exit(code=1)

        # Get repository name from metadata or use directory name
        repo_name = config.metadata.get("name", ctx.filesystem.get_cwd().split("/")[-1])

        # Get workspace name (use repo name for now, later can be from workspace.yaml)
        workspace_name = repo_name

        # Determine if this is a historical query (requires worktree)
        try:
            current_commit = ctx.git.get_current_commit(".")
            use_worktree = commit_hash != current_commit
        except ValueError:
            # Can't determine current commit (detached HEAD, not in repo, etc.)
            # Assume historical query for safety - will use worktree
            use_worktree = True

        # Execute state query (with caching)
        try:
            result = get_state(
                ctx=ctx,
                query=query,
                workspace_name=workspace_name,
                repo_name=repo_name,
                repo_path=".",
                commit_hash=commit_hash,
                refresh=refresh,
                use_worktree=use_worktree,
            )
        except subprocess.CalledProcessError as e:
            typer.echo(
                f"Error: State query '{query_name}' failed"
            )
            typer.echo(f"Exit code: {e.returncode}")
            if e.stderr:
                typer.echo(f"stderr:\n{e.stderr}")
            if e.stdout:
                typer.echo(f"stdout:\n{e.stdout}")
            raise typer.Exit(code=1)
        except json.JSONDecodeError as e:
            typer.echo(
                f"Error: State query '{query_name}' output is not valid JSON"
            )
            typer.echo(f"{e}")
            raise typer.Exit(code=1)
        except ValueError as e:
            typer.echo(
                f"Error: State query '{query_name}' output is invalid"
            )
            typer.echo(f"{e}")
            raise typer.Exit(code=1)

        # Output result
        if raw:
            # Output only the data
            if pretty:
                print(json.dumps(result.data, indent=2))
            else:
                print(json.dumps(result.data))
        else:
            # Output full result with metadata
            full_output = result.to_cache_file()
            if pretty:
                print(json.dumps(full_output, indent=2))
            else:
                print(json.dumps(full_output))

        # Show cache status to stderr
        if result.cached:
            typer.echo(
                "(from cache)",
                file=sys.stderr,
            )

    except Exception as e:
        typer.echo(f"Error: {e}")
        raise typer.Exit(code=1)


@state_app.command("list")
def list_state_queries(
    show_cache: Annotated[
        bool,
        typer.Option("--cache", "-c", help="Show cache status for current commit"),
    ] = True,
) -> None:
    """List all defined state queries.

    Examples:
        graft state list
        graft state list --cache
    """
    # Setup context
    ctx = DependencyContext(
        filesystem=RealFileSystem(),
        git=SubprocessGitOperations(),
        deps_directory=".graft",
    )

    try:
        # Find and parse graft.yaml
        config_path = find_graft_yaml(ctx)
        config = parse_graft_yaml(ctx, config_path)

        if not config.state:
            typer.echo("No state queries defined in graft.yaml")
            typer.echo("\nAdd a 'state:' section like:")
            typer.echo("```yaml")
            typer.echo("state:")
            typer.echo("  coverage:")
            typer.echo("    run: 'pytest --cov --cov-report=json | jq .totals.percent_covered'")
            typer.echo("    cache:")
            typer.echo("      deterministic: true")
            typer.echo("```")
            return

        typer.echo("State queries defined in graft.yaml:\n")

        # Get current commit for cache checking
        commit_hash = None
        if show_cache:
            try:
                commit_hash = ctx.git.resolve_ref(".", "HEAD")
            except subprocess.CalledProcessError:
                pass

        repo_name = config.metadata.get("name", ctx.filesystem.get_cwd().split("/")[-1])
        workspace_name = repo_name

        for name, query in config.state.items():
            typer.echo(f"{name}")
            typer.echo(f"  Command: {query.run}")

            if show_cache and commit_hash:
                cached = read_cached_state(
                    ctx, workspace_name, repo_name, name, commit_hash
                )
                if cached:
                    typer.echo(
                        f"  Cached:  Yes "
                        f"(commit {commit_hash[:7]}, "
                        f"{_format_time_ago(cached.timestamp)})"
                    )
                else:
                    typer.echo("  Cached:  No")

            typer.echo()

    except Exception as e:
        typer.echo(f"Error: {e}")
        raise typer.Exit(code=1)


@state_app.command("invalidate")
def invalidate_cache(
    query_name: Annotated[
        str | None,
        typer.Argument(help="Name of state query to invalidate (or all if omitted)"),
    ] = None,
    all_queries: Annotated[
        bool,
        typer.Option("--all", "-a", help="Invalidate all state caches"),
    ] = False,
) -> None:
    """Invalidate cached state.

    Examples:
        graft state invalidate coverage
        graft state invalidate --all
    """
    # Setup context
    ctx = DependencyContext(
        filesystem=RealFileSystem(),
        git=SubprocessGitOperations(),
        deps_directory=".graft",
    )

    try:
        # Find and parse graft.yaml
        config_path = find_graft_yaml(ctx)
        config = parse_graft_yaml(ctx, config_path)

        repo_name = config.metadata.get("name", ctx.filesystem.get_cwd().split("/")[-1])
        workspace_name = repo_name

        if all_queries:
            count = invalidate_cached_state(ctx, workspace_name, repo_name, None)
            typer.echo(f"Invalidated all state caches ({count} cache entries)")
        elif query_name:
            if not config.has_state_query(query_name):
                typer.echo(
                    f"Error: State query '{query_name}' not found"
                )
                raise typer.Exit(code=1)

            count = invalidate_cached_state(ctx, workspace_name, repo_name, query_name)
            typer.echo(
                f"Invalidated cache for '{query_name}' ({count} cache entries)"
            )
        else:
            typer.echo(
                "Error: Specify a query name or use --all"
            )
            raise typer.Exit(code=1)

    except Exception as e:
        typer.echo(f"Error: {e}")
        raise typer.Exit(code=1)


def _format_time_ago(timestamp: datetime) -> str:
    """Format timestamp as relative time.

    Args:
        timestamp: Timestamp to format

    Returns:
        Human-readable relative time (e.g., "5 minutes ago")
    """

    now = datetime.now(UTC)
    # Make timestamp timezone-aware if it isn't
    if timestamp.tzinfo is None:
        timestamp = timestamp.replace(tzinfo=UTC)

    delta = now - timestamp
    seconds = delta.total_seconds()

    if seconds < 60:
        return "just now"
    elif seconds < 3600:
        minutes = int(seconds / 60)
        return f"{minutes} minute{'s' if minutes != 1 else ''} ago"
    elif seconds < 86400:
        hours = int(seconds / 3600)
        return f"{hours} hour{'s' if hours != 1 else ''} ago"
    else:
        days = int(seconds / 86400)
        return f"{days} day{'s' if days != 1 else ''} ago"
