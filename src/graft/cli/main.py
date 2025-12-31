"""Main CLI entry point.

Registers command groups and top-level commands.
"""

import typer

from graft.cli.commands import example, resolve

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


@app.command()
def version() -> None:
    """Show version information."""
    from graft import __version__

    typer.echo(f"Graft v{__version__}")


if __name__ == "__main__":
    app()
