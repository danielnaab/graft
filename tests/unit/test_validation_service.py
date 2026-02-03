"""Unit tests for validation service.

Tests validation logic for graft.yaml and graft.lock files.
"""

from datetime import UTC, datetime

import pytest

from graft.domain.config import GraftConfig
from graft.domain.dependency import DependencySpec, GitRef, GitUrl
from graft.domain.lock_entry import LockEntry
from graft.services import validation_service
from tests.fakes.fake_filesystem import FakeFileSystem
from tests.fakes.fake_git import FakeGitOperations


@pytest.fixture
def fake_git() -> FakeGitOperations:
    """Provide fresh fake git operations."""
    git = FakeGitOperations()
    yield git
    git.reset()


@pytest.fixture
def fake_filesystem() -> FakeFileSystem:
    """Provide fresh fake filesystem."""
    return FakeFileSystem()


class TestValidateConfigSchema:
    """Tests for validate_config_schema function.

    Note: GraftConfig already validates in __post_init__, so these tests
    verify that the validation service provides additional/consistent validation.
    """

    def test_valid_config_returns_no_errors(self):
        """Should return empty list for valid configuration."""
        config = GraftConfig(
            api_version="graft/v0",
            dependencies={
                "test-dep": DependencySpec(
                    name="test-dep",
                    git_url=GitUrl("https://github.com/test/repo.git"),
                    git_ref=GitRef("main"),
                )
            },
            changes={},
            commands={},
            metadata={},
        )

        errors = validation_service.validate_config_schema(config)

        assert len(errors) == 0

    def test_no_dependencies(self):
        """Should error when no dependencies are defined."""
        config = GraftConfig(
            api_version="graft/v0",
            dependencies={},
            changes={},
            commands={},
            metadata={},
        )

        errors = validation_service.validate_config_schema(config)

        assert len(errors) == 1
        assert "No dependencies defined" in errors[0].message


# Note: Command reference validation tests removed because this validation
# happens in GraftConfig.__post_init__ during domain model construction.
# Tests for domain validation belong in tests/unit/test_config.py instead.


class TestGetValidationSummary:
    """Tests for get_validation_summary function."""

    def test_separate_errors_and_warnings(self):
        """Should separate errors and warnings correctly."""
        validation_errors = [
            validation_service.ValidationError("Error 1", severity="error"),
            validation_service.ValidationError("Warning 1", severity="warning"),
            validation_service.ValidationError("Error 2", severity="error"),
            validation_service.ValidationError("Warning 2", severity="warning"),
        ]

        errors, warnings = validation_service.get_validation_summary(validation_errors)

        assert len(errors) == 2
        assert "Error 1" in errors
        assert "Error 2" in errors

        assert len(warnings) == 2
        assert "Warning 1" in warnings
        assert "Warning 2" in warnings

    def test_empty_list(self):
        """Should return empty lists for empty input."""
        errors, warnings = validation_service.get_validation_summary([])

        assert errors == []
        assert warnings == []

    def test_only_errors(self):
        """Should return only errors when no warnings."""
        validation_errors = [
            validation_service.ValidationError("Error 1", severity="error"),
            validation_service.ValidationError("Error 2", severity="error"),
        ]

        errors, warnings = validation_service.get_validation_summary(validation_errors)

        assert len(errors) == 2
        assert len(warnings) == 0

    def test_only_warnings(self):
        """Should return only warnings when no errors."""
        validation_errors = [
            validation_service.ValidationError("Warning 1", severity="warning"),
            validation_service.ValidationError("Warning 2", severity="warning"),
        ]

        errors, warnings = validation_service.get_validation_summary(validation_errors)

        assert len(errors) == 0
        assert len(warnings) == 2


