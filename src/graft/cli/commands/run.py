"""Run command - execute command from graft.yaml.

CLI command for executing commands defined in current repository's graft.yaml
or dependency's graft.yaml.
"""

import os
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
    Resolves symlinks to handle linked directories correctly.

    Returns:
        Path to graft.yaml if found, None otherwise
    """
    current = Path.cwd().resolve()  # Resolve symlinks

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
        max_name_len = max(len(name) for name in config.commands)

        for name, command in config.commands.items():
            description = command.description or ""
            # Align descriptions
            typer.echo(f"  {name:<{max_name_len}}  {description}")

        typer.echo("\nUse: graft run <command-name>")

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

        # Build command safely using domain model
        full_command_str = cmd.get_full_command(args)

        # Determine working directory and validate it exists
        working_dir = graft_yaml_path.parent
        if cmd.working_dir:
            working_dir = working_dir / cmd.working_dir
            if not working_dir.exists():
                typer.secho(
                    f"Error: Working directory does not exist: {working_dir}",
                    fg=typer.colors.RED,
                    err=True,
                )
                raise typer.Exit(code=1)

        # Build environment
        env = None
        if cmd.env:
            env = os.environ.copy()
            # Validate all values are strings
            for key, value in cmd.env.items():
                if not isinstance(value, str):
                    typer.secho(
                        f"Error: Environment variable {key} must be string, got {type(value).__name__}",
                        fg=typer.colors.RED,
                        err=True,
                    )
                    raise typer.Exit(code=1)
            env.update(cmd.env)

        # Execute command with proper error handling
        try:
            # SECURITY: Use shell=True but with proper understanding:
            # The command comes from graft.yaml (trusted), but we still
            # validate the source and working directory exist.
            # Arguments are appended by get_full_command() which doesn't escape,
            # but they come from CLI (trusted user input).
            result = subprocess.run(
                full_command_str,
                shell=True,
                cwd=str(working_dir),
                env=env,
                stdout=None,  # Stream to stdout
                stderr=None,  # Stream to stderr
                timeout=None,  # No timeout for user-initiated commands
            )
        except subprocess.TimeoutExpired:
            typer.secho(
                "✗ Command timed out",
                fg=typer.colors.RED,
                err=True,
            )
            raise typer.Exit(code=124) from None  # Standard timeout exit code
        except FileNotFoundError as e:
            typer.secho(
                f"✗ Command not found: {e}",
                fg=typer.colors.RED,
                err=True,
            )
            raise typer.Exit(code=127) from None  # Standard command not found code
        except PermissionError as e:
            typer.secho(
                f"✗ Permission denied: {e}",
                fg=typer.colors.RED,
                err=True,
            )
            raise typer.Exit(code=126) from None  # Standard permission denied code
        except KeyboardInterrupt:
            typer.echo("\n✗ Interrupted by user", err=True)
            raise typer.Exit(code=130) from None  # Standard SIGINT code
        except Exception as e:
            typer.secho(
                f"✗ Failed to execute command: {e}",
                fg=typer.colors.RED,
                err=True,
            )
            raise typer.Exit(code=1) from None

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
        parts = command.split(":", 1)
        if not parts[0] or not parts[1]:
            typer.secho(
                f"Error: Invalid command format: '{command}'",
                fg=typer.colors.RED,
                err=True,
            )
            typer.echo("  Expected format: <dependency>:<command>", err=True)
            raise typer.Exit(code=1)

        dep_name, cmd_name = parts
        exec_dependency_command(dep_name, cmd_name, args if args else None)
    else:
        # Execute from current repo
        run_current_repo_command(command, args if args else None)
