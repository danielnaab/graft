"""Tests for run command CLI functionality."""

import subprocess
from pathlib import Path
from unittest.mock import Mock, patch

import pytest
import typer

from graft.cli.commands.run import find_graft_yaml, run_current_repo_command


class TestFindGraftYaml:
    """Tests for find_graft_yaml function."""

    def test_find_in_current_directory(self, tmp_path: Path) -> None:
        """Should find graft.yaml in current directory."""
        graft_yaml = tmp_path / "graft.yaml"
        graft_yaml.write_text("apiVersion: graft/v0\ncommands: {}")

        with patch("graft.cli.commands.run.Path.cwd", return_value=tmp_path):
            result = find_graft_yaml()

        assert result == graft_yaml

    def test_find_in_parent_directory(self, tmp_path: Path) -> None:
        """Should find graft.yaml in parent directory."""
        graft_yaml = tmp_path / "graft.yaml"
        graft_yaml.write_text("apiVersion: graft/v0\ncommands: {}")

        subdir = tmp_path / "subdir"
        subdir.mkdir()

        with patch("graft.cli.commands.run.Path.cwd", return_value=subdir):
            result = find_graft_yaml()

        assert result == graft_yaml

    def test_resolves_symlinks(self, tmp_path: Path) -> None:
        """Should resolve symlinks when searching for graft.yaml."""
        # Create real directory with graft.yaml
        real_dir = tmp_path / "real"
        real_dir.mkdir()
        graft_yaml = real_dir / "graft.yaml"
        graft_yaml.write_text("apiVersion: graft/v0\ncommands: {}")

        # Create symlink to real directory
        link_dir = tmp_path / "link"
        link_dir.symlink_to(real_dir)

        with patch("graft.cli.commands.run.Path.cwd", return_value=link_dir):
            result = find_graft_yaml()

        # Should find graft.yaml even when cwd is symlink
        assert result is not None
        assert result.name == "graft.yaml"

    def test_returns_none_when_not_found(self, tmp_path: Path) -> None:
        """Should return None when graft.yaml not found."""
        with patch("graft.cli.commands.run.Path.cwd", return_value=tmp_path):
            result = find_graft_yaml()

        assert result is None


