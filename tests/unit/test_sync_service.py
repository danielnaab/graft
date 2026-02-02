"""Unit tests for sync service.

Tests for sync_dependency and sync_all_dependencies service functions.
"""

from datetime import UTC, datetime

import pytest

from graft.domain.lock_entry import LockEntry
from graft.services import sync_service
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


class TestSyncDependency:
    """Tests for sync_dependency service function."""

    def test_sync_clones_missing_dependency(
        self,
        fake_filesystem: FakeFileSystem,
        fake_git: FakeGitOperations,
    ) -> None:
        """Should clone dependency that doesn't exist."""
        entry = LockEntry(
            source="https://github.com/user/repo.git",
            ref="v1.0.0",
            commit="a" * 40,
            consumed_at=datetime.now(UTC),
        )

        result = sync_service.sync_dependency(
            filesystem=fake_filesystem,
            git=fake_git,
            deps_directory="/deps",
            name="my-dep",
            entry=entry,
        )

        assert result.success is True
        assert result.action == "cloned"
        assert "a" * 7 in result.message  # Short commit hash

    def test_sync_checks_out_correct_commit(
        self,
        fake_filesystem: FakeFileSystem,
        fake_git: FakeGitOperations,
    ) -> None:
        """Should checkout the locked commit for existing repo with different commit."""
        # Setup existing repo with different commit
        fake_filesystem.mkdir("/deps/my-dep")
        fake_git._cloned_repos["/deps/my-dep"] = (
            "https://github.com/user/repo.git",
            "v0.9.0",
        )
        fake_git.configure_current_commit("/deps/my-dep", "b" * 40)

        entry = LockEntry(
            source="https://github.com/user/repo.git",
            ref="v1.0.0",
            commit="a" * 40,
            consumed_at=datetime.now(UTC),
        )

        result = sync_service.sync_dependency(
            filesystem=fake_filesystem,
            git=fake_git,
            deps_directory="/deps",
            name="my-dep",
            entry=entry,
        )

        assert result.success is True
        assert result.action == "checked_out"
        assert "a" * 7 in result.message

    def test_sync_up_to_date_when_commit_matches(
        self,
        fake_filesystem: FakeFileSystem,
        fake_git: FakeGitOperations,
    ) -> None:
        """Should report up-to-date when commit already matches."""
        commit = "a" * 40

        # Setup existing repo at correct commit
        fake_filesystem.mkdir("/deps/my-dep")
        fake_git._cloned_repos["/deps/my-dep"] = (
            "https://github.com/user/repo.git",
            "v1.0.0",
        )
        fake_git.configure_current_commit("/deps/my-dep", commit)

        entry = LockEntry(
            source="https://github.com/user/repo.git",
            ref="v1.0.0",
            commit=commit,
            consumed_at=datetime.now(UTC),
        )

        result = sync_service.sync_dependency(
            filesystem=fake_filesystem,
            git=fake_git,
            deps_directory="/deps",
            name="my-dep",
            entry=entry,
        )

        assert result.success is True
        assert result.action == "up_to_date"

    def test_sync_fails_for_non_git_directory(
        self,
        fake_filesystem: FakeFileSystem,
        fake_git: FakeGitOperations,
    ) -> None:
        """Should fail if path exists but is not a git repository."""
        # Create directory but don't mark it as a repo
        fake_filesystem.mkdir("/deps/my-dep")

        entry = LockEntry(
            source="https://github.com/user/repo.git",
            ref="v1.0.0",
            commit="a" * 40,
            consumed_at=datetime.now(UTC),
        )

        result = sync_service.sync_dependency(
            filesystem=fake_filesystem,
            git=fake_git,
            deps_directory="/deps",
            name="my-dep",
            entry=entry,
        )

        assert result.success is False
        assert result.action == "failed"
        assert "not a git repository" in result.message


class TestSyncAllDependencies:
    """Tests for sync_all_dependencies service function."""

    def test_sync_all_returns_results_for_each(
        self,
        fake_filesystem: FakeFileSystem,
        fake_git: FakeGitOperations,
    ) -> None:
        """Should return results for each dependency."""
        entries = {
            "dep1": LockEntry(
                source="https://github.com/user/repo1.git",
                ref="v1.0.0",
                commit="a" * 40,
                consumed_at=datetime.now(UTC),
            ),
            "dep2": LockEntry(
                source="https://github.com/user/repo2.git",
                ref="v2.0.0",
                commit="b" * 40,
                consumed_at=datetime.now(UTC),
            ),
        }

        results = sync_service.sync_all_dependencies(
            filesystem=fake_filesystem,
            git=fake_git,
            deps_directory="/deps",
            lock_entries=entries,
        )

        assert len(results) == 2
        assert all(r.success for r in results)

    def test_sync_all_continues_on_failure(
        self,
        fake_filesystem: FakeFileSystem,
        fake_git: FakeGitOperations,
    ) -> None:
        """Should continue syncing even if one dependency fails."""
        # Create non-git directory to cause failure
        fake_filesystem.mkdir("/deps/dep1")

        entries = {
            "dep1": LockEntry(
                source="https://github.com/user/repo1.git",
                ref="v1.0.0",
                commit="a" * 40,
                consumed_at=datetime.now(UTC),
            ),
            "dep2": LockEntry(
                source="https://github.com/user/repo2.git",
                ref="v2.0.0",
                commit="b" * 40,
                consumed_at=datetime.now(UTC),
            ),
        }

        results = sync_service.sync_all_dependencies(
            filesystem=fake_filesystem,
            git=fake_git,
            deps_directory="/deps",
            lock_entries=entries,
        )

        assert len(results) == 2
        # First should fail, second should succeed
        assert results[0].name == "dep1"
        assert results[0].success is False
        assert results[1].name == "dep2"
        assert results[1].success is True
