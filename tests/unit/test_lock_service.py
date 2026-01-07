"""Tests for lock file service."""

from datetime import UTC, datetime

import pytest

from graft.domain.lock_entry import LockEntry
from graft.services import lock_service
from tests.fakes.fake_lock_file import FakeLockFile


@pytest.fixture
def fake_lock_file() -> FakeLockFile:
    """Create fake lock file for testing."""
    return FakeLockFile()


class TestGetLockEntry:
    """Tests for get_lock_entry function."""

    def test_get_existing_entry(self, fake_lock_file: FakeLockFile) -> None:
        """Should return lock entry if it exists."""
        # Setup: Create lock file with entry
        entry = LockEntry(
            source="git@github.com:org/repo.git",
            ref="v1.0.0",
            commit="a" * 40,
            consumed_at=datetime.now(UTC),
        )
        fake_lock_file.write_lock_file("/test/graft.lock", {"test-dep": entry})

        # Execute
        result = lock_service.get_lock_entry(
            fake_lock_file, "/test/graft.lock", "test-dep"
        )

        # Verify
        assert result is not None
        assert result.ref == "v1.0.0"
        assert result.source == "git@github.com:org/repo.git"

    def test_get_nonexistent_entry(self, fake_lock_file: FakeLockFile) -> None:
        """Should return None if dependency not in lock file."""
        # Setup: Empty lock file
        fake_lock_file.write_lock_file("/test/graft.lock", {})

        # Execute
        result = lock_service.get_lock_entry(
            fake_lock_file, "/test/graft.lock", "nonexistent"
        )

        # Verify
        assert result is None

    def test_get_entry_from_nonexistent_lock_file(
        self, fake_lock_file: FakeLockFile
    ) -> None:
        """Should raise FileNotFoundError if lock file doesn't exist."""
        with pytest.raises(FileNotFoundError):
            lock_service.get_lock_entry(
                fake_lock_file, "/test/graft.lock", "test-dep"
            )


class TestUpdateDependencyLock:
    """Tests for update_dependency_lock function."""

    def test_update_existing_dependency(self, fake_lock_file: FakeLockFile) -> None:
        """Should update existing dependency entry."""
        # Setup: Lock file with old version
        old_entry = LockEntry(
            source="git@github.com:org/repo.git",
            ref="v1.0.0",
            commit="a" * 40,
            consumed_at=datetime(2025, 1, 1, tzinfo=UTC),
        )
        fake_lock_file.write_lock_file("/test/graft.lock", {"test-dep": old_entry})

        # Execute: Update to new version
        lock_service.update_dependency_lock(
            fake_lock_file,
            "/test/graft.lock",
            "test-dep",
            "git@github.com:org/repo.git",
            "v2.0.0",
            "b" * 40,
        )

        # Verify
        entries = fake_lock_file.read_lock_file("/test/graft.lock")
        assert "test-dep" in entries
        assert entries["test-dep"].ref == "v2.0.0"
        assert entries["test-dep"].commit == "b" * 40
        # Timestamp should be updated (recent)
        assert entries["test-dep"].consumed_at > old_entry.consumed_at

    def test_add_new_dependency(self, fake_lock_file: FakeLockFile) -> None:
        """Should add new dependency to lock file."""
        # Setup: Lock file with one dependency
        existing_entry = LockEntry(
            source="git@github.com:org/repo1.git",
            ref="v1.0.0",
            commit="a" * 40,
            consumed_at=datetime.now(UTC),
        )
        fake_lock_file.write_lock_file("/test/graft.lock", {"dep1": existing_entry})

        # Execute: Add second dependency
        lock_service.update_dependency_lock(
            fake_lock_file,
            "/test/graft.lock",
            "dep2",
            "git@github.com:org/repo2.git",
            "v1.5.0",
            "b" * 40,
        )

        # Verify
        entries = fake_lock_file.read_lock_file("/test/graft.lock")
        assert len(entries) == 2
        assert "dep1" in entries
        assert "dep2" in entries
        assert entries["dep2"].ref == "v1.5.0"

    def test_create_lock_file_if_not_exists(
        self, fake_lock_file: FakeLockFile
    ) -> None:
        """Should create lock file if it doesn't exist."""
        # Execute: Update dependency when lock file doesn't exist
        lock_service.update_dependency_lock(
            fake_lock_file,
            "/test/graft.lock",
            "new-dep",
            "git@github.com:org/repo.git",
            "v1.0.0",
            "a" * 40,
        )

        # Verify
        assert fake_lock_file.lock_file_exists("/test/graft.lock")
        entries = fake_lock_file.read_lock_file("/test/graft.lock")
        assert "new-dep" in entries


class TestGetAllLockEntries:
    """Tests for get_all_lock_entries function."""

    def test_get_all_from_populated_lock_file(
        self, fake_lock_file: FakeLockFile
    ) -> None:
        """Should return all entries from lock file."""
        # Setup: Lock file with multiple entries
        entries = {
            "dep1": LockEntry(
                source="git@github.com:org/repo1.git",
                ref="v1.0.0",
                commit="a" * 40,
                consumed_at=datetime.now(UTC),
            ),
            "dep2": LockEntry(
                source="git@github.com:org/repo2.git",
                ref="v2.0.0",
                commit="b" * 40,
                consumed_at=datetime.now(UTC),
            ),
        }
        fake_lock_file.write_lock_file("/test/graft.lock", entries)

        # Execute
        result = lock_service.get_all_lock_entries(fake_lock_file, "/test/graft.lock")

        # Verify
        assert len(result) == 2
        assert "dep1" in result
        assert "dep2" in result
        assert result["dep1"].ref == "v1.0.0"
        assert result["dep2"].ref == "v2.0.0"

    def test_get_all_from_empty_lock_file(self, fake_lock_file: FakeLockFile) -> None:
        """Should return empty dict for empty lock file."""
        # Setup: Empty lock file
        fake_lock_file.write_lock_file("/test/graft.lock", {})

        # Execute
        result = lock_service.get_all_lock_entries(fake_lock_file, "/test/graft.lock")

        # Verify
        assert result == {}

    def test_get_all_from_nonexistent_lock_file(
        self, fake_lock_file: FakeLockFile
    ) -> None:
        """Should return empty dict if lock file doesn't exist."""
        # Execute
        result = lock_service.get_all_lock_entries(fake_lock_file, "/test/graft.lock")

        # Verify
        assert result == {}


class TestFindLockFile:
    """Tests for find_lock_file function."""

    def test_find_existing_lock_file(self, fake_lock_file: FakeLockFile) -> None:
        """Should return path to lock file if it exists."""
        # Setup: Create lock file in directory
        fake_lock_file.write_lock_file("/test/dir/graft.lock", {})

        # Execute
        result = lock_service.find_lock_file(fake_lock_file, "/test/dir")

        # Verify
        assert result == "/test/dir/graft.lock"

    def test_find_nonexistent_lock_file(self, fake_lock_file: FakeLockFile) -> None:
        """Should return None if lock file doesn't exist."""
        # Execute
        result = lock_service.find_lock_file(fake_lock_file, "/test/dir")

        # Verify
        assert result is None


class TestCreateEmptyLockFile:
    """Tests for create_empty_lock_file function."""

    def test_create_empty_lock_file(self, fake_lock_file: FakeLockFile) -> None:
        """Should create empty lock file."""
        # Execute
        lock_service.create_empty_lock_file(fake_lock_file, "/test/graft.lock")

        # Verify
        assert fake_lock_file.lock_file_exists("/test/graft.lock")
        entries = fake_lock_file.read_lock_file("/test/graft.lock")
        assert entries == {}
