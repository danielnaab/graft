"""Changes command - list changes for a dependency.

CLI command for viewing available changes/updates for a dependency.
"""

import json
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


def changes_command(
    dep_name: str,
    from_ref: str | None = None,
    to_ref: str | None = None,
    change_type: str | None = typer.Option(
        None, "--type", help="Filter by type (breaking, feature, fix, etc.)"
    ),
    breaking_only: bool = typer.Option(
        False, "--breaking", help="Show only breaking changes"
    ),
    format_option: str = typer.Option(
        "text", "--format", help="Output format (text or json)"
    ),
) -> None:
    """List changes for a dependency.

    Reads the dependency's graft.yaml and shows available changes/updates.
    By default shows all changes. Use --from and --to to filter by ref range.

    Args:
        dep_name: Name of dependency
        from_ref: Starting ref (optional)
        to_ref: Ending ref (optional)
        change_type: Filter by change type
        breaking_only: Show only breaking changes

    Example:
        $ graft changes meta-kb

        Changes for meta-kb:

        v1.6.0 (feature)
          Added response caching
          No migration required

        v2.0.0 (breaking)
          Renamed getUserData → fetchUserData
          Migration: migrate-v2
          Verify: verify-v2

        $ graft changes meta-kb --breaking

        Breaking changes for meta-kb:

        v2.0.0 (breaking)
          Renamed getUserData → fetchUserData
          Migration: migrate-v2
          Verify: verify-v2
    """
    ctx = get_dependency_context()

    try:
        # Find dependency's graft.yaml
        dep_path = Path(ctx.deps_directory) / dep_name / "graft.yaml"
        dep_config_path = str(dep_path)

        # Parse dependency's graft.yaml
        config = config_service.parse_graft_yaml(ctx, dep_config_path)

        # Get changes (optionally filtered by range)
        changes = query_service.get_changes_in_range(config, from_ref, to_ref)

        # Apply filters
        if breaking_only:
            changes = query_service.filter_breaking_changes(changes)
        elif change_type:
            changes = query_service.filter_changes_by_type(changes, change_type)

        # Display results
        if not changes:
            if format_option == "json":
                # JSON output for empty case
                filter_desc = ""
                if breaking_only:
                    filter_desc = "breaking "
                elif change_type:
                    filter_desc = f"{change_type} "

                output = {
                    "dependency": dep_name,
                    "from": from_ref,
                    "to": to_ref,
                    "changes": [],
                    "message": f"No {filter_desc}changes found"
                }
                typer.echo(json.dumps(output, indent=2))
            else:
                # Text output
                filter_desc = ""
                if breaking_only:
                    filter_desc = "breaking "
                elif change_type:
                    filter_desc = f"{change_type} "

                typer.secho(
                    f"No {filter_desc}changes found for {dep_name}",
                    fg=typer.colors.YELLOW,
                )
            return

        if format_option == "json":
            # JSON output for changes
            changes_list = []
            for change in changes:
                change_obj = {
                    "ref": change.ref,
                    "type": change.type,
                    "description": change.description,
                    "migration": change.migration,
                    "verify": change.verify,
                }
                changes_list.append(change_obj)

            output = {
                "dependency": dep_name,
                "from": from_ref,
                "to": to_ref,
                "changes": changes_list
            }
            typer.echo(json.dumps(output, indent=2))
        else:
            # Text output
            # Header
            header = f"Changes for {dep_name}:"
            if from_ref or to_ref:
                range_str = f"{from_ref or '(start)'} → {to_ref or '(latest)'}"
                header = f"Changes for {dep_name}: {range_str}"
            elif breaking_only:
                header = f"Breaking changes for {dep_name}:"
            elif change_type:
                header = f"{change_type.capitalize()} changes for {dep_name}:"

            typer.secho(header, fg=typer.colors.BLUE)
            typer.echo()

            # Display each change
            for change in changes:
                # Ref and type
                type_str = f"({change.type})" if change.type else ""
                type_color = typer.colors.RED if change.is_breaking() else typer.colors.GREEN
                typer.secho(f"{change.ref} {type_str}", fg=type_color)

                # Description
                if change.description:
                    typer.echo(f"  {change.description}")

                # Migration/verification info
                if change.migration or change.verify:
                    if change.migration:
                        typer.echo(f"  Migration: {change.migration}")
                    if change.verify:
                        typer.echo(f"  Verify: {change.verify}")
                else:
                    typer.echo("  No migration required")

                typer.echo()

    except ConfigFileNotFoundError as e:
        typer.secho(
            "Error: Dependency configuration not found",
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
