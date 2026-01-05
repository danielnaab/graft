"""Resolve command - dependency resolution.

CLI command for resolving knowledge base dependencies.
"""

import typer

from graft.adapters.lock_file import YamlLockFile
from graft.cli.dependency_context_factory import get_dependency_context
from graft.domain.dependency import DependencyStatus
from graft.domain.exceptions import (
    ConfigFileNotFoundError,
    ConfigParseError,
    ConfigValidationError,
    DependencyResolutionError,
    DomainError,
)
from graft.services import config_service, lock_service, resolution_service


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

    # Resolve dependencies recursively (v2)
    typer.echo("Resolving dependencies (including transitive)...")
    typer.echo()

    try:
        # Use recursive resolution to get all dependencies
        lock_entries = resolution_service.resolve_all_recursive(ctx, config)

        # Separate direct and transitive dependencies
        direct_deps = {name: entry for name, entry in lock_entries.items() if entry.direct}
        transitive_deps = {
            name: entry for name, entry in lock_entries.items() if not entry.direct
        }

        # Display direct dependencies
        if direct_deps:
            typer.echo("Direct dependencies:")
            for name in sorted(direct_deps.keys()):
                entry = direct_deps[name]
                local_path = f"{ctx.deps_directory}/{name}"
                typer.secho(
                    f"  ✓ {name}: {entry.ref} → {local_path}",
                    fg=typer.colors.GREEN,
                )

        # Display transitive dependencies
        if transitive_deps:
            typer.echo()
            typer.echo("Transitive dependencies:")
            for name in sorted(transitive_deps.keys()):
                entry = transitive_deps[name]
                local_path = f"{ctx.deps_directory}/{name}"
                parents = ", ".join(entry.required_by)
                typer.secho(
                    f"  ✓ {name}: {entry.ref} → {local_path} (via {parents})",
                    fg=typer.colors.BRIGHT_BLACK,
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
        total_count = len(lock_entries)
        direct_count = len(direct_deps)
        transitive_count = len(transitive_deps)

        typer.echo(f"Resolved: {total_count} dependencies")
        typer.echo(f"  Direct: {direct_count}")
        if transitive_count > 0:
            typer.echo(f"  Transitive: {transitive_count}")

        typer.echo()
        typer.secho("All dependencies resolved successfully!", fg=typer.colors.GREEN)

    except DependencyResolutionError as e:
        typer.secho("Error: Dependency resolution failed", fg=typer.colors.RED, err=True)
        typer.echo(f"  Dependency: {e.dependency_name}", err=True)
        typer.echo(f"  Reason: {e.reason}", err=True)
        raise typer.Exit(code=1) from e
