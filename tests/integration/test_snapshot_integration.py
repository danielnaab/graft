"""Integration tests for filesystem snapshot adapter."""

import tempfile
from pathlib import Path

import pytest

from graft.adapters.snapshot import FilesystemSnapshot


class TestFilesystemSnapshot:
    """Integration tests for FilesystemSnapshot."""

    @pytest.fixture
    def temp_dir(self):
        """Create temporary directory for tests."""
        with tempfile.TemporaryDirectory() as tmpdir:
            yield tmpdir

    @pytest.fixture
    def snapshot(self, temp_dir):
        """Create FilesystemSnapshot instance."""
        return FilesystemSnapshot(snapshot_dir=f"{temp_dir}/.graft/snapshots")

    def test_create_and_restore_single_file(self, snapshot, temp_dir):
        """Should create snapshot and restore single file."""
        # Create test file
        test_file = Path(temp_dir) / "test.txt"
        test_file.write_text("original content")

        # Create snapshot
        snapshot_id = snapshot.create_snapshot(["test.txt"], temp_dir)

        # Modify file
        test_file.write_text("modified content")
        assert test_file.read_text() == "modified content"

        # Restore
        snapshot.restore_snapshot(snapshot_id)

        # Verify restored
        assert test_file.read_text() == "original content"

    def test_create_snapshot_with_multiple_files(self, snapshot, temp_dir):
        """Should snapshot multiple files."""
        # Create test files
        file1 = Path(temp_dir) / "file1.txt"
        file2 = Path(temp_dir) / "file2.txt"
        file1.write_text("content1")
        file2.write_text("content2")

        # Create snapshot
        snapshot_id = snapshot.create_snapshot(["file1.txt", "file2.txt"], temp_dir)

        # Verify snapshot exists
        assert snapshot.snapshot_exists(snapshot_id)

    def test_restore_multiple_files(self, snapshot, temp_dir):
        """Should restore multiple files from snapshot."""
        # Create test files
        file1 = Path(temp_dir) / "file1.txt"
        file2 = Path(temp_dir) / "file2.txt"
        file1.write_text("original1")
        file2.write_text("original2")

        # Create snapshot
        snapshot_id = snapshot.create_snapshot(["file1.txt", "file2.txt"], temp_dir)

        # Modify files
        file1.write_text("modified1")
        file2.write_text("modified2")

        # Restore
        snapshot.restore_snapshot(snapshot_id)

        # Verify both restored
        assert file1.read_text() == "original1"
        assert file2.read_text() == "original2"

    def test_snapshot_nested_directory(self, snapshot, temp_dir):
        """Should snapshot files in nested directories."""
        # Create nested structure
        nested_dir = Path(temp_dir) / "nested" / "deep"
        nested_dir.mkdir(parents=True)
        nested_file = nested_dir / "file.txt"
        nested_file.write_text("nested content")

        # Create snapshot
        snapshot_id = snapshot.create_snapshot(["nested"], temp_dir)

        # Modify file
        nested_file.write_text("modified")

        # Restore
        snapshot.restore_snapshot(snapshot_id)

        # Verify restored
        assert nested_file.read_text() == "nested content"

    def test_raises_error_if_path_not_found(self, snapshot, temp_dir):
        """Should raise FileNotFoundError if path doesn't exist."""
        with pytest.raises(FileNotFoundError, match="nonexistent.txt"):
            snapshot.create_snapshot(["nonexistent.txt"], temp_dir)

    def test_delete_snapshot(self, snapshot, temp_dir):
        """Should delete snapshot."""
        # Create test file and snapshot
        test_file = Path(temp_dir) / "test.txt"
        test_file.write_text("content")

        snapshot_id = snapshot.create_snapshot(["test.txt"], temp_dir)

        assert snapshot.snapshot_exists(snapshot_id)

        # Delete
        snapshot.delete_snapshot(snapshot_id)

        assert not snapshot.snapshot_exists(snapshot_id)

    def test_delete_nonexistent_snapshot_raises_error(self, snapshot, temp_dir):
        """Should raise ValueError when deleting nonexistent snapshot."""
        with pytest.raises(ValueError, match="Snapshot not found"):
            snapshot.delete_snapshot("nonexistent")

    def test_list_snapshots_returns_newest_first(self, snapshot, temp_dir):
        """Should list snapshots with newest first."""
        # Create test file
        test_file = Path(temp_dir) / "test.txt"
        test_file.write_text("content")

        # Create multiple snapshots
        id1 = snapshot.create_snapshot(["test.txt"], temp_dir)
        id2 = snapshot.create_snapshot(["test.txt"], temp_dir)

        snapshots = snapshot.list_snapshots()

        assert len(snapshots) == 2
        # Newer snapshot should come first
        assert snapshots[0] == id2
        assert snapshots[1] == id1

    def test_list_snapshots_returns_empty_if_none_exist(self, snapshot, temp_dir):
        """Should return empty list if no snapshots exist."""
        snapshots = snapshot.list_snapshots()

        assert snapshots == []

    def test_snapshot_preserves_file_metadata(self, snapshot, temp_dir):
        """Should preserve file modification time."""
        # Create test file
        test_file = Path(temp_dir) / "test.txt"
        test_file.write_text("content")
        original_mtime = test_file.stat().st_mtime

        # Create snapshot and restore
        snapshot_id = snapshot.create_snapshot(["test.txt"], temp_dir)
        snapshot.restore_snapshot(snapshot_id)

        # Verify mtime preserved (within 1 second tolerance)
        restored_mtime = test_file.stat().st_mtime
        assert abs(restored_mtime - original_mtime) < 1.0
