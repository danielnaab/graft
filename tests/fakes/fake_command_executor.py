"""Fake command executor for testing.

In-memory command executor that records executions.
"""

from graft.protocols.command_executor import CommandResult


class FakeCommandExecutor:
    """Fake command executor for testing.

    Records all command executions and allows configuring responses.
    """

    def __init__(self) -> None:
        """Initialize fake executor."""
        self.executions: list[dict[str, str | dict[str, str] | None]] = []
        self.next_result: CommandResult | None = None
        self.should_raise: Exception | None = None

    def execute(
        self,
        command: str,
        working_dir: str | None = None,
        env: dict[str, str] | None = None,
    ) -> CommandResult:
        """Record execution and return configured result.

        Args:
            command: Shell command to execute
            working_dir: Working directory for command (optional)
            env: Additional environment variables (optional)

        Returns:
            Configured CommandResult or default success

        Raises:
            Exception: If should_raise is configured
        """
        # Record execution
        self.executions.append({
            "command": command,
            "working_dir": working_dir,
            "env": env,
        })

        # Raise if configured
        if self.should_raise:
            raise self.should_raise

        # Return configured result or default success
        if self.next_result:
            result = self.next_result
            self.next_result = None
            return result

        return CommandResult(exit_code=0, stdout="", stderr="")

    def set_next_result(
        self, exit_code: int, stdout: str = "", stderr: str = ""
    ) -> None:
        """Configure next result to return.

        Args:
            exit_code: Exit code to return
            stdout: Standard output to return
            stderr: Standard error to return
        """
        self.next_result = CommandResult(
            exit_code=exit_code,
            stdout=stdout,
            stderr=stderr,
        )

    def set_should_raise(self, exception: Exception) -> None:
        """Configure exception to raise on next execution.

        Args:
            exception: Exception to raise
        """
        self.should_raise = exception

    def get_last_execution(self) -> dict[str, str | dict[str, str] | None]:
        """Get most recent execution record.

        Returns:
            Dictionary with command, working_dir, env

        Raises:
            IndexError: If no executions recorded
        """
        return self.executions[-1]

    def clear(self) -> None:
        """Clear execution history and reset state."""
        self.executions.clear()
        self.next_result = None
        self.should_raise = None
