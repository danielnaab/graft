"""Tree command - dependency listing.

CLI command for listing dependencies.

Note: With the flat-only dependency model (Decision 0007), this command
simply lists all dependencies. There is no hierarchy since transitive
resolution was removed.
"""

import typer

from graft.adapters.lock_file import YamlLockFile
from graft.services import lock_service


def tree_command(
    show_details: bool = typer.Option(
        False,
        "--details",
        "-d",
        help="Show detailed information for each dependency",
    ),
) -> None:
    """List dependencies from lock file.

    With the flat-only model, all dependencies are direct (declared in
    graft.yaml). This command lists them with optional details.

    Example:
        $ graft tree

        Dependencies:
          meta-knowledge-base (main)
          python-starter (main)

        Total: 2 dependencies

        $ graft tree --details

        Dependencies:

          meta-knowledge-base (main)
            source: https://github.com/user/meta-kb.git
            commit: abc123d

          python-starter (v1.0.0)
            source: https://github.com/user/python-starter.git
            commit: def456a

        Total: 2 dependencies
    """
    # Find and read lock file
    try:
        lock_file = YamlLockFile()
        lock_file_path = lock_service.find_lock_file(lock_file, ".")
        if not lock_file_path:
            raise FileNotFoundError("graft.lock not found")
        entries = lock_file.read_lock_file(lock_file_path)

        if not entries:
            typer.echo("No dependencies found in lock file.")
            typer.echo()
            typer.secho(
                "Run 'graft resolve' to resolve dependencies.",
                fg=typer.colors.YELLOW,
            )
            return

    except FileNotFoundError:
        typer.secho("Error: Lock file not found", fg=typer.colors.RED, err=True)
        typer.secho(
            "  Suggestion: Run 'graft resolve' first", fg=typer.colors.YELLOW, err=True
        )
        raise typer.Exit(code=1)

    except Exception as e:
        typer.secho(
            f"Error: Failed to read lock file: {e}", fg=typer.colors.RED, err=True
        )
        raise typer.Exit(code=1) from e

    # Display dependencies
    typer.echo("Dependencies:")
    typer.echo()

    for name in sorted(entries.keys()):
        entry = entries[name]

        if show_details:
            typer.secho(f"  {name} ({entry.ref})", fg=typer.colors.GREEN)
            typer.echo(f"    source: {entry.source}")
            typer.echo(f"    commit: {entry.commit[:7]}")
            typer.echo()
        else:
            typer.secho(f"  {name} ({entry.ref})", fg=typer.colors.GREEN)

    # Summary
    if not show_details:
        typer.echo()
    typer.echo(f"Total: {len(entries)} dependencies")
