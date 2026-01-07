"""Fetch command - update local cache of dependencies.

CLI command for fetching latest from remote repositories.
"""

from pathlib import Path

import typer

from graft.cli.dependency_context_factory import get_dependency_context
from graft.domain.exceptions import ConfigFileNotFoundError
from graft.services import config_service


def fetch_command(
    dep_name: str | None = typer.Argument(
        None,
        help="Dependency name (fetch all if not specified)",
    ),
) -> None:
    """Update local cache of dependency's remote state.

    Fetches latest from remote repository without modifying lock file.
    Use 'graft changes' to see what's available after fetching.

    Example:
        $ graft fetch my-knowledge
        Fetching my-knowledge...
          ✓ Fetched successfully

        $ graft fetch
        Fetching all dependencies...
        my-knowledge:
          ✓ Fetched successfully
        other-dep:
          ✓ Fetched successfully
    """
    ctx = get_dependency_context()

    try:
        # Find and parse configuration
        config_path = config_service.find_graft_yaml(ctx)
        config = config_service.parse_graft_yaml(ctx, config_path)

        # Determine which dependencies to fetch
        if dep_name:
            # Fetch specific dependency
            if dep_name not in config.dependencies:
                typer.secho(
                    f"Error: Dependency '{dep_name}' not found in graft.yaml",
                    fg=typer.colors.RED,
                    err=True,
                )
                raise typer.Exit(code=1)

            deps_to_fetch = {dep_name: config.dependencies[dep_name]}
            typer.echo(f"Fetching {dep_name}...")
        else:
            # Fetch all dependencies
            deps_to_fetch = config.dependencies
            typer.echo("Fetching all dependencies...")

        # Fetch each dependency
        success_count = 0
        error_count = 0

        for name, _dep_spec in deps_to_fetch.items():
            dep_path = Path(ctx.deps_directory) / name

            # Check if dependency is cloned
            if not dep_path.exists():
                typer.secho(
                    f"  ⚠ {name}: not cloned (run 'graft resolve')",
                    fg=typer.colors.YELLOW,
                )
                error_count += 1
                continue

            if not ctx.git.is_repository(str(dep_path)):
                typer.secho(
                    f"  ✗ {name}: not a git repository",
                    fg=typer.colors.RED,
                    err=True,
                )
                error_count += 1
                continue

            # Fetch from remote (all refs)
            try:
                ctx.git.fetch_all(str(dep_path))
                typer.secho(f"  ✓ {name}: fetched successfully", fg=typer.colors.GREEN)
                success_count += 1
            except Exception as e:
                typer.secho(
                    f"  ✗ {name}: fetch failed: {e}",
                    fg=typer.colors.RED,
                    err=True,
                )
                error_count += 1

        # Summary
        typer.echo()
        if error_count == 0:
            typer.secho(
                f"✓ Successfully fetched {success_count} {'dependency' if success_count == 1 else 'dependencies'}",
                fg=typer.colors.GREEN,
                bold=True,
            )
        else:
            typer.secho(
                f"Fetched {success_count}, {error_count} {'error' if error_count == 1 else 'errors'}",
                fg=typer.colors.YELLOW,
            )
            if error_count > 0 and success_count == 0:
                raise typer.Exit(code=1)

    except ConfigFileNotFoundError:
        typer.secho(
            "Error: graft.yaml not found in current directory",
            fg=typer.colors.RED,
            err=True,
        )
        raise typer.Exit(code=1) from None
