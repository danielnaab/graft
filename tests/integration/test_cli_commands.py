"""Integration tests for CLI commands.

Tests the CLI by actually executing commands via subprocess.

Note: These tests focus on testing the CLI interface, argument parsing,
and output formatting. They use minimal fixtures to avoid complex
dependency resolution setup.
"""

import json
import subprocess
import tempfile
from pathlib import Path

import pytest


@pytest.fixture
def temp_project_base():
    """Create temporary project with graft.yaml and graft.lock.

    Shared fixture for tests that need a basic project setup.
    """
    with tempfile.TemporaryDirectory() as tmpdir:
        project_dir = Path(tmpdir)

        # Create graft.yaml
        graft_yaml = project_dir / "graft.yaml"
        graft_yaml.write_text("""apiVersion: graft/v0
deps:
  test-dep: "https://github.com/test/repo.git#main"
""")

        # Create graft.lock
        graft_lock = project_dir / "graft.lock"
        graft_lock.write_text("""version: 1
dependencies:
  test-dep:
    source: "https://github.com/test/repo.git"
    ref: "v1.0.0"
    commit: "abc123def456789012345678901234567890abcd"
    consumed_at: "2026-01-04T00:00:00+00:00"
""")

        yield project_dir


class TestCLIHelp:
    """Test --help functionality for all commands."""

    def test_main_help(self):
        """Should display help message."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "--help"],
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0
        assert "graft" in result.stdout.lower()
        assert "status" in result.stdout
        assert "upgrade" in result.stdout

    def test_status_help(self):
        """Should display status command help."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "status", "--help"],
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0
        assert "status" in result.stdout.lower()
        assert "--format" in result.stdout


class TestCLIStatus:
    """Integration tests for 'graft status' command."""

    @pytest.fixture
    def temp_project(self):
        """Create temporary project with graft.yaml and graft.lock."""
        with tempfile.TemporaryDirectory() as tmpdir:
            project_dir = Path(tmpdir)

            # Create graft.yaml
            graft_yaml = project_dir / "graft.yaml"
            graft_yaml.write_text("""apiVersion: graft/v0
deps:
  test-dep: "https://github.com/test/repo.git#main"
""")

            # Create graft.lock
            graft_lock = project_dir / "graft.lock"
            graft_lock.write_text("""version: 1
dependencies:
  test-dep:
    source: "https://github.com/test/repo.git"
    ref: "v1.0.0"
    commit: "abc123def456789012345678901234567890abcd"
    consumed_at: "2026-01-04T00:00:00+00:00"
""")

            yield project_dir

    def test_status_shows_all_dependencies(self, temp_project):
        """Should show all dependencies from lock file."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "status"],
            cwd=temp_project,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0
        assert "test-dep" in result.stdout
        assert "v1.0.0" in result.stdout

    def test_status_specific_dependency(self, temp_project):
        """Should show specific dependency when name provided."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "status", "test-dep"],
            cwd=temp_project,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0
        assert "test-dep" in result.stdout
        assert "v1.0.0" in result.stdout

    def test_status_json_format(self, temp_project):
        """Should output valid JSON with --format json."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "status", "--format", "json"],
            cwd=temp_project,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0

        # Parse JSON
        data = json.loads(result.stdout)
        assert "dependencies" in data
        assert "test-dep" in data["dependencies"]
        assert data["dependencies"]["test-dep"]["current_ref"] == "v1.0.0"
        assert data["dependencies"]["test-dep"]["commit"] == "abc123def456789012345678901234567890abcd"

    def test_status_no_lock_file_shows_message(self, temp_project):
        """Should show helpful message when graft.lock doesn't exist."""
        # Remove lock file
        (temp_project / "graft.lock").unlink()

        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "status"],
            cwd=temp_project,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0
        assert "No dependencies found" in result.stdout
        assert "graft resolve" in result.stdout

    def test_status_invalid_format_option(self, temp_project):
        """Should error on invalid format option."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "status", "--format", "xml"],
            cwd=temp_project,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 1
        assert "Error" in result.stderr
        assert "Invalid format" in result.stderr or "xml" in result.stderr

    def test_status_nonexistent_dependency(self, temp_project):
        """Should error when requesting status for nonexistent dependency."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "status", "nonexistent"],
            cwd=temp_project,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 1
        assert "not found" in result.stderr.lower() or "not found" in result.stdout.lower()

    def test_status_check_updates_flag(self, temp_project):
        """Should fetch and check for updates with --check-updates."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "status", "--check-updates"],
            cwd=temp_project,
            capture_output=True,
            text=True,
        )

        # Should succeed (warns if deps not cloned)
        assert "Checking for updates" in result.stdout

    def test_status_check_updates_json(self, temp_project):
        """Should output JSON with --check-updates --format json."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "status", "--check-updates", "--format", "json"],
            cwd=temp_project,
            capture_output=True,
            text=True,
        )

        # Should output valid JSON
        assert result.returncode == 0
        data = json.loads(result.stdout)
        assert "dependencies" in data


