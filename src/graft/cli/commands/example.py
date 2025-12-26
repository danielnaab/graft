"""Example CLI commands demonstrating CLI patterns.

Each command wires the service context and calls service functions.
Commands handle I/O and presentation - service functions handle logic.
"""

import typer

from graft.cli.main import get_context
from graft.domain.exceptions import EntityNotFoundError, ValidationError
from graft.services import example_service

app = typer.Typer()


@app.command()
def create(
    name: str = typer.Argument(..., help="Entity name"),
    value: int = typer.Argument(..., help="Entity value (non-negative)"),
) -> None:
    """Create a new example entity.

    Example:
        graft-cli example create "My Entity" 100
    """
    ctx = get_context()

    try:
        entity = example_service.create_example(ctx, name=name, value=value)
        typer.echo(f"Created entity: {entity.id}")
        typer.echo(f"  Name: {entity.name.text}")
        typer.echo(f"  Value: {entity.value.amount}")
    except ValidationError as e:
        typer.echo(f"Validation error: {e}", err=True)
        raise typer.Exit(code=1)


@app.command()
def get(
    entity_id: str = typer.Argument(..., help="Entity ID"),
) -> None:
    """Get an example entity by ID.

    Example:
        graft-cli example get <entity-id>
    """
    ctx = get_context()

    try:
        entity = example_service.get_example(ctx, entity_id=entity_id)
        typer.echo(f"Entity: {entity.id}")
        typer.echo(f"  Name: {entity.name.text}")
        typer.echo(f"  Value: {entity.value.amount}")
    except EntityNotFoundError as e:
        typer.echo(f"Error: {e}", err=True)
        raise typer.Exit(code=1)


@app.command(name="list")
def list_entities() -> None:
    """List all example entities.

    Example:
        graft-cli example list
    """
    ctx = get_context()

    entities = example_service.list_examples(ctx)

    if not entities:
        typer.echo("No entities found.")
        return

    typer.echo(f"Found {len(entities)} entities:")
    for entity in entities:
        typer.echo(f"  - {entity.id}: {entity.name.text} (value: {entity.value.amount})")
