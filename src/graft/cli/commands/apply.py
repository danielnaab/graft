"""Apply command - update lock file without migrations.

CLI command for updating the lock file to acknowledge a version
without running migrations. Useful for manual migration workflows
or initial setup.
"""

import subprocess
from pathlib import Path

import typer

from graft.adapters.lock_file import YamlLockFile
from graft.cli.dependency_context_factory import get_dependency_context
from graft.domain.exceptions import (
    ConfigFileNotFoundError,
    ConfigParseError,
    ConfigValidationError,
    DomainError,
)
from graft.services import config_service, lock_service


def apply_command(
    dep_name: str,
    to: str = typer.Option(..., "--to", help="Target ref to apply (e.g., main, v1.0.0)"),
) -> None:
    """Apply version to lock file without running migrations.

    Updates graft.lock to acknowledge a specific version of a dependency
    without running migration or verification commands. Use this when:
    - Setting up initial dependency versions
    - You've manually performed migrations
    - Syncing lock file with current state

    Args:
        dep_name: Name of dependency to apply
        to: Target ref to apply (e.g., "main", "v1.0.0")

    Example:
        $ graft apply graft-knowledge --to main

        Applied graft-knowledge@main
        Updated graft.lock

        Note: No migrations were run.
    """
    ctx = get_dependency_context()

    try:
        # Step 1: Find and parse consumer's graft.yaml to get dependency source
        consumer_config_path = config_service.find_graft_yaml(ctx)
        consumer_config = config_service.parse_graft_yaml(ctx, consumer_config_path)

        # Check dependency exists in consumer's config
        if dep_name not in consumer_config.dependencies:
            typer.secho(
                f"Error: Dependency '{dep_name}' not found in graft.yaml",
                fg=typer.colors.RED,
                err=True,
            )
            typer.echo(
                f"  Available dependencies: {', '.join(consumer_config.dependencies.keys())}",
                err=True,
            )
            raise typer.Exit(code=1)

        dep_spec = consumer_config.dependencies[dep_name]
        source = dep_spec.git_url.url

        # Step 2: Verify dependency is resolved (directory exists)
        dep_path = Path(ctx.deps_directory) / dep_name
        if not dep_path.exists():
            typer.secho(
                f"Error: Dependency '{dep_name}' not resolved",
                fg=typer.colors.RED,
                err=True,
            )
            typer.echo(
                f"  Expected path: {dep_path}",
                err=True,
            )
            typer.secho(
                "  Run 'graft resolve' first to clone dependencies",
                fg=typer.colors.YELLOW,
                err=True,
            )
            raise typer.Exit(code=1)

        # Step 3: Resolve ref to commit hash
        dep_repo_path = str(dep_path)

        # Try to fetch the ref to ensure we have it locally
        # (this may fail for local-only repos, which is OK)
        fetch_cmd = ["git", "-C", dep_repo_path, "fetch", "origin", to]
        fetch_result = subprocess.run(fetch_cmd, capture_output=True, text=True, check=False)

        # Now try to resolve the ref to a commit hash
        try:
            rev_parse_cmd = ["git", "-C", dep_repo_path, "rev-parse", to]
            result = subprocess.run(
                rev_parse_cmd, capture_output=True, text=True, check=True
            )
            commit = result.stdout.strip()
        except subprocess.CalledProcessError as e:
            # If resolution failed and fetch also failed, show helpful error
            if fetch_result.returncode != 0:
                typer.secho(
                    f"Error: Could not resolve ref '{to}'",
                    fg=typer.colors.RED,
                    err=True,
                )
                typer.echo(f"  Fetch failed: {fetch_result.stderr.strip()}", err=True)
                typer.echo(f"  Resolve failed: {e.stderr.strip()}", err=True)
                typer.secho(
                    "  Suggestion: Ensure the ref exists locally or can be fetched from origin",
                    fg=typer.colors.YELLOW,
                    err=True,
                )
            else:
                typer.secho(
                    f"Error: Failed to resolve ref '{to}' to commit hash",
                    fg=typer.colors.RED,
                    err=True,
                )
                typer.echo(f"  Git error: {e.stderr.strip()}", err=True)
            raise typer.Exit(code=1) from e

        # Step 4: Update lock file
        lock_file = YamlLockFile()
        lock_path = "graft.lock"

        lock_service.update_dependency_lock(
            lock_file,
            lock_path,
            dep_name,
            source,
            to,
            commit,
        )

        # Step 5: Display success
        typer.echo()
        typer.secho(f"Applied {dep_name}@{to}", fg=typer.colors.GREEN, bold=True)
        typer.echo(f"  Source: {source}")
        typer.echo(f"  Commit: {commit[:7]}...")
        typer.echo("Updated graft.lock")
        typer.echo()
        typer.secho(
            "Note: No migrations were run.",
            fg=typer.colors.YELLOW,
        )

    except ConfigFileNotFoundError as e:
        typer.secho("Error: Configuration file not found", fg=typer.colors.RED, err=True)
        typer.echo(f"  Path: {e.path}", err=True)
        typer.secho(f"  Suggestion: {e.suggestion}", fg=typer.colors.YELLOW, err=True)
        raise typer.Exit(code=1) from e

    except ConfigParseError as e:
        typer.secho(
            "Error: Failed to parse configuration", fg=typer.colors.RED, err=True
        )
        typer.echo(f"  File: {e.path}", err=True)
        typer.echo(f"  Reason: {e.reason}", err=True)
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

    except Exception as e:
        typer.secho(
            f"Error: Unexpected error during apply: {e}",
            fg=typer.colors.RED,
            err=True,
        )
        raise typer.Exit(code=1) from e