class TestCLIChanges:
    """Integration tests for 'graft changes' command."""

    @pytest.fixture
    def temp_project_with_dep(self):
        """Create temporary project with dependency that has changes."""
        with tempfile.TemporaryDirectory() as tmpdir:
            # Create parent directory that will contain both project and dependency
            base_dir = Path(tmpdir)

            # Create project directory
            project_dir = base_dir / "project"
            project_dir.mkdir()

            # Create graft.yaml in project
            graft_yaml = project_dir / "graft.yaml"
            graft_yaml.write_text("""apiVersion: graft/v0
deps:
  test-dep: "https://github.com/test/repo.git#main"
""")

            # Create dependency as sibling directory (../test-dep from project's perspective)
            dep_dir = base_dir / "test-dep"
            dep_dir.mkdir()

            dep_graft_yaml = dep_dir / "graft.yaml"
            dep_graft_yaml.write_text("""apiVersion: graft/v0
changes:
  v1.0.0:
    type: feature
    description: "Initial release"
  v2.0.0:
    type: breaking
    description: "Major refactor"
    migration: migrate-v2
    verify: verify-v2

commands:
  migrate-v2:
    run: "./migrate.sh"
    description: "Migrate to v2"
  verify-v2:
    run: "./verify.sh"
    description: "Verify v2"
""")

            yield project_dir

    def test_changes_lists_all(self, temp_project_with_dep):
        """Should list all changes for dependency."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "changes", "test-dep"],
            cwd=temp_project_with_dep,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0
        assert "v1.0.0" in result.stdout
        assert "v2.0.0" in result.stdout
        assert "feature" in result.stdout
        assert "breaking" in result.stdout

    def test_changes_filter_breaking(self, temp_project_with_dep):
        """Should filter to show only breaking changes."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "changes", "test-dep", "--breaking"],
            cwd=temp_project_with_dep,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0
        assert "v2.0.0" in result.stdout
        assert "breaking" in result.stdout

    def test_changes_json_format(self, temp_project_with_dep):
        """Should output valid JSON with --format json."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "changes", "test-dep", "--format", "json"],
            cwd=temp_project_with_dep,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0

        # Parse JSON
        data = json.loads(result.stdout)
        assert "changes" in data
        assert len(data["changes"]) == 2
        assert data["dependency"] == "test-dep"

    def test_changes_since_option(self, temp_project_with_dep):
        """Should support --since alias for --from-ref."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "changes", "test-dep", "--since", "v1.0.0"],
            cwd=temp_project_with_dep,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0

    def test_changes_invalid_format(self, temp_project_with_dep):
        """Should error on invalid format option."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "changes", "test-dep", "--format", "yaml"],
            cwd=temp_project_with_dep,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 1
        assert "Error" in result.stderr
        assert "Invalid format" in result.stderr or "yaml" in result.stderr

    def test_changes_conflicting_options(self, temp_project_with_dep):
        """Should error when both --since and --from-ref are provided."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "changes", "test-dep", "--since", "v1.0.0", "--from-ref", "v1.0.0"],
            cwd=temp_project_with_dep,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 1
        assert "Error" in result.stderr
        assert "both" in result.stderr.lower() or "conflict" in result.stderr.lower()


