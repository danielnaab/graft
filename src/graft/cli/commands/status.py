"""Status command - show dependency status.

CLI command for viewing the current state of dependencies.
"""

import json

import typer

from graft.adapters.lock_file import YamlLockFile
from graft.domain.exceptions import DomainError
from graft.services import query_service


def status_command(
    dep_name: str | None = typer.Argument(None, help="Optional dependency name"),
    format_option: str = typer.Option(
        "text", "--format", help="Output format (text or json)"
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
    lock_file = YamlLockFile()
    lock_path = "graft.lock"

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
