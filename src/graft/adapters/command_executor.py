"""Command executor adapter implementation.

Subprocess-based command execution.
"""

import os
import subprocess
from pathlib import Path

from graft.protocols.command_executor import CommandResult


class SubprocessCommandExecutor:
    """Execute commands using subprocess.

    Runs shell commands with environment control and captures output.
    """

    def execute(
        self,
        command: str,
        working_dir: str | None = None,
        env: dict[str, str] | None = None,
    ) -> CommandResult:
        """Execute shell command using subprocess.

        Args:
            command: Shell command to execute
            working_dir: Working directory for command (optional)
            env: Additional environment variables (optional)

        Returns:
            CommandResult with exit code and output

        Raises:
            IOError: If unable to execute command
        """
        # Validate working directory if provided
        if working_dir:
            working_path = Path(working_dir)
            if not working_path.exists():
                raise IOError(f"Working directory does not exist: {working_dir}")
            if not working_path.is_dir():
                raise IOError(f"Working directory is not a directory: {working_dir}")

        # Build environment
        command_env = os.environ.copy()
        if env:
            command_env.update(env)

        try:
            # Execute command
            result = subprocess.run(
                command,
                shell=True,
                cwd=working_dir,
                env=command_env,
                capture_output=True,
                text=True,
                check=False,  # Don't raise exception on non-zero exit
            )

            return CommandResult(
                exit_code=result.returncode,
                stdout=result.stdout,
                stderr=result.stderr,
            )

        except Exception as e:
            raise IOError(f"Failed to execute command: {e}") from e