class TestCLIShow:
    """Integration tests for 'graft show' command."""

    @pytest.fixture
    def temp_project_with_dep(self):
        """Create temporary project with dependency."""
        with tempfile.TemporaryDirectory() as tmpdir:
            # Create parent directory that will contain both project and dependency
            base_dir = Path(tmpdir)

            # Create project directory
            project_dir = base_dir / "project"
            project_dir.mkdir()

            # Create graft.yaml in project
            graft_yaml = project_dir / "graft.yaml"
            graft_yaml.write_text("""apiVersion: graft/v0
deps:
  test-dep: "https://github.com/test/repo.git#main"
""")

            # Create dependency as sibling directory (../test-dep from project's perspective)
            dep_dir = base_dir / "test-dep"
            dep_dir.mkdir()

            dep_graft_yaml = dep_dir / "graft.yaml"
            dep_graft_yaml.write_text("""apiVersion: graft/v0
changes:
  v2.0.0:
    type: breaking
    description: "Major refactor"
    migration: migrate-v2
    verify: verify-v2

commands:
  migrate-v2:
    run: "./migrate.sh"
    description: "Migrate to v2"
  verify-v2:
    run: "./verify.sh"
    description: "Verify v2"
""")

            yield project_dir

    def test_show_displays_change_details(self, temp_project_with_dep):
        """Should show detailed information about a change."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "show", "test-dep@v2.0.0"],
            cwd=temp_project_with_dep,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0
        assert "v2.0.0" in result.stdout
        assert "breaking" in result.stdout
        assert "Major refactor" in result.stdout
        assert "migrate-v2" in result.stdout

    def test_show_json_format(self, temp_project_with_dep):
        """Should output valid JSON with --format json."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "show", "test-dep@v2.0.0", "--format", "json"],
            cwd=temp_project_with_dep,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0

        # Parse JSON
        data = json.loads(result.stdout)
        assert data["dependency"] == "test-dep"
        assert data["ref"] == "v2.0.0"
        assert data["type"] == "breaking"
        assert data["description"] == "Major refactor"
        assert "migration" in data
        assert "verify" in data

    def test_show_field_option(self, temp_project_with_dep):
        """Should show only specific field with --field."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "show", "test-dep@v2.0.0", "--field", "type"],
            cwd=temp_project_with_dep,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0
        assert "breaking" in result.stdout

    def test_show_invalid_ref_error(self, temp_project_with_dep):
        """Should error when ref doesn't exist."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "show", "test-dep@v99.0.0"],
            cwd=temp_project_with_dep,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 1

    def test_show_invalid_field(self, temp_project_with_dep):
        """Should error on invalid field option."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "show", "test-dep@v2.0.0", "--field", "author"],
            cwd=temp_project_with_dep,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 1
        assert "Error" in result.stderr
        assert "Invalid field" in result.stderr or "author" in result.stderr

    def test_show_missing_at_symbol(self, temp_project_with_dep):
        """Should error when @ symbol is missing from dep_ref."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "show", "test-dep-v2.0.0"],
            cwd=temp_project_with_dep,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 1
        assert "Error" in result.stderr
        assert "@" in result.stderr or "Invalid format" in result.stderr


