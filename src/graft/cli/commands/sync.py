"""Sync command - synchronize dependencies to lock file state.

CLI command for syncing .graft/ to match graft.lock.
"""

from pathlib import Path

import typer

from graft.adapters.lock_file import YamlLockFile
from graft.cli.dependency_context_factory import get_dependency_context
from graft.services import lock_service, sync_service


def sync_command() -> None:
    """Sync dependencies to match lock file state.

    Reads graft.lock and ensures .graft/ matches:
    - Clones missing dependencies
    - Checks out correct commits for existing ones

    Example:
        $ graft sync

        Syncing dependencies to lock file...

        ✓ graft-knowledge: Already at abc123
        ✓ python-starter: Checked out def456
        ✓ meta-knowledge-base: Cloned and checked out 789abc

        Synced: 3/3 dependencies
    """
    ctx = get_dependency_context()
    lock_file = YamlLockFile()
    lock_path = "graft.lock"

    # Check lock file exists
    if not Path(lock_path).exists():
        typer.secho(
            "Error: graft.lock not found",
            fg=typer.colors.RED,
            err=True,
        )
        typer.echo("Run 'graft resolve' to create the lock file.", err=True)
        raise typer.Exit(code=1)

    # Read lock file
    try:
        lock_entries = lock_service.get_all_lock_entries(lock_file, lock_path)
    except Exception as e:
        typer.secho(
            f"Error: Failed to read lock file: {e}",
            fg=typer.colors.RED,
            err=True,
        )
        raise typer.Exit(code=1) from e

    if not lock_entries:
        typer.echo("No dependencies in lock file.")
        return

    typer.echo("Syncing dependencies to lock file...")
    typer.echo()

    # Sync all dependencies
    results = sync_service.sync_all_dependencies(
        filesystem=ctx.filesystem,
        git=ctx.git,
        deps_directory=ctx.deps_directory,
        lock_entries=lock_entries,
    )

    # Display results
    success_count = 0
    for result in results:
        if result.success:
            success_count += 1
            if result.action == "up_to_date":
                typer.secho(
                    f"  ✓ {result.name}: {result.message}",
                    fg=typer.colors.BRIGHT_BLACK,
                )
            else:
                typer.secho(
                    f"  ✓ {result.name}: {result.message}",
                    fg=typer.colors.GREEN,
                )
        else:
            typer.secho(
                f"  ✗ {result.name}: {result.message}",
                fg=typer.colors.RED,
            )

    # Summary
    typer.echo()
    total = len(results)
    if success_count == total:
        typer.secho(
            f"Synced: {success_count}/{total} dependencies",
            fg=typer.colors.GREEN,
        )
    else:
        typer.secho(
            f"Synced: {success_count}/{total} dependencies ({total - success_count} failed)",
            fg=typer.colors.YELLOW,
        )
        raise typer.Exit(code=1)
