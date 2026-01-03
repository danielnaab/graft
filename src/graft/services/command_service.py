"""Command execution service.

Service functions for executing dependency-defined commands.
"""

from pathlib import Path

from graft.domain.command import Command
from graft.protocols.command_executor import CommandExecutor, CommandResult


def execute_command(
    executor: CommandExecutor,
    command: Command,
    args: list[str] | None = None,
    base_dir: str | None = None,
) -> CommandResult:
    """Execute a dependency-defined command.

    Args:
        executor: Command executor protocol
        command: Command to execute
        args: Additional command-line arguments (optional)
        base_dir: Base directory for resolving relative working_dir (optional)

    Returns:
        CommandResult with exit code and output

    Raises:
        IOError: If unable to execute command
    """
    # Build full command with args
    full_command = command.get_full_command(args)

    # Resolve working directory
    working_dir = None
    if command.working_dir:
        # Resolve relative to base_dir if provided
        working_dir = str(Path(base_dir) / command.working_dir) if base_dir else command.working_dir

    # Execute
    return executor.execute(
        command=full_command,
        working_dir=working_dir,
        env=command.env,
    )


def execute_command_by_name(
    executor: CommandExecutor,
    commands: dict[str, Command],
    command_name: str,
    args: list[str] | None = None,
    base_dir: str | None = None,
) -> CommandResult:
    """Execute a command by name from command registry.

    Args:
        executor: Command executor protocol
        commands: Dictionary mapping command names to Command objects
        command_name: Name of command to execute
        args: Additional command-line arguments (optional)
        base_dir: Base directory for resolving relative working_dir (optional)

    Returns:
        CommandResult with exit code and output

    Raises:
        KeyError: If command name not found
        IOError: If unable to execute command
    """
    if command_name not in commands:
        raise KeyError(f"Command not found: {command_name}")

    command = commands[command_name]
    return execute_command(executor, command, args, base_dir)
