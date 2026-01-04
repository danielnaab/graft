"""Execute command - run command from dependency's graft.yaml.

CLI command for executing commands defined in a dependency's configuration.
"""

import subprocess
from pathlib import Path

import typer

from graft.cli.dependency_context_factory import get_dependency_context
from graft.domain.exceptions import (
    ConfigFileNotFoundError,
    ConfigParseError,
    ConfigValidationError,
    DomainError,
)
from graft.services import config_service


def exec_dependency_command(dep_name: str, command_name: str, args: list[str] | None = None) -> None:
    """Execute a command from dependency's graft.yaml.

    Args:
        dep_name: Name of dependency
        command_name: Name of command to execute
        args: Additional arguments to pass to command

    Example:
        $ graft meta-kb:migrate-v2

        Running: npx jscodeshift -t codemods/v2.js src/
        Processing 15 files...
        ✓ Completed
    """
    ctx = get_dependency_context()

    try:
        # Find dependency's graft.yaml
        dep_path = Path(ctx.deps_directory) / dep_name / "graft.yaml"
        dep_config_path = str(dep_path)

        # Parse dependency's graft.yaml
        config = config_service.parse_graft_yaml(ctx, dep_config_path)

        # Check if command exists
        if command_name not in config.commands:
            typer.secho(
                f"Error: Command '{command_name}' not found in {dep_name}/graft.yaml",
                fg=typer.colors.RED,
                err=True,
            )
            available_commands = list(config.commands.keys())
            if available_commands:
                typer.echo(
                    f"  Available commands: {', '.join(available_commands)}",
                    err=True,
                )
            else:
                typer.echo(
                    f"  No commands defined in {dep_name}/graft.yaml",
                    err=True,
                )
            raise typer.Exit(code=1)

        cmd = config.commands[command_name]

        # Display what we're running
        typer.secho(f"Executing: {dep_name}:{command_name}", fg=typer.colors.BLUE, bold=True)
        if cmd.description:
            typer.echo(f"  {cmd.description}")
        typer.echo(f"  Command: {cmd.run}")
        if cmd.working_dir:
            typer.echo(f"  Working directory: {cmd.working_dir}")
        typer.echo()

        # Build command with args if provided
        full_command = cmd.run
        if args:
            full_command = f"{cmd.run} {' '.join(args)}"

        # Execute command
        working_dir = cmd.working_dir if cmd.working_dir else "."
        env = None
        if cmd.env:
            import os
            env = os.environ.copy()
            env.update(cmd.env)

        result = subprocess.run(
            full_command,
            shell=True,
            cwd=working_dir,
            env=env,
            # Stream output directly to stdout/stderr
            stdout=None,
            stderr=None,
        )

        # Exit with same code as command
        if result.returncode != 0:
            typer.echo()
            typer.secho(
                f"✗ Command failed with exit code {result.returncode}",
                fg=typer.colors.RED,
                err=True,
            )
            raise typer.Exit(code=result.returncode)

        typer.echo()
        typer.secho("✓ Command completed successfully", fg=typer.colors.GREEN)

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