class TestCLIExecCommand:
    """Integration tests for 'graft <dep>:<command>' syntax."""

    @pytest.fixture
    def temp_project_with_commands(self):
        """Create temporary project with dependency that has commands."""
        with tempfile.TemporaryDirectory() as tmpdir:
            # Create parent directory that will contain both project and dependency
            base_dir = Path(tmpdir)

            # Create project directory
            project_dir = base_dir / "project"
            project_dir.mkdir()

            # Create graft.yaml in project
            graft_yaml = project_dir / "graft.yaml"
            graft_yaml.write_text("""apiVersion: graft/v0
deps:
  test-dep: "https://github.com/test/repo.git#main"
""")

            # Create dependency as sibling directory (../test-dep from project's perspective)
            dep_dir = base_dir / "test-dep"
            dep_dir.mkdir()

            dep_graft_yaml = dep_dir / "graft.yaml"
            dep_graft_yaml.write_text("""apiVersion: graft/v0
commands:
  test-cmd:
    run: "echo 'Hello from test-cmd'"
    description: "Test command"

  failing-cmd:
    run: "exit 1"
    description: "Command that fails"
""")

            yield project_dir

    def test_exec_command_success(self, temp_project_with_commands):
        """Should execute command and show output."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "test-dep:test-cmd"],
            cwd=temp_project_with_commands,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0
        assert "Hello from test-cmd" in result.stdout

    def test_exec_command_not_found_error(self, temp_project_with_commands):
        """Should error when command doesn't exist."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "test-dep:nonexistent"],
            cwd=temp_project_with_commands,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 1

    def test_exec_command_failing_command(self, temp_project_with_commands):
        """Should return non-zero exit code when command fails."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "test-dep:failing-cmd"],
            cwd=temp_project_with_commands,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 1


class TestValidateCommand:
    """Tests for graft validate command."""

    def test_validate_help(self):
        """Should display validate help message."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "validate", "--help"],
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0
        assert "validate" in result.stdout.lower()

    @pytest.fixture
    def temp_project_for_validation(self):
        """Create temporary project for validation testing."""
        with tempfile.TemporaryDirectory() as tmpdir:
            project_dir = Path(tmpdir)

            # Create valid graft.yaml
            graft_yaml = project_dir / "graft.yaml"
            graft_yaml.write_text("""apiVersion: graft/v0
deps:
  test-dep: "https://github.com/test/repo.git#main"
""")

            # Create valid graft.lock
            graft_lock = project_dir / "graft.lock"
            graft_lock.write_text("""version: 1
dependencies:
  test-dep:
    source: "https://github.com/test/repo.git"
    ref: "v1.0.0"
    commit: "abc123def456789012345678901234567890abcd"
    consumed_at: "2026-01-04T00:00:00+00:00"
""")

            yield project_dir

    def test_validate_schema_only(self, temp_project_for_validation):
        """Should validate schema with --schema flag."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "validate", "--schema"],
            cwd=temp_project_for_validation,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0
        assert "Schema is valid" in result.stdout

    def test_validate_missing_graft_yaml(self):
        """Should error when graft.yaml not found."""
        with tempfile.TemporaryDirectory() as tmpdir:
            result = subprocess.run(
                ["uv", "run", "python", "-m", "graft", "validate"],
                cwd=tmpdir,
                capture_output=True,
                text=True,
            )

            assert result.returncode == 1
            assert "graft.yaml not found" in result.stderr

    def test_validate_missing_lock_file_warning(self):
        """Should warn when graft.lock not found."""
        with tempfile.TemporaryDirectory() as tmpdir:
            project_dir = Path(tmpdir)

            # Create graft.yaml but no lock file
            graft_yaml = project_dir / "graft.yaml"
            graft_yaml.write_text("""apiVersion: graft/v0
deps:
  test-dep: "https://github.com/test/repo.git#main"
