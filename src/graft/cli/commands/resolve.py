"""Resolve command - dependency resolution.

CLI command for resolving knowledge base dependencies.

Uses flat-only resolution model (Decision 0007): only direct dependencies
declared in graft.yaml are resolved. There is no transitive resolution.
"""

from pathlib import Path

import typer

from graft.adapters.lock_file import YamlLockFile
from graft.cli.dependency_context_factory import get_dependency_context
from graft.domain.exceptions import (
    ConfigFileNotFoundError,
    ConfigParseError,
    ConfigValidationError,
    DependencyResolutionError,
    DomainError,
)
from graft.services import config_service, lock_service, resolution_service

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

    # Resolve dependencies (flat-only model)
    typer.echo("Resolving dependencies...")
    typer.echo()

    try:
        # Use flat resolution to get all direct dependencies
        lock_entries = resolution_service.resolve_to_lock_entries(ctx, config)

        # Display resolved dependencies
        for name in sorted(lock_entries.keys()):
            entry = lock_entries[name]
            local_path = f"{ctx.deps_directory}/{name}"
            typer.secho(
                f"  ✓ {name}: {entry.ref} → {local_path}",
                fg=typer.colors.GREEN,
            )

        # Write lock file
        typer.echo()
        typer.echo("Writing lock file...")
        lock_file = YamlLockFile()
        lock_file_path = lock_service.find_lock_file(lock_file, ".") or "./graft.lock"
        lock_file.write_lock_file(lock_file_path, lock_entries)
        typer.secho(f"  ✓ Updated {lock_file_path}", fg=typer.colors.GREEN)

        # Summary
        typer.echo()
        typer.echo(f"Resolved: {len(lock_entries)} dependencies")

        # Ensure .graft is in .gitignore
        if _ensure_gitignore_has_graft():
            typer.echo()
            typer.secho("Added .graft to .gitignore", fg=typer.colors.BLUE)

        typer.echo()
        typer.secho("All dependencies resolved successfully!", fg=typer.colors.GREEN)

    except DependencyResolutionError as e:
        typer.secho("Error: Dependency resolution failed", fg=typer.colors.RED, err=True)
        typer.echo(f"  Dependency: {e.dependency_name}", err=True)
        typer.echo(f"  Reason: {e.reason}", err=True)
        raise typer.Exit(code=1) from e
