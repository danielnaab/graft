"""Main CLI entry point.

Registers command groups and top-level commands.
"""

import typer

from graft.cli.commands import apply, changes, example, resolve, show, status, upgrade

app = typer.Typer(
    name="graft-cli",
    help="Knowledge base tooling with language server support",
    add_completion=False,
)

# Register command groups
app.add_typer(example.app, name="example", help="Example commands")

# Register commands
app.command(name="resolve", help="Resolve dependencies from graft.yaml")(
    resolve.resolve_command
)

app.command(name="status", help="Show status of dependencies")(
    status.status_command
)

app.command(name="changes", help="List changes for a dependency")(
    changes.changes_command
)

app.command(name="show", help="Show details of a specific change")(
    show.show_command
)

app.command(name="upgrade", help="Upgrade dependency to new version")(
    upgrade.upgrade_command
)

app.command(name="apply", help="Update lock file without running migrations")(
    apply.apply_command
)


@app.command()
def version() -> None:
    """Show version information."""
    from graft import __version__

    typer.echo(f"Graft v{__version__}")


if __name__ == "__main__":
    app()
