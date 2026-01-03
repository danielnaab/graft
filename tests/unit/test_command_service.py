"""Tests for command execution service."""

import pytest

from graft.domain.command import Command
from graft.services import command_service
from tests.fakes.fake_command_executor import FakeCommandExecutor


@pytest.fixture
def fake_executor() -> FakeCommandExecutor:
    """Create fake command executor for testing."""
    return FakeCommandExecutor()


class TestExecuteCommand:
    """Tests for execute_command function."""

    def test_execute_simple_command(self, fake_executor: FakeCommandExecutor) -> None:
        """Should execute command with basic settings."""
        # Setup
        command = Command(
            name="test-cmd",
            run="echo hello",
            description="Test command",
        )

        # Execute
        result = command_service.execute_command(fake_executor, command)

        # Verify execution was called
        assert len(fake_executor.executions) == 1
        execution = fake_executor.get_last_execution()
        assert execution["command"] == "echo hello"
        assert execution["working_dir"] is None
        assert execution["env"] == {}  # Command has default empty dict for env

        # Verify result
        assert result.success
        assert result.exit_code == 0

    def test_execute_command_with_args(self, fake_executor: FakeCommandExecutor) -> None:
        """Should append args to command."""
        # Setup
        command = Command(
            name="test-cmd",
            run="python script.py",
        )

        # Execute with args
        result = command_service.execute_command(
            fake_executor, command, args=["--flag", "value"]
        )

        # Verify args were appended
        execution = fake_executor.get_last_execution()
        assert execution["command"] == "python script.py --flag value"

    def test_execute_command_with_working_dir(
        self, fake_executor: FakeCommandExecutor
    ) -> None:
        """Should use command's working directory."""
        # Setup
        command = Command(
            name="test-cmd",
            run="make build",
            working_dir="subdir",
        )

        # Execute
        command_service.execute_command(fake_executor, command)

        # Verify working_dir was passed
        execution = fake_executor.get_last_execution()
        assert execution["working_dir"] == "subdir"

    def test_execute_command_with_base_dir(
        self, fake_executor: FakeCommandExecutor
    ) -> None:
        """Should resolve working_dir relative to base_dir."""
        # Setup
        command = Command(
            name="test-cmd",
            run="make build",
            working_dir="subdir",
        )

        # Execute with base_dir
        command_service.execute_command(
            fake_executor, command, base_dir="/home/user/project"
        )

        # Verify working_dir was resolved
        execution = fake_executor.get_last_execution()
        assert execution["working_dir"] == "/home/user/project/subdir"

    def test_execute_command_with_env_vars(
        self, fake_executor: FakeCommandExecutor
    ) -> None:
        """Should pass environment variables."""
        # Setup
        command = Command(
            name="test-cmd",
            run="npm test",
            env={"NODE_ENV": "test", "DEBUG": "true"},
        )

        # Execute
        command_service.execute_command(fake_executor, command)

        # Verify env vars were passed
        execution = fake_executor.get_last_execution()
        assert execution["env"] == {"NODE_ENV": "test", "DEBUG": "true"}

    def test_execute_command_returns_stdout(
        self, fake_executor: FakeCommandExecutor
    ) -> None:
        """Should return command stdout."""
        # Setup
        command = Command(name="test-cmd", run="echo hello")
        fake_executor.set_next_result(exit_code=0, stdout="hello\n")

        # Execute
        result = command_service.execute_command(fake_executor, command)

        # Verify output
        assert result.stdout == "hello\n"
        assert result.success

    def test_execute_command_returns_stderr(
        self, fake_executor: FakeCommandExecutor
    ) -> None:
        """Should return command stderr."""
        # Setup
        command = Command(name="test-cmd", run="echo error >&2")
        fake_executor.set_next_result(exit_code=1, stderr="error\n")

        # Execute
        result = command_service.execute_command(fake_executor, command)

        # Verify error output
        assert result.stderr == "error\n"
        assert not result.success
        assert result.exit_code == 1

    def test_execute_command_propagates_ioerror(
        self, fake_executor: FakeCommandExecutor
    ) -> None:
        """Should propagate IOError from executor."""
        # Setup
        command = Command(name="test-cmd", run="invalid")
        fake_executor.set_should_raise(IOError("Command failed"))

        # Execute and verify error
        with pytest.raises(IOError) as exc_info:
            command_service.execute_command(fake_executor, command)

        assert "Command failed" in str(exc_info.value)


class TestExecuteCommandByName:
    """Tests for execute_command_by_name function."""

    def test_execute_command_by_name(
        self, fake_executor: FakeCommandExecutor
    ) -> None:
        """Should execute command from registry by name."""
        # Setup command registry
        commands = {
            "build": Command(name="build", run="make build"),
            "test": Command(name="test", run="npm test"),
        }

        # Execute
        result = command_service.execute_command_by_name(
            fake_executor, commands, "build"
        )

        # Verify correct command was executed
        execution = fake_executor.get_last_execution()
        assert execution["command"] == "make build"
        assert result.success

    def test_execute_nonexistent_command_raises(
        self, fake_executor: FakeCommandExecutor
    ) -> None:
        """Should raise KeyError for nonexistent command."""
        # Setup empty registry
        commands: dict[str, Command] = {}

        # Execute and verify error
        with pytest.raises(KeyError) as exc_info:
            command_service.execute_command_by_name(
                fake_executor, commands, "nonexistent"
            )

        assert "Command not found: nonexistent" in str(exc_info.value)

    def test_execute_command_by_name_with_args(
        self, fake_executor: FakeCommandExecutor
    ) -> None:
        """Should pass args when executing by name."""
        # Setup
        commands = {
            "test": Command(name="test", run="pytest"),
        }

        # Execute with args
        command_service.execute_command_by_name(
            fake_executor, commands, "test", args=["-v", "tests/"]
        )

        # Verify args were included
        execution = fake_executor.get_last_execution()
        assert execution["command"] == "pytest -v tests/"

    def test_execute_command_by_name_with_base_dir(
        self, fake_executor: FakeCommandExecutor
    ) -> None:
        """Should resolve working_dir when executing by name."""
        # Setup
        commands = {
            "build": Command(
                name="build", run="make", working_dir="build"
            ),
        }

        # Execute with base_dir
        command_service.execute_command_by_name(
            fake_executor, commands, "build", base_dir="/project"
        )

        # Verify working_dir was resolved
        execution = fake_executor.get_last_execution()
        assert execution["working_dir"] == "/project/build"
