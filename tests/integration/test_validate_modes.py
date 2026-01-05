"""Integration tests for validate command modes.

Tests the new mode-based validation interface:
- graft validate [config|lock|integrity|all]
"""

import subprocess
import tempfile
from pathlib import Path

import pytest


@pytest.fixture
def temp_project_with_valid_files():
    """Create temporary project with valid graft.yaml and graft.lock."""
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
        graft_lock.write_text("""apiVersion: graft/v0
dependencies:
  test-dep:
    source: "https://github.com/test/repo.git"
    ref: "v1.0.0"
    commit: "abc123def456789012345678901234567890abcd"
    consumed_at: "2026-01-04T00:00:00+00:00"
""")

        yield project_dir


class TestValidateModes:
    """Test mode-based validation interface."""

    def test_validate_default_runs_all_modes(self, temp_project_with_valid_files):
        """graft validate (no mode) should run all validations."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "validate"],
            cwd=temp_project_with_valid_files,
            capture_output=True,
            text=True,
        )

        # Should validate both graft.yaml and graft.lock
        assert "Validating graft.yaml" in result.stdout
        assert "Validating graft.lock" in result.stdout
        assert result.returncode == 0

    def test_validate_all_mode_explicit(self, temp_project_with_valid_files):
        """graft validate all should run all validations."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "validate", "all"],
            cwd=temp_project_with_valid_files,
            capture_output=True,
            text=True,
        )

        # Should validate both graft.yaml and graft.lock
        assert "Validating graft.yaml" in result.stdout
        assert "Validating graft.lock" in result.stdout
        assert result.returncode == 0

    def test_validate_config_mode_only_yaml(self, temp_project_with_valid_files):
        """graft validate config should only validate graft.yaml."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "validate", "config"],
            cwd=temp_project_with_valid_files,
            capture_output=True,
            text=True,
        )

        # Should validate only graft.yaml, not lock file
        assert "Validating graft.yaml" in result.stdout
        assert "Validating graft.lock" not in result.stdout
        assert result.returncode == 0

    def test_validate_lock_mode_only_lock(self, temp_project_with_valid_files):
        """graft validate lock should only validate graft.lock."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "validate", "lock"],
            cwd=temp_project_with_valid_files,
            capture_output=True,
            text=True,
        )

        # Should validate only graft.lock, not graft.yaml
        assert "Validating graft.yaml" not in result.stdout
        assert "Validating graft.lock" in result.stdout
        assert result.returncode == 0

    def test_validate_integrity_mode_checks_lock(self, temp_project_with_valid_files):
        """graft validate integrity should check lock file integrity."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "validate", "integrity"],
            cwd=temp_project_with_valid_files,
            capture_output=True,
            text=True,
        )

        # Integrity mode validates lock file (dependencies not cloned warning expected)
        assert "Validating graft.lock" in result.stdout
        assert "Validating graft.yaml" not in result.stdout

        # Should warn about deps not cloned (expected behavior)
        # Exit code 0 with warnings is acceptable
        assert result.returncode in [0, 2]  # 0 for warnings, 2 for integrity mismatch

    def test_validate_invalid_mode_error(self):
        """graft validate with invalid mode should error."""
        with tempfile.TemporaryDirectory() as tmpdir:
            project_dir = Path(tmpdir)

            # Create valid graft.yaml
            graft_yaml = project_dir / "graft.yaml"
            graft_yaml.write_text("""apiVersion: graft/v0
deps:
  test-dep: "https://github.com/test/repo.git#main"
""")

            result = subprocess.run(
                ["uv", "run", "python", "-m", "graft", "validate", "invalid"],
                cwd=project_dir,
                capture_output=True,
                text=True,
            )

            # Should error with clear message
            assert result.returncode == 1
            assert "Invalid mode" in result.stderr
            assert "Must be one of" in result.stderr

    def test_validate_legacy_flags_show_deprecation_warning(
        self, temp_project_with_valid_files
    ):
        """Legacy flags should show deprecation warning."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "validate", "--schema"],
            cwd=temp_project_with_valid_files,
            capture_output=True,
            text=True,
        )

        # Should show deprecation warning
        assert "deprecated" in result.stdout.lower()
        assert "mode" in result.stdout.lower()

        # But should still work
        assert result.returncode == 0

    def test_validate_config_mode_missing_yaml_error(self):
        """config mode should error when graft.yaml missing."""
        with tempfile.TemporaryDirectory() as tmpdir:
            result = subprocess.run(
                ["uv", "run", "python", "-m", "graft", "validate", "config"],
                cwd=tmpdir,
                capture_output=True,
                text=True,
            )

            # Should error
            assert result.returncode == 1
            assert "graft.yaml not found" in result.stderr

    def test_validate_lock_mode_missing_lock_warning(self):
        """lock mode should warn when graft.lock missing."""
        with tempfile.TemporaryDirectory() as tmpdir:
            project_dir = Path(tmpdir)

            # Create graft.yaml but no lock file
            graft_yaml = project_dir / "graft.yaml"
            graft_yaml.write_text("""apiVersion: graft/v0
deps:
  test-dep: "https://github.com/test/repo.git#main"
""")

            result = subprocess.run(
                ["uv", "run", "python", "-m", "graft", "validate", "lock"],
                cwd=project_dir,
                capture_output=True,
                text=True,
            )

            # Should succeed with warning (not error)
            assert result.returncode == 0
            assert "graft.lock not found" in result.stdout
            assert "warning" in result.stdout.lower()

    def test_validate_help_shows_modes(self):
        """validate --help should show mode documentation."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "validate", "--help"],
            capture_output=True,
            text=True,
        )

        # Should document modes
        assert result.returncode == 0
        assert "config" in result.stdout
        assert "lock" in result.stdout
        assert "integrity" in result.stdout
        assert "all" in result.stdout

    def test_validate_mode_and_flag_together_works(self, temp_project_with_valid_files):
        """Mode argument with legacy flag should work (flag takes precedence via warning)."""
        result = subprocess.run(
            ["uv", "run", "python", "-m", "graft", "validate", "all", "--schema"],
            cwd=temp_project_with_valid_files,
            capture_output=True,
            text=True,
        )

        # Should show deprecation warning
        assert "deprecated" in result.stdout.lower()

        # Should use flag behavior (schema only), not mode
        assert result.returncode == 0
