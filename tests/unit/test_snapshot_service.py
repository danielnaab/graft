"""Tests for snapshot service."""

import pytest

from graft.services.snapshot_service import (
    cleanup_old_snapshots,
    cleanup_snapshot,
    create_workspace_snapshot,
    get_snapshot_paths_for_dependency,
    restore_workspace_snapshot,
)
from tests.fakes.fake_snapshot import FakeSnapshot


class TestCreateWorkspaceSnapshot:
    """Tests for create_workspace_snapshot()."""

    def test_creates_snapshot_with_specified_paths(self):
        """Should create snapshot with specified paths."""
        fake = FakeSnapshot()
        fake.set_file_content("file1.txt", "content1")
        fake.set_file_content("file2.txt", "content2")

        snapshot_id = create_workspace_snapshot(
            fake,
            ["file1.txt", "file2.txt"],
            "/base/dir",
        )

        assert snapshot_id.startswith("snapshot-")
        assert fake.snapshot_exists(snapshot_id)

    def test_returns_snapshot_id(self):
        """Should return snapshot ID for later use."""
        fake = FakeSnapshot()
        fake.set_file_content("file.txt", "content")

        snapshot_id = create_workspace_snapshot(fake, ["file.txt"], "/base")

        assert isinstance(snapshot_id, str)
        assert len(snapshot_id) > 0

    def test_raises_error_if_path_not_found(self):
        """Should raise FileNotFoundError if path doesn't exist."""
        fake = FakeSnapshot()

        with pytest.raises(FileNotFoundError, match="nonexistent.txt"):
            create_workspace_snapshot(fake, ["nonexistent.txt"], "/base")


class TestRestoreWorkspaceSnapshot:
    """Tests for restore_workspace_snapshot()."""

    def test_restores_files_from_snapshot(self):
        """Should restore files to their snapshotted state."""
        fake = FakeSnapshot()
        fake.set_file_content("file.txt", "original")

        snapshot_id = create_workspace_snapshot(fake, ["file.txt"], "/base")

        # Modify file
        fake.set_file_content("file.txt", "modified")
        assert fake.get_file_content("file.txt") == "modified"

        # Restore
        restore_workspace_snapshot(fake, snapshot_id)

        assert fake.get_file_content("file.txt") == "original"

    def test_restores_multiple_files(self):
        """Should restore multiple files from snapshot."""
        fake = FakeSnapshot()
        fake.set_file_content("file1.txt", "content1")
        fake.set_file_content("file2.txt", "content2")

        snapshot_id = create_workspace_snapshot(
            fake,
            ["file1.txt", "file2.txt"],
            "/base",
        )

        # Modify files
        fake.set_file_content("file1.txt", "new1")
        fake.set_file_content("file2.txt", "new2")

        # Restore
        restore_workspace_snapshot(fake, snapshot_id)

        assert fake.get_file_content("file1.txt") == "content1"
        assert fake.get_file_content("file2.txt") == "content2"

    def test_raises_error_if_snapshot_not_found(self):
        """Should raise ValueError if snapshot doesn't exist."""
        fake = FakeSnapshot()

        with pytest.raises(ValueError, match="Snapshot not found"):
            restore_workspace_snapshot(fake, "nonexistent")


class TestCleanupSnapshot:
    """Tests for cleanup_snapshot()."""

    def test_deletes_snapshot(self):
        """Should delete specified snapshot."""
        fake = FakeSnapshot()
        fake.set_file_content("file.txt", "content")

        snapshot_id = create_workspace_snapshot(fake, ["file.txt"], "/base")

        assert fake.snapshot_exists(snapshot_id)

        cleanup_snapshot(fake, snapshot_id)

        assert not fake.snapshot_exists(snapshot_id)

    def test_raises_error_if_snapshot_not_found(self):
        """Should raise ValueError if snapshot doesn't exist."""
        fake = FakeSnapshot()

        with pytest.raises(ValueError, match="Snapshot not found"):
            cleanup_snapshot(fake, "nonexistent")


class TestCleanupOldSnapshots:
    """Tests for cleanup_old_snapshots()."""

    def test_keeps_most_recent_snapshots(self):
        """Should keep specified number of recent snapshots."""
        fake = FakeSnapshot()
        fake.set_file_content("file.txt", "content")

        # Create 5 snapshots
        ids = []
        for _ in range(5):
            snapshot_id = create_workspace_snapshot(fake, ["file.txt"], "/base")
            ids.append(snapshot_id)

        # Keep 3 most recent
        deleted = cleanup_old_snapshots(fake, keep_count=3)

        assert len(deleted) == 2
        # Check that the 3 most recent still exist
        remaining = fake.list_snapshots()
        assert len(remaining) == 3

    def test_deletes_oldest_snapshots_first(self):
        """Should delete oldest snapshots first."""
        fake = FakeSnapshot()
        fake.set_file_content("file.txt", "content")

        # Create snapshots
        old_id = create_workspace_snapshot(fake, ["file.txt"], "/base")
        new_id = create_workspace_snapshot(fake, ["file.txt"], "/base")

        # Keep 1
        deleted = cleanup_old_snapshots(fake, keep_count=1)

        assert old_id in deleted
        assert fake.snapshot_exists(new_id)
        assert not fake.snapshot_exists(old_id)

    def test_returns_empty_list_if_under_limit(self):
        """Should return empty list if snapshot count under limit."""
        fake = FakeSnapshot()
        fake.set_file_content("file.txt", "content")

        create_workspace_snapshot(fake, ["file.txt"], "/base")

        deleted = cleanup_old_snapshots(fake, keep_count=5)

        assert deleted == []

    def test_handles_zero_snapshots(self):
        """Should handle case with no snapshots."""
        fake = FakeSnapshot()

        deleted = cleanup_old_snapshots(fake, keep_count=3)

        assert deleted == []


class TestGetSnapshotPathsForDependency:
    """Tests for get_snapshot_paths_for_dependency()."""

    def test_returns_dependency_paths(self):
        """Should return typical paths for dependency upgrade."""
        paths = get_snapshot_paths_for_dependency("my-dep")

        assert ".graft/deps/my-dep" in paths
        assert "graft.lock" in paths

    def test_includes_dependency_specific_path(self):
        """Should include dependency-specific path."""
        paths = get_snapshot_paths_for_dependency("other-dep")

        assert ".graft/deps/other-dep" in paths

    def test_returns_list_of_strings(self):
        """Should return list of string paths."""
        paths = get_snapshot_paths_for_dependency("dep")

        assert isinstance(paths, list)
        assert all(isinstance(p, str) for p in paths)
