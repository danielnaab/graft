"""Show command - show details of a specific change.

CLI command for viewing detailed information about a specific change/version.
"""

from pathlib import Path

import typer

from graft.cli.dependency_context_factory import get_dependency_context
from graft.domain.exceptions import (
    ConfigFileNotFoundError,
    ConfigParseError,
    ConfigValidationError,
    DomainError,
)
from graft.services import config_service, query_service


def show_command(dep_ref: str) -> None:
    """Show details of a specific change.

    Displays full details including description, migration command,
    and verification command for a specific version/ref.

    Args:
        dep_ref: Dependency and ref in format "dep-name@ref" (e.g., "meta-kb@v2.0.0")

    Example:
        $ graft show meta-kb@v2.0.0

        Change: meta-kb@v2.0.0

        Type: breaking
        Description: Renamed getUserData → fetchUserData

        Migration: migrate-v2
          Command: npx jscodeshift -t codemods/v2.js src/
          Description: Rename getUserData to fetchUserData

        Verification: verify-v2
          Command: npm test && ! grep -r 'getUserData' src/
          Description: Verify v2 migration: tests pass and no old API usage
    """
    # Parse dep_name@ref format
    if "@" not in dep_ref:
        typer.secho(
            "Error: Invalid format. Use 'dep-name@ref' (e.g., 'meta-kb@v2.0.0')",
            fg=typer.colors.RED,
            err=True,
        )
        raise typer.Exit(code=1)

    dep_name, ref = dep_ref.split("@", 1)
    ctx = get_dependency_context()

    try:
        # Find dependency's graft.yaml
        dep_path = Path(ctx.deps_directory) / dep_name / "graft.yaml"
        dep_config_path = str(dep_path)

        # Parse dependency's graft.yaml
        config = config_service.parse_graft_yaml(ctx, dep_config_path)

        # Get change details
        details = query_service.get_change_details(config, ref)

        if not details:
            typer.secho(
                f"Error: Change {ref} not found for {dep_name}",
                fg=typer.colors.RED,
                err=True,
            )
            typer.echo(
                f"  Run 'graft changes {dep_name}' to see available changes",
                err=True,
            )
            raise typer.Exit(code=1)

        # Display header
        typer.secho(f"Change: {dep_name}@{ref}", fg=typer.colors.BLUE, bold=True)
        typer.echo()

        # Display type
        if details.change.type:
            type_color = (
                typer.colors.RED
                if details.change.is_breaking()
                else typer.colors.GREEN
            )
            typer.secho(f"Type: {details.change.type}", fg=type_color)

        # Display description
        if details.change.description:
            typer.echo(f"Description: {details.change.description}")
            typer.echo()

        # Display migration details
        if details.migration_command:
            cmd = details.migration_command
            typer.secho(f"Migration: {cmd.name}", fg=typer.colors.YELLOW)
            typer.echo(f"  Command: {cmd.run}")
            if cmd.description:
                typer.echo(f"  Description: {cmd.description}")
            if cmd.working_dir:
                typer.echo(f"  Working directory: {cmd.working_dir}")
            typer.echo()

        # Display verification details
        if details.verify_command:
            cmd = details.verify_command
            typer.secho(f"Verification: {cmd.name}", fg=typer.colors.YELLOW)
            typer.echo(f"  Command: {cmd.run}")
            if cmd.description:
                typer.echo(f"  Description: {cmd.description}")
            if cmd.working_dir:
                typer.echo(f"  Working directory: {cmd.working_dir}")
            typer.echo()

        # Show if no migration/verification required
        if not details.migration_command and not details.verify_command:
            typer.secho("No migration or verification required", fg=typer.colors.GREEN)
            typer.echo()

    except ConfigFileNotFoundError as e:
        typer.secho(
            f"Error: Dependency configuration not found",
            fg=typer.colors.RED,
            err=True,
        )
        typer.echo(f"  Path: {e.path}", err=True)
        typer.secho(
            f"  Suggestion: Check that {dep_name} is resolved in {ctx.deps_directory}",
            fg=typer.colors.YELLOW,
            err=True,
        )
        raise typer.Exit(code=1) from e

    except ConfigParseError as e:
        typer.secho(
            "Error: Failed to parse dependency configuration",
            fg=typer.colors.RED,
            err=True,
        )
        typer.echo(f"  File: {e.path}", err=True)
        typer.echo(f"  Reason: {e.reason}", err=True)
        raise typer.Exit(code=1) from e

    except ConfigValidationError as e:
        typer.secho(
            "Error: Invalid dependency configuration",
            fg=typer.colors.RED,
            err=True,
        )
        typer.echo(f"  File: {e.path}", err=True)
        typer.echo(f"  Field: {e.field}", err=True)
        typer.echo(f"  Reason: {e.reason}", err=True)
        raise typer.Exit(code=1) from e

    except DomainError as e:
        typer.secho(f"Error: {e}", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=1) from e