""")

            result = subprocess.run(
                ["uv", "run", "python", "-m", "graft", "validate"],
                cwd=project_dir,
                capture_output=True,
                text=True,
            )

            # Should succeed with warning (exit 0)
            assert result.returncode == 0
            assert "graft.lock not found" in result.stdout
            assert "warning" in result.stdout.lower()

    def test_validate_lock_only(self, temp_project_for_validation):
        """Should validate lock file only with --lock flag."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "validate", "--lock"],
            cwd=temp_project_for_validation,
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0
        assert "Validating graft.lock" in result.stdout
        assert "Validating graft.yaml" not in result.stdout

    def test_validate_invalid_yaml(self):
        """Should error on malformed YAML."""
        with tempfile.TemporaryDirectory() as tmpdir:
            project_dir = Path(tmpdir)

            # Create invalid YAML (invalid indentation)
            graft_yaml = project_dir / "graft.yaml"
            graft_yaml.write_text("""apiVersion: graft/v0
deps:
test-dep: "https://github.com/test/repo.git#main"
""")

            result = subprocess.run(
                ["uv", "run", "python", "-m", "graft", "validate"],
                cwd=project_dir,
                capture_output=True,
                text=True,
            )

            assert result.returncode == 1
            assert "Failed to parse" in result.stderr or "error" in result.stderr.lower()

    def test_validate_no_dependencies_error(self):
        """Should error when no dependencies defined."""
        with tempfile.TemporaryDirectory() as tmpdir:
            project_dir = Path(tmpdir)

            # Create graft.yaml with no dependencies
            graft_yaml = project_dir / "graft.yaml"
            graft_yaml.write_text("""apiVersion: graft/v0
deps: {}
""")

            result = subprocess.run(
                ["uv", "run", "python", "-m", "graft", "validate"],
                cwd=project_dir,
                capture_output=True,
                text=True,
            )

            assert result.returncode == 1
            assert "No dependencies defined" in result.stderr

    def test_validate_mutually_exclusive_flags_error(self):
        """Should error when multiple flags are used together."""
        with tempfile.TemporaryDirectory() as tmpdir:
            project_dir = Path(tmpdir)

            # Create valid graft.yaml
            graft_yaml = project_dir / "graft.yaml"
            graft_yaml.write_text("""apiVersion: graft/v0
deps:
  test-dep: "https://github.com/test/repo.git#main"
""")

            # Try using --schema and --lock together
            result = subprocess.run(
                ["uv", "run", "python", "-m", "graft", "validate", "--schema", "--lock"],
                cwd=project_dir,
                capture_output=True,
                text=True,
            )

            assert result.returncode == 1
            assert "mutually exclusive" in result.stderr

    def test_validate_refs_only_flag(self, temp_project_for_validation):
        """Should validate only refs when --refs flag is used."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "validate", "--refs"],
            cwd=temp_project_for_validation,
            capture_output=True,
            text=True,
        )

        # Should succeed (even if deps not cloned, should warn)
        # Should NOT validate lock file
        assert "Validating graft.yaml" in result.stdout
        assert "Validating graft.lock" not in result.stdout


class TestFetchCommand:
    """Tests for graft fetch command."""

    def test_fetch_help(self):
        """Should display fetch help message."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "fetch", "--help"],
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0
        assert "fetch" in result.stdout.lower()

    def test_fetch_missing_graft_yaml(self):
        """Should error when graft.yaml not found."""
        with tempfile.TemporaryDirectory() as tmpdir:
            result = subprocess.run(
                ["uv", "run", "python", "-m", "graft", "fetch"],
                cwd=tmpdir,
                capture_output=True,
                text=True,
            )

            assert result.returncode == 1
            assert "graft.yaml not found" in result.stderr

    def test_fetch_nonexistent_dependency(self):
        """Should error when specified dependency doesn't exist."""
        with tempfile.TemporaryDirectory() as tmpdir:
            project_dir = Path(tmpdir)

            # Create graft.yaml
            graft_yaml = project_dir / "graft.yaml"
            graft_yaml.write_text("""apiVersion: graft/v0
deps:
  test-dep: "https://github.com/test/repo.git#main"
""")

            result = subprocess.run(
                ["uv", "run", "python", "-m", "graft", "fetch", "nonexistent"],
                cwd=project_dir,
                capture_output=True,
                text=True,
            )

            assert result.returncode == 1
            assert "not found" in result.stderr

    def test_fetch_dependency_not_cloned(self):
        """Should warn when dependency not cloned."""
        with tempfile.TemporaryDirectory() as tmpdir:
            project_dir = Path(tmpdir)

            # Create graft.yaml
            graft_yaml = project_dir / "graft.yaml"
            graft_yaml.write_text("""apiVersion: graft/v0
deps:
  test-dep: "https://github.com/test/repo.git#main"
""")

            result = subprocess.run(
                ["uv", "run", "python", "-m", "graft", "fetch"],
                cwd=project_dir,
                capture_output=True,
                text=True,
            )

            # Should warn but not fail completely
            assert "not cloned" in result.stdout