class TestValidateIntegrity:
    """Tests for validate_integrity function."""

    def test_integrity_pass_when_commits_match(
        self,
        fake_filesystem: FakeFileSystem,
        fake_git: FakeGitOperations,
    ) -> None:
        """Should pass when commits match lock file."""
        commit = "a" * 40

        # Setup repository at correct commit
        fake_filesystem.mkdir("/deps/my-dep")
        fake_git._cloned_repos["/deps/my-dep"] = (
            "https://github.com/user/repo.git",
            "v1.0.0",
        )
        fake_git.configure_current_commit("/deps/my-dep", commit)

        lock_entries = {
            "my-dep": LockEntry(
                source="https://github.com/user/repo.git",
                ref="v1.0.0",
                commit=commit,
                consumed_at=datetime.now(UTC),
            ),
        }

        results = validation_service.validate_integrity(
            filesystem=fake_filesystem,
            git=fake_git,
            deps_directory="/deps",
            lock_entries=lock_entries,
        )

        assert len(results) == 1
        assert results[0].valid is True
        assert results[0].name == "my-dep"
        assert "Commit matches" in results[0].message

    def test_integrity_fail_when_commits_differ(
        self,
        fake_filesystem: FakeFileSystem,
        fake_git: FakeGitOperations,
    ) -> None:
        """Should fail when commit differs from lock file."""
        expected_commit = "a" * 40
        actual_commit = "b" * 40

        # Setup repository at wrong commit
        fake_filesystem.mkdir("/deps/my-dep")
        fake_git._cloned_repos["/deps/my-dep"] = (
            "https://github.com/user/repo.git",
            "v1.0.0",
        )
        fake_git.configure_current_commit("/deps/my-dep", actual_commit)

        lock_entries = {
            "my-dep": LockEntry(
                source="https://github.com/user/repo.git",
                ref="v1.0.0",
                commit=expected_commit,
                consumed_at=datetime.now(UTC),
            ),
        }

        results = validation_service.validate_integrity(
            filesystem=fake_filesystem,
            git=fake_git,
            deps_directory="/deps",
            lock_entries=lock_entries,
        )

        assert len(results) == 1
        assert results[0].valid is False
        assert "mismatch" in results[0].message
        assert expected_commit[:7] in results[0].message
        assert actual_commit[:7] in results[0].message

    def test_integrity_fail_when_dependency_missing(
        self,
        fake_filesystem: FakeFileSystem,
        fake_git: FakeGitOperations,
    ) -> None:
        """Should fail when dependency is not cloned."""
        lock_entries = {
            "my-dep": LockEntry(
                source="https://github.com/user/repo.git",
                ref="v1.0.0",
                commit="a" * 40,
                consumed_at=datetime.now(UTC),
            ),
        }

        results = validation_service.validate_integrity(
            filesystem=fake_filesystem,
            git=fake_git,
            deps_directory="/deps",
            lock_entries=lock_entries,
        )

        assert len(results) == 1
        assert results[0].valid is False
        assert "not found" in results[0].message

    def test_integrity_fail_when_not_git_repo(
        self,
        fake_filesystem: FakeFileSystem,
        fake_git: FakeGitOperations,
    ) -> None:
        """Should fail when path exists but is not a git repository."""
        # Create directory but don't mark as repo
        fake_filesystem.mkdir("/deps/my-dep")

        lock_entries = {
            "my-dep": LockEntry(
                source="https://github.com/user/repo.git",
                ref="v1.0.0",
                commit="a" * 40,
                consumed_at=datetime.now(UTC),
            ),
        }

        results = validation_service.validate_integrity(
            filesystem=fake_filesystem,
            git=fake_git,
            deps_directory="/deps",
            lock_entries=lock_entries,
        )

        assert len(results) == 1
        assert results[0].valid is False
        assert "not a git repository" in results[0].message

    def test_integrity_multiple_dependencies(
        self,
        fake_filesystem: FakeFileSystem,
        fake_git: FakeGitOperations,
    ) -> None:
        """Should validate all dependencies and return results for each."""
        commit1 = "a" * 40
        commit2 = "b" * 40
        wrong_commit = "c" * 40

        # Setup first dep at correct commit
        fake_filesystem.mkdir("/deps/dep1")
        fake_git._cloned_repos["/deps/dep1"] = ("url1", "v1.0.0")
        fake_git.configure_current_commit("/deps/dep1", commit1)

        # Setup second dep at wrong commit
        fake_filesystem.mkdir("/deps/dep2")
        fake_git._cloned_repos["/deps/dep2"] = ("url2", "v2.0.0")
        fake_git.configure_current_commit("/deps/dep2", wrong_commit)

        lock_entries = {
            "dep1": LockEntry(
                source="url1",
                ref="v1.0.0",
                commit=commit1,
                consumed_at=datetime.now(UTC),
            ),
            "dep2": LockEntry(
                source="url2",
                ref="v2.0.0",
                commit=commit2,
                consumed_at=datetime.now(UTC),
            ),
        }

        results = validation_service.validate_integrity(
            filesystem=fake_filesystem,
            git=fake_git,
            deps_directory="/deps",
            lock_entries=lock_entries,
        )

        assert len(results) == 2
        # First should pass
        dep1_result = next(r for r in results if r.name == "dep1")
        assert dep1_result.valid is True
        # Second should fail
        dep2_result = next(r for r in results if r.name == "dep2")
        assert dep2_result.valid is False
