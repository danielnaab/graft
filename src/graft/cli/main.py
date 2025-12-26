"""Main CLI entry point.

Builds production context and registers command groups.
"""

import typer

from graft.adapters.repository import InMemoryRepository
from graft.domain.entities import Entity
from graft.services.context import ServiceContext
from graft.cli.commands import example

app = typer.Typer(
    name="graft-cli",
    help="Knowledge base tooling with language server support",
    add_completion=False,
)

# Register command groups
app.add_typer(example.app, name="example", help="Example commands")


def get_context() -> ServiceContext:
    """Build production service context.

    Creates ServiceContext with real adapters.
    Modify this to use production implementations (e.g., PostgresRepository).

    Returns:
        ServiceContext with production dependencies

    Example:
        For production, replace InMemoryRepository with real implementation:

        return ServiceContext(
            repository=PostgresRepository(
                connection_string=os.getenv("DATABASE_URL")
            )
        )
    """
    return ServiceContext(
        repository=InMemoryRepository[Entity](),
    )


@app.command()
def version() -> None:
    """Show version information."""
    from graft import __version__

    typer.echo(f"Graft v{__version__}")


if __name__ == "__main__":
    app()
