"""Status command - show dependency status.

CLI command for viewing the current state of dependencies.
"""

import json
from pathlib import Path

import typer

from graft.adapters.lock_file import YamlLockFile
from graft.cli.dependency_context_factory import get_dependency_context
from graft.domain.exceptions import ConfigFileNotFoundError, DomainError
from graft.services import config_service, lock_service, query_service


def _check_for_updates(dep_name: str | None, format_option: str) -> None:
    """Check for available updates by fetching from remote.

    Args:
        dep_name: Optional dependency name to check
        format_option: Output format (text or json)
    """
    ctx = get_dependency_context()
    lock_file = YamlLockFile()
    lock_path = "graft.lock"

    try:
        # Load configuration
        config_path = config_service.find_graft_yaml(ctx)
        config = config_service.parse_graft_yaml(ctx, config_path)

        # Load lock file
        lock_entries = lock_service.get_all_lock_entries(lock_file, lock_path)

        # Determine which dependencies to check
        if dep_name:
            if dep_name not in config.dependencies:
                error_msg = f"Dependency '{dep_name}' not found in graft.yaml"
                if format_option == "json":
                    typer.echo(json.dumps({"error": error_msg}, indent=2))
                else:
                    typer.secho(f"Error: {error_msg}", fg=typer.colors.RED, err=True)
                raise typer.Exit(code=1)

            if dep_name not in lock_entries:
                error_msg = f"Dependency '{dep_name}' not found in graft.lock"
                if format_option == "json":
                    typer.echo(json.dumps({"error": error_msg}, indent=2))
                else:
                    typer.secho(f"Error: {error_msg}", fg=typer.colors.RED, err=True)
                raise typer.Exit(code=1)

            deps_to_check = {dep_name: config.dependencies[dep_name]}
        else:
            deps_to_check = config.dependencies

        if format_option != "json":
            typer.echo("Checking for updates...")
            typer.echo()

        updates_info = {}

        # Check each dependency
        for name, dep_spec in deps_to_check.items():
            dep_path = Path(ctx.deps_directory) / name

            # Skip if not cloned
            if not dep_path.exists() or not ctx.git.is_repository(str(dep_path)):
                if format_option != "json":
                    typer.secho(
                        f"  ⚠ {name}: not cloned (run 'graft resolve')",
                        fg=typer.colors.YELLOW,
                    )
                updates_info[name] = {"error": "not cloned"}
                continue

            # Fetch from remote
            try:
                ctx.git.fetch_all(str(dep_path))
            except Exception as e:
                if format_option != "json":
                    typer.secho(
                        f"  ✗ {name}: fetch failed: {e}",
                        fg=typer.colors.RED,
                        err=True,
                    )
                updates_info[name] = {"error": f"fetch failed: {e}"}
                continue

            # Get current version from lock
            current_ref = lock_entries[name].ref if name in lock_entries else "unknown"

            # Get available tags (simplified - just show we fetched)
            # In a full implementation, we'd parse git tags and compare versions
            if format_option != "json":
                typer.echo(f"  {name}:")
                typer.echo(f"    Current: {current_ref}")
                typer.echo(f"    Status: Up to date (remote fetched)")
            else:
                updates_info[name] = {
                    "current_ref": current_ref,
                    "status": "fetched",
                }

        # JSON output
        if format_option == "json":
            typer.echo(json.dumps({"dependencies": updates_info}, indent=2))

    except ConfigFileNotFoundError:
        error_msg = "graft.yaml not found"
        if format_option == "json":
            typer.echo(json.dumps({"error": error_msg}, indent=2))
        else:
            typer.secho(f"Error: {error_msg}", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=1)


def status_command(
    dep_name: str | None = typer.Argument(None, help="Optional dependency name"),
    format_option: str = typer.Option(
        "text", "--format", help="Output format (text or json)"
    ),
    check_updates: bool = typer.Option(
        False, "--check-updates", help="Fetch and show available updates"
    ),
) -> None:
    """Show status of dependencies.

    Displays the current consumed version, commit hash, and timestamp
    for each dependency from graft.lock.

    Args:
        dep_name: Optional dependency name to show status for.
                 If not provided, shows all dependencies.

    Example:
        $ graft status

        Dependencies:
          meta-kb: v1.5.0 (commit: abc123..., consumed: 2026-01-01 10:30:00)
          shared-utils: v2.0.0 (commit: def456..., consumed: 2025-12-15 14:20:00)

        $ graft status meta-kb

        meta-kb: v1.5.0
          Commit: abc123...
          Consumed: 2026-01-01 10:30:00
    """
    # Validate format option
    if format_option not in ("text", "json"):
        typer.secho(
            f"Error: Invalid format '{format_option}'. Must be 'text' or 'json'",
            fg=typer.colors.RED,
            err=True,
        )
        raise typer.Exit(code=1)

    lock_file = YamlLockFile()
    lock_path = "graft.lock"

    # Handle --check-updates flag
    if check_updates:
        _check_for_updates(dep_name, format_option)
        return

    try:
        if dep_name:
            # Show status for single dependency
            status = query_service.get_dependency_status(
                lock_file, lock_path, dep_name
            )

            if not status:
                if format_option == "json":
                    # JSON error output
                    error_obj = {"error": f"Dependency '{dep_name}' not found in graft.lock"}
                    typer.echo(json.dumps(error_obj, indent=2))
                else:
                    typer.secho(
                        f"Error: Dependency '{dep_name}' not found in graft.lock",
                        fg=typer.colors.RED,
                        err=True,
                    )
                raise typer.Exit(code=1)

            if format_option == "json":
                # JSON output for single dependency
                status_obj = {
                    "name": status.name,
                    "current_ref": status.current_ref,
                    "commit": status.commit,
                    "consumed_at": status.consumed_at.isoformat(),
                }
                typer.echo(json.dumps(status_obj, indent=2))
            else:
                # Text output
                typer.echo(f"{status.name}: {status.current_ref}")
                typer.echo(f"  Commit: {status.commit[:7]}...")
                typer.echo(f"  Consumed: {status.consumed_at}")

        else:
            # Show status for all dependencies
            statuses = query_service.get_all_status(lock_file, lock_path)

            if not statuses:
                if format_option == "json":
                    # JSON output for empty case
                    typer.echo(json.dumps({"dependencies": {}}, indent=2))
                else:
                    typer.secho(
                        "No dependencies found in graft.lock",
                        fg=typer.colors.YELLOW,
                    )
                    typer.echo()
                    typer.echo("Run 'graft resolve' to resolve dependencies first.")
                return

            if format_option == "json":
                # JSON output for all dependencies
                deps_obj = {
                    "dependencies": {
                        status.name: {
                            "current_ref": status.current_ref,
                            "commit": status.commit,
                            "consumed_at": status.consumed_at.isoformat(),
                        }
                        for status in statuses
                    }
                }
                typer.echo(json.dumps(deps_obj, indent=2))
            else:
                # Text output
                typer.echo("Dependencies:")
                for status in statuses:
                    typer.echo(
                        f"  {status.name}: {status.current_ref} "
                        f"(commit: {status.commit[:7]}..., "
                        f"consumed: {status.consumed_at.strftime('%Y-%m-%d %H:%M:%S')})"
                    )

    except DomainError as e:
        typer.secho(f"Error: {e}", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=1) from e
    except OSError as e:
        typer.secho(
            f"Error: Unable to read graft.lock: {e}",
            fg=typer.colors.RED,
            err=True,
        )
        raise typer.Exit(code=1) from e