class TestRunCurrentRepoCommand:
    """Tests for run_current_repo_command function."""

    def test_validates_working_dir_exists(self, tmp_path: Path) -> None:
        """Should exit with error if working_dir does not exist."""
        graft_yaml = tmp_path / "graft.yaml"
        graft_yaml.write_text("""apiVersion: graft/v0
commands:
  test:
    run: "echo hello"
    working_dir: "nonexistent"
""")

        with (
            patch("graft.cli.commands.run.find_graft_yaml", return_value=graft_yaml),
            pytest.raises(typer.Exit) as exc_info,
        ):
            run_current_repo_command("test")

        assert exc_info.value.exit_code == 1

    def test_handles_file_not_found_error(self, tmp_path: Path) -> None:
        """Should exit with code 127 when command not found."""
        graft_yaml = tmp_path / "graft.yaml"
        graft_yaml.write_text("""apiVersion: graft/v0
commands:
  test:
    run: "nonexistentcommand12345"
""")

        with (
            patch("graft.cli.commands.run.find_graft_yaml", return_value=graft_yaml),
            patch("subprocess.run", side_effect=FileNotFoundError("command not found")),
            pytest.raises(typer.Exit) as exc_info,
        ):
            run_current_repo_command("test")

        assert exc_info.value.exit_code == 127

    def test_handles_permission_error(self, tmp_path: Path) -> None:
        """Should exit with code 126 when permission denied."""
        graft_yaml = tmp_path / "graft.yaml"
        graft_yaml.write_text("""apiVersion: graft/v0
commands:
  test:
    run: "echo hello"
""")

        with (
            patch("graft.cli.commands.run.find_graft_yaml", return_value=graft_yaml),
            patch("subprocess.run", side_effect=PermissionError("permission denied")),
            pytest.raises(typer.Exit) as exc_info,
        ):
            run_current_repo_command("test")

        assert exc_info.value.exit_code == 126

    def test_handles_keyboard_interrupt(self, tmp_path: Path) -> None:
        """Should exit with code 130 when interrupted by user."""
        graft_yaml = tmp_path / "graft.yaml"
        graft_yaml.write_text("""apiVersion: graft/v0
commands:
  test:
    run: "echo hello"
""")

        with (
            patch("graft.cli.commands.run.find_graft_yaml", return_value=graft_yaml),
            patch("subprocess.run", side_effect=KeyboardInterrupt()),
            pytest.raises(typer.Exit) as exc_info,
        ):
            run_current_repo_command("test")

        assert exc_info.value.exit_code == 130

    def test_handles_timeout_expired(self, tmp_path: Path) -> None:
        """Should exit with code 124 when command times out."""
        graft_yaml = tmp_path / "graft.yaml"
        graft_yaml.write_text("""apiVersion: graft/v0
commands:
  test:
    run: "sleep 100"
""")

        with (
            patch("graft.cli.commands.run.find_graft_yaml", return_value=graft_yaml),
            patch(
                "subprocess.run",
                side_effect=subprocess.TimeoutExpired("sleep 100", 30),
            ),
            pytest.raises(typer.Exit) as exc_info,
        ):
            run_current_repo_command("test")

        assert exc_info.value.exit_code == 124

    def test_validates_env_values_are_strings(self, tmp_path: Path) -> None:
        """Should validate that env values are strings."""
        graft_yaml = tmp_path / "graft.yaml"
        graft_yaml.write_text("""apiVersion: graft/v0
commands:
  test:
    run: "echo hello"
    env:
      VALID: "string"
      INVALID: 123
""")

        with (
            patch("graft.cli.commands.run.find_graft_yaml", return_value=graft_yaml),
            pytest.raises(typer.Exit) as exc_info,
        ):
            run_current_repo_command("test")

        assert exc_info.value.exit_code == 1

    def test_uses_domain_model_get_full_command(self, tmp_path: Path) -> None:
        """Should use Command.get_full_command() to build command string."""
        graft_yaml = tmp_path / "graft.yaml"
        graft_yaml.write_text("""apiVersion: graft/v0
commands:
  test:
    run: "echo"
""")

        mock_result = Mock()
        mock_result.returncode = 0

        with (
            patch("graft.cli.commands.run.find_graft_yaml", return_value=graft_yaml),
            patch("subprocess.run", return_value=mock_result) as mock_run,
        ):
            run_current_repo_command("test", args=["hello", "world"])

        # Verify subprocess.run was called with full command including args
        call_args = mock_run.call_args
        assert call_args[0][0] == "echo hello world"

    def test_passes_env_to_subprocess(self, tmp_path: Path) -> None:
        """Should pass environment variables to subprocess."""
        graft_yaml = tmp_path / "graft.yaml"
        graft_yaml.write_text("""apiVersion: graft/v0
commands:
  test:
    run: "echo hello"
    env:
      TEST_VAR: "test_value"
""")

        mock_result = Mock()
        mock_result.returncode = 0

        with (
            patch("graft.cli.commands.run.find_graft_yaml", return_value=graft_yaml),
            patch("subprocess.run", return_value=mock_result) as mock_run,
        ):
            run_current_repo_command("test")

        # Verify env was passed to subprocess
        call_args = mock_run.call_args
        assert call_args[1]["env"] is not None
        assert "TEST_VAR" in call_args[1]["env"]
        assert call_args[1]["env"]["TEST_VAR"] == "test_value"

    def test_successful_execution_exits_cleanly(self, tmp_path: Path) -> None:
        """Should exit cleanly when command succeeds."""
        graft_yaml = tmp_path / "graft.yaml"
        graft_yaml.write_text("""apiVersion: graft/v0
commands:
  test:
    run: "echo hello"
""")

        mock_result = Mock()
        mock_result.returncode = 0

        with (
            patch("graft.cli.commands.run.find_graft_yaml", return_value=graft_yaml),
            patch("subprocess.run", return_value=mock_result),
        ):
            # Should not raise exception
            run_current_repo_command("test")

    def test_failed_execution_exits_with_command_code(self, tmp_path: Path) -> None:
        """Should exit with command's exit code when it fails."""
        graft_yaml = tmp_path / "graft.yaml"
        graft_yaml.write_text("""apiVersion: graft/v0
commands:
  test:
    run: "exit 42"
""")

        mock_result = Mock()
        mock_result.returncode = 42

        with (
            patch("graft.cli.commands.run.find_graft_yaml", return_value=graft_yaml),
            patch("subprocess.run", return_value=mock_result),
            pytest.raises(typer.Exit) as exc_info,
        ):
            run_current_repo_command("test")

        assert exc_info.value.exit_code == 42
