"""Pytest fixtures for black-box integration testing of the Graft CLI.

Provides isolated test environments with subprocess execution and filesystem inspection.
"""

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any, List, Optional

import pytest


@pytest.fixture
def examples_dir() -> Path:
    """Path to the examples directory in the project root."""
    return Path(__file__).parent.parent / "examples"


@pytest.fixture
def tmp_repo(tmp_path: Path, examples_dir: Path) -> Path:
    """Create an isolated copy of the agile-ops example in a temp directory.

    This fixture provides a clean workspace for each test, preventing tests
    from interfering with each other or modifying the actual examples.
    """
    src = examples_dir / "agile-ops"
    dst = tmp_path / "agile-ops"
    shutil.copytree(src, dst)
    return dst


@pytest.fixture
def empty_repo(tmp_path: Path) -> Path:
    """Create an empty repository directory for tests that need to start fresh."""
    repo = tmp_path / "test-repo"
    repo.mkdir()
    return repo


class GraftResult:
    """Wrapper for subprocess results with convenient assertions."""

    def __init__(self, completed: subprocess.CompletedProcess):
        self.returncode = completed.returncode
        self.stdout = completed.stdout
        self.stderr = completed.stderr
        self._completed = completed

    def assert_success(self) -> "GraftResult":
        """Assert that the command succeeded (returncode == 0)."""
        assert self.returncode == 0, (
            f"Command failed with exit code {self.returncode}\n"
            f"stdout: {self.stdout}\n"
            f"stderr: {self.stderr}"
        )
        return self

    def assert_failure(self) -> "GraftResult":
        """Assert that the command failed (returncode != 0)."""
        assert self.returncode != 0, (
            f"Command unexpectedly succeeded\n"
            f"stdout: {self.stdout}"
        )
        return self

    def json(self) -> dict:
        """Parse stdout as JSON."""
        try:
            return json.loads(self.stdout)
        except json.JSONDecodeError as e:
            raise AssertionError(
                f"Failed to parse JSON from stdout:\n{self.stdout}"
            ) from e

    def assert_json_contains(self, **expected: Any) -> "GraftResult":
        """Assert that JSON output contains the expected key-value pairs."""
        data = self.json()
        for key, value in expected.items():
            assert key in data, f"Expected key '{key}' not found in JSON output"
            assert data[key] == value, (
                f"Expected {key}={value}, got {key}={data[key]}"
            )
        return self


@pytest.fixture
def run_graft(tmp_path: Path):
    """Factory fixture to run graft CLI commands in isolated environments.

    Usage:
        result = run_graft("explain", "artifacts/sprint-brief/", "--json", cwd=repo_path)
        result.assert_success()
        data = result.json()
    """
    def _run(*args: str, cwd: Optional[Path] = None, env: Optional[dict] = None) -> GraftResult:
        """Execute a graft CLI command and return the result.

        Args:
            *args: Command arguments (e.g., "explain", "artifacts/sprint-brief/", "--json")
            cwd: Working directory for the command (defaults to tmp_path)
            env: Environment variables to set (merged with current environment)

        Returns:
            GraftResult with stdout, stderr, and returncode
        """
        cmd = [sys.executable, "-m", "graft.cli"] + list(args)

        # Merge environment variables
        cmd_env = os.environ.copy()
        if env:
            cmd_env.update(env)

        completed = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            cwd=str(cwd or tmp_path),
            env=cmd_env
        )

        return GraftResult(completed)

    return _run


@pytest.fixture
def assert_file_exists():
    """Helper to assert that a file exists at the given path."""
    def _assert(path: Path, msg: str = "") -> Path:
        assert path.exists(), msg or f"Expected file to exist: {path}"
        assert path.is_file(), f"Expected path to be a file: {path}"
        return path
    return _assert


@pytest.fixture
def assert_file_contains():
    """Helper to assert that a file contains expected content."""
    def _assert(path: Path, expected: str, msg: str = "") -> None:
        assert path.exists(), f"File does not exist: {path}"
        content = path.read_text()
        assert expected in content, msg or (
            f"Expected content not found in {path}\n"
            f"Looking for: {expected}\n"
            f"File contents:\n{content}"
        )
    return _assert


@pytest.fixture
def assert_json_file():
    """Helper to load and validate JSON files."""
    def _assert(path: Path) -> dict:
        assert path.exists(), f"JSON file does not exist: {path}"
        try:
            return json.loads(path.read_text())
        except json.JSONDecodeError as e:
            raise AssertionError(f"Invalid JSON in {path}: {e}") from e
    return _assert


@pytest.fixture
def file_tree():
    """Helper to get a sorted list of all files in a directory tree.

    Useful for asserting that certain files were created or modified.
    """
    def _tree(root: Path, relative: bool = True) -> List[Path]:
        """Get all files under root, optionally as relative paths."""
        files = sorted(root.rglob("*"))
        if relative:
            files = [f.relative_to(root) for f in files if f.is_file()]
        else:
            files = [f for f in files if f.is_file()]
        return files
    return _tree
