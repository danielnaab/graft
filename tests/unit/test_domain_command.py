"""Tests for Command domain model."""

import dataclasses

import pytest

from graft.domain.command import Command
from graft.domain.exceptions import ValidationError


class TestCommand:
    """Tests for Command value object."""

    def test_create_minimal_command(self) -> None:
        """Should create command with just name and run."""
        command = Command(name="test", run="npm test")

        assert command.name == "test"
        assert command.run == "npm test"
        assert command.description is None
        assert command.working_dir is None
        assert command.env == {}

    def test_create_full_command(self) -> None:
        """Should create command with all fields."""
        command = Command(
            name="migrate-v2",
            run="npx jscodeshift -t codemods/v2.js src/",
            description="Rename getUserData to fetchUserData",
            working_dir="src",
            env={"NODE_ENV": "production", "DEBUG": "false"},
        )

        assert command.name == "migrate-v2"
        assert command.run == "npx jscodeshift -t codemods/v2.js src/"
        assert command.description == "Rename getUserData to fetchUserData"
        assert command.working_dir == "src"
        assert command.env == {"NODE_ENV": "production", "DEBUG": "false"}

    def test_empty_name_raises_validation_error(self) -> None:
        """Should raise ValidationError for empty name."""
        with pytest.raises(ValidationError) as exc_info:
            Command(name="", run="npm test")

        assert "Command name cannot be empty" in str(exc_info.value)

    def test_whitespace_name_raises_validation_error(self) -> None:
        """Should raise ValidationError for whitespace-only name."""
        with pytest.raises(ValidationError) as exc_info:
            Command(name="   ", run="npm test")

        assert "Command name cannot be only whitespace" in str(exc_info.value)

    def test_too_long_name_raises_validation_error(self) -> None:
        """Should raise ValidationError for name > 100 chars."""
        long_name = "x" * 101

        with pytest.raises(ValidationError) as exc_info:
            Command(name=long_name, run="npm test")

        assert "Command name too long" in str(exc_info.value)
        assert "100" in str(exc_info.value)

    def test_empty_run_raises_validation_error(self) -> None:
        """Should raise ValidationError for empty run."""
        with pytest.raises(ValidationError) as exc_info:
            Command(name="test", run="")

        assert "'run' field is required" in str(exc_info.value)

    def test_whitespace_run_raises_validation_error(self) -> None:
        """Should raise ValidationError for whitespace-only run."""
        with pytest.raises(ValidationError) as exc_info:
            Command(name="test", run="   ")

        assert "'run' field cannot be only whitespace" in str(exc_info.value)

    def test_too_long_description_raises_validation_error(self) -> None:
        """Should raise ValidationError for description > 500 chars."""
        long_description = "x" * 501

        with pytest.raises(ValidationError) as exc_info:
            Command(name="test", run="npm test", description=long_description)

        assert "description too long" in str(exc_info.value)
        assert "500" in str(exc_info.value)

    def test_whitespace_working_dir_raises_validation_error(self) -> None:
        """Should raise ValidationError for whitespace-only working_dir."""
        with pytest.raises(ValidationError) as exc_info:
            Command(name="test", run="npm test", working_dir="   ")

        assert "working_dir cannot be only whitespace" in str(exc_info.value)

    def test_absolute_path_working_dir_raises_validation_error(self) -> None:
        """Should raise ValidationError for absolute working_dir."""
        with pytest.raises(ValidationError) as exc_info:
            Command(name="test", run="npm test", working_dir="/absolute/path")

        assert "working_dir must be relative, not absolute" in str(exc_info.value)

    def test_windows_absolute_path_working_dir_raises_validation_error(self) -> None:
        """Should raise ValidationError for Windows absolute path."""
        with pytest.raises(ValidationError) as exc_info:
            Command(name="test", run="npm test", working_dir="C:\\absolute\\path")

        assert "working_dir must be relative, not absolute" in str(exc_info.value)

    def test_has_env_vars_when_env_not_empty(self) -> None:
        """Should return True if env dict is not empty."""
        command = Command(name="test", run="npm test", env={"NODE_ENV": "test"})

        assert command.has_env_vars() is True

    def test_has_env_vars_when_env_empty(self) -> None:
        """Should return False if env dict is empty."""
        command1 = Command(name="test", run="npm test")
        command2 = Command(name="test", run="npm test", env={})

        assert command1.has_env_vars() is False
        assert command2.has_env_vars() is False

    def test_get_full_command_without_args(self) -> None:
        """Should return run command as-is without args."""
        command = Command(name="test", run="npm test")

        assert command.get_full_command() == "npm test"

    def test_get_full_command_with_args(self) -> None:
        """Should append args to run command."""
        command = Command(name="test", run="npm test")

        full_cmd = command.get_full_command(["--verbose", "--coverage"])

        assert full_cmd == "npm test --verbose --coverage"

    def test_get_full_command_with_empty_args(self) -> None:
        """Should return run command when args is empty list."""
        command = Command(name="test", run="npm test")

        full_cmd = command.get_full_command([])

        assert full_cmd == "npm test"

    def test_commands_are_frozen(self) -> None:
        """Should not allow modification after creation."""
        command = Command(name="test", run="npm test")

        with pytest.raises(dataclasses.FrozenInstanceError):
            command.name = "changed"  # type: ignore

    def test_commands_with_same_fields_are_equal(self) -> None:
        """Should consider commands equal if all fields match."""
        cmd1 = Command(
            name="test",
            run="npm test",
            description="Run tests",
            working_dir="src",
            env={"NODE_ENV": "test"},
        )
        cmd2 = Command(
            name="test",
            run="npm test",
            description="Run tests",
            working_dir="src",
            env={"NODE_ENV": "test"},
        )

        assert cmd1 == cmd2

    def test_commands_with_different_fields_are_not_equal(self) -> None:
        """Should not be equal if any field differs."""
        base = Command(name="test", run="npm test")
        different_name = Command(name="build", run="npm test")
        different_run = Command(name="test", run="npm build")

        assert base != different_name
        assert base != different_run

    def test_command_repr(self) -> None:
        """Should have helpful repr."""
        command = Command(name="test", run="npm test")

        repr_str = repr(command)

        assert "Command" in repr_str
        assert "test" in repr_str

    def test_env_default_is_empty_dict(self) -> None:
        """Should default to empty dict for env."""
        command = Command(name="test", run="npm test")

        assert command.env == {}
        assert isinstance(command.env, dict)

    def test_multiline_run_command(self) -> None:
        """Should support multiline run commands."""
        multiline_cmd = """
echo "Step 1"
npm install
npm test
"""
        command = Command(name="complex", run=multiline_cmd)

        assert command.run == multiline_cmd

    def test_complex_shell_command(self) -> None:
        """Should support complex shell commands."""
        complex_cmd = 'if [ -f "package.json" ]; then npm test; fi'
        command = Command(name="conditional", run=complex_cmd)

        assert command.run == complex_cmd

    def test_relative_working_dir_variations(self) -> None:
        """Should accept various relative path formats."""
        valid_paths = [
            "src",
            "src/lib",
            "./src",
            "../parent",
            ".",
            "..",
        ]

        for path in valid_paths:
            command = Command(name="test", run="npm test", working_dir=path)
            assert command.working_dir == path

    def test_env_with_various_value_types(self) -> None:
        """Should handle string values in env (as per spec)."""
        env = {
            "STRING_VAR": "value",
            "BOOL_VAR": "true",
            "NUMBER_VAR": "42",
            "EMPTY_VAR": "",
        }

        command = Command(name="test", run="npm test", env=env)

        assert command.env == env
        assert all(isinstance(v, str) for v in command.env.values())

    def test_command_name_with_hyphens_and_underscores(self) -> None:
        """Should support hyphens and underscores in command names."""
        cmd1 = Command(name="migrate-v2", run="./migrate.sh")
        cmd2 = Command(name="verify_migration", run="./verify.sh")
        cmd3 = Command(name="migrate-v2_final", run="./migrate-final.sh")

        assert cmd1.name == "migrate-v2"
        assert cmd2.name == "verify_migration"
        assert cmd3.name == "migrate-v2_final"
