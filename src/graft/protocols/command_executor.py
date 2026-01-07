"""Command executor protocol.

Protocol for executing shell commands with environment control.
"""

from typing import Protocol


class CommandResult:
    """Result of command execution.

    Attributes:
        exit_code: Command exit code (0 for success)
        stdout: Standard output as string
        stderr: Standard error as string
    """

    def __init__(self, exit_code: int, stdout: str, stderr: str) -> None:
        """Initialize command result.

        Args:
            exit_code: Command exit code
            stdout: Standard output
            stderr: Standard error
        """
        self.exit_code = exit_code
        self.stdout = stdout
        self.stderr = stderr

    @property
    def success(self) -> bool:
        """Check if command succeeded.

        Returns:
            True if exit code is 0
        """
        return self.exit_code == 0

    def __repr__(self) -> str:
        """Return string representation."""
        return (
            f"CommandResult(exit_code={self.exit_code}, "
            f"stdout={len(self.stdout)} chars, "
            f"stderr={len(self.stderr)} chars)"
        )


class CommandExecutor(Protocol):
    """Protocol for executing shell commands.

    Enables running commands with custom working directory
    and environment variables.
    """

    def execute(
        self,
        command: str,
        working_dir: str | None = None,
        env: dict[str, str] | None = None,
    ) -> CommandResult:
        """Execute shell command.

        Args:
            command: Shell command to execute
            working_dir: Working directory for command (optional)
            env: Additional environment variables (optional)

        Returns:
            CommandResult with exit code and output

        Raises:
            IOError: If unable to execute command
        """
        ...
