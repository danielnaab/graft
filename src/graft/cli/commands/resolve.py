"""Resolve command - dependency resolution.

CLI command for resolving knowledge base dependencies.
"""

from pathlib import Path

import typer

from graft.cli.dependency_context_factory import get_dependency_context
from graft.domain.dependency import DependencyStatus
from graft.domain.exceptions import (
    ConfigFileNotFoundError,
    ConfigParseError,
    ConfigValidationError,
    DomainError,
    GitAuthenticationError,
)
from graft.services import config_service, resolution_service

GITIGNORE_ENTRY = ".graft"


def _ensure_gitignore_has_graft() -> bool:
    """Ensure .gitignore contains .graft entry.

    Returns:
        True if .gitignore was modified, False otherwise.
    """
    gitignore_path = Path(".gitignore")

    # Check if .gitignore exists
    if gitignore_path.exists():
        content = gitignore_path.read_text()
        lines = content.splitlines()

        # Check if .graft is already in .gitignore
        if GITIGNORE_ENTRY in lines:
            return False

        # Append .graft to existing .gitignore
        # Ensure there's a newline before adding
        if content and not content.endswith("\n"):
            content += "\n"
        content += f"{GITIGNORE_ENTRY}\n"
        gitignore_path.write_text(content)
        return True
    else:
        # Create new .gitignore with .graft
        gitignore_path.write_text(f"{GITIGNORE_ENTRY}\n")
        return True


def resolve_command() -> None:
    """Resolve dependencies from graft.yaml.

    Reads graft.yaml from current directory and resolves all dependencies
    by cloning or fetching git repositories.

    Example:
        $ graft resolve

        Found configuration: /home/user/project/graft.yaml
        API Version: graft/v0
        Dependencies: 2

        Resolving dependencies...

        ✓ graft-knowledge: resolved to ../graft-knowledge
        ✓ python-starter: resolved to ../python-starter

        Resolved: 2/2

        All dependencies resolved successfully!
    """
    ctx = get_dependency_context()

    # Find and parse configuration
    try:
        config_path = config_service.find_graft_yaml(ctx)
        typer.secho(f"Found configuration: {config_path}", fg=typer.colors.BLUE)

        config = config_service.parse_graft_yaml(ctx, config_path)
        typer.echo(f"API Version: {config.api_version}")
        typer.echo(f"Dependencies: {len(config.dependencies)}")
        typer.echo()

    except ConfigFileNotFoundError as e:
        typer.secho("Error: Configuration file not found", fg=typer.colors.RED, err=True)
        typer.echo(f"  Path: {e.path}", err=True)
        typer.secho(f"  Suggestion: {e.suggestion}", fg=typer.colors.YELLOW, err=True)
        raise typer.Exit(code=1) from e

    except ConfigParseError as e:
        typer.secho("Error: Failed to parse configuration", fg=typer.colors.RED, err=True)
        typer.echo(f"  File: {e.path}", err=True)
        typer.echo(f"  Reason: {e.reason}", err=True)
        typer.secho("  Suggestion: Check YAML syntax", fg=typer.colors.YELLOW, err=True)
        raise typer.Exit(code=1) from e

    except ConfigValidationError as e:
        typer.secho("Error: Invalid configuration", fg=typer.colors.RED, err=True)
        typer.echo(f"  File: {e.path}", err=True)
        typer.echo(f"  Field: {e.field}", err=True)
        typer.echo(f"  Reason: {e.reason}", err=True)
        raise typer.Exit(code=1) from e

    except DomainError as e:
        typer.secho(f"Error: {e}", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=1) from e

    # Resolve dependencies
    typer.echo("Resolving dependencies...")
    typer.echo()

    resolutions = resolution_service.resolve_all_dependencies(ctx, config)

    # Display results
    success_count = 0
    failure_count = 0

    for resolution in resolutions:
        if resolution.status == DependencyStatus.RESOLVED:
            success_count += 1
            msg = f"✓ {resolution.name}: resolved to {resolution.local_path}"
            if resolution.symlink_path:
                msg += f" (symlink: {resolution.symlink_path})"
            typer.secho(msg, fg=typer.colors.GREEN)
        else:
            failure_count += 1
            typer.secho(
                f"✗ {resolution.name}: failed",
                fg=typer.colors.RED,
                err=True,
            )
            if resolution.error_message:
                # Check if it's an authentication error for special handling
                if "Authentication failed" in resolution.error_message:
                    typer.echo(f"  Reason: {resolution.error_message}", err=True)
                    typer.secho(
                        "  Suggestion: Check SSH keys or credentials",
                        fg=typer.colors.YELLOW,
                        err=True,
                    )
                else:
                    typer.echo(f"  Reason: {resolution.error_message}", err=True)

    # Summary
    typer.echo()
    typer.echo(f"Resolved: {success_count}/{len(resolutions)}")

    if failure_count > 0:
        typer.secho(f"Failed: {failure_count}", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=1)

    # Ensure .graft is in .gitignore
    if _ensure_gitignore_has_graft():
        typer.echo()
        typer.secho("Added .graft to .gitignore", fg=typer.colors.BLUE)

    typer.echo()
    typer.secho("All dependencies resolved successfully!", fg=typer.colors.GREEN)
