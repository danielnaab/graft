"""Run command - execute command from graft.yaml.

CLI command for executing commands defined in current repository's graft.yaml
or dependency's graft.yaml.
"""

import subprocess
from pathlib import Path

import typer

from graft.cli.commands.exec_command import exec_dependency_command
from graft.domain.exceptions import (
    ConfigFileNotFoundError,
    ConfigParseError,
    ConfigValidationError,
    DomainError,
)
from graft.services import config_service


def find_graft_yaml() -> Path | None:
    """Find graft.yaml by searching current directory and parents.

    Searches from current working directory upward (like git does).

    Returns:
        Path to graft.yaml if found, None otherwise
    """
    current = Path.cwd()

    # Search current directory and all parents
    for directory in [current, *current.parents]:
        graft_yaml = directory / "graft.yaml"
        if graft_yaml.exists() and graft_yaml.is_file():
            return graft_yaml

    return None


def list_commands(config_path: str) -> None:
    """List available commands from graft.yaml.

    Args:
        config_path: Path to graft.yaml file
    """
    from graft.cli.dependency_context_factory import get_dependency_context

    ctx = get_dependency_context()

    try:
        config = config_service.parse_graft_yaml(ctx, config_path)

        if not config.commands:
            typer.echo(f"No commands defined in {config_path}")
            return

        typer.secho(f"\nAvailable commands in {config_path}:\n", fg=typer.colors.BLUE, bold=True)

        # Find longest command name for alignment
        max_name_len = max(len(name) for name in config.commands.keys())

        for name, command in config.commands.items():
            description = command.description or ""
            # Align descriptions
            typer.echo(f"  {name:<{max_name_len}}  {description}")

        typer.echo(f"\nUse: graft run <command-name>")

    except (ConfigParseError, ConfigValidationError) as e:
        typer.secho(f"Error: Failed to parse {config_path}", fg=typer.colors.RED, err=True)
        typer.echo(f"  {e}", err=True)
        raise typer.Exit(code=1) from e


def run_current_repo_command(command_name: str, args: list[str] | None = None) -> None:
    """Execute command from current repository's graft.yaml.

    Args:
        command_name: Name of command to execute
        args: Additional arguments to pass to command
    """
    from graft.cli.dependency_context_factory import get_dependency_context

    # Find graft.yaml
    graft_yaml_path = find_graft_yaml()
    if not graft_yaml_path:
        typer.secho(
            "Error: No graft.yaml found in current directory or parent directories",
            fg=typer.colors.RED,
            err=True,
        )
        raise typer.Exit(code=1)

    ctx = get_dependency_context()

    try:
        # Parse graft.yaml
        config = config_service.parse_graft_yaml(ctx, str(graft_yaml_path))

        # Check if command exists
        if command_name not in config.commands:
            typer.secho(
                f"Error: Command '{command_name}' not found in {graft_yaml_path}",
                fg=typer.colors.RED,
                err=True,
            )

            if config.commands:
                typer.echo("\nAvailable commands:", err=True)
                for name, cmd in config.commands.items():
                    desc = cmd.description or ""
                    typer.echo(f"  {name}  {desc}", err=True)
            else:
                typer.echo("  No commands defined in graft.yaml", err=True)

            raise typer.Exit(code=1)

        cmd = config.commands[command_name]

        # Display what we're running
        typer.secho(f"Executing: {command_name}", fg=typer.colors.BLUE, bold=True)
        if cmd.description:
            typer.echo(f"  {cmd.description}")
        typer.echo(f"  Command: {cmd.run}")
        if args:
            typer.echo(f"  Arguments: {' '.join(args)}")
        if cmd.working_dir:
            typer.echo(f"  Working directory: {cmd.working_dir}")
        typer.echo()

        # Build command with args if provided
        full_command = cmd.run
        if args:
            full_command = f"{cmd.run} {' '.join(args)}"

        # Determine working directory
        working_dir = graft_yaml_path.parent
        if cmd.working_dir:
            working_dir = working_dir / cmd.working_dir

        # Build environment
        env = None
        if cmd.env:
            import os
            env = os.environ.copy()
            env.update(cmd.env)

        # Execute command
        result = subprocess.run(
            full_command,
            shell=True,
            cwd=str(working_dir),
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
            f"Error: Configuration file not found: {e.path}",
            fg=typer.colors.RED,
            err=True,
        )
        raise typer.Exit(code=1) from e

    except ConfigParseError as e:
        typer.secho(
            f"Error: Failed to parse {e.path}",
            fg=typer.colors.RED,
            err=True,
        )
        typer.echo(f"  {e.reason}", err=True)
        raise typer.Exit(code=1) from e

    except ConfigValidationError as e:
        typer.secho(
            f"Error: Invalid configuration in {e.path}",
            fg=typer.colors.RED,
            err=True,
        )
        if e.field:
            typer.echo(f"  Field: {e.field}", err=True)
        typer.echo(f"  {e.reason}", err=True)
        raise typer.Exit(code=1) from e

    except DomainError as e:
        typer.secho(f"Error: {e}", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=1) from e


def run_command(
    command: str | None = typer.Argument(None, help="Command name to execute"),
    args: list[str] = typer.Argument(None, help="Arguments to pass to command"),
) -> None:
    """Execute a command from graft.yaml.

    Examples:
        # List available commands
        $ graft run

        # Execute command from current repo
        $ graft run test
        $ graft run build --verbose

        # Execute command from dependency
        $ graft run meta-kb:migrate-v2
    """
    # No command specified - list available commands
    if command is None:
        graft_yaml_path = find_graft_yaml()
        if not graft_yaml_path:
            typer.secho(
                "Error: No graft.yaml found in current directory or parent directories",
                fg=typer.colors.RED,
                err=True,
            )
            raise typer.Exit(code=1)

        list_commands(str(graft_yaml_path))
        return

    # Check if command contains ':' (dependency command)
    if ":" in command:
        # Parse as dep:cmd
        dep_name, cmd_name = command.split(":", 1)
        exec_dependency_command(dep_name, cmd_name, args if args else None)
    else:
        # Execute from current repo
        run_current_repo_command(command, args if args else None)
