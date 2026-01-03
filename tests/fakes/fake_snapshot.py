"""Fake snapshot implementation for testing."""

import copy
from datetime import UTC, datetime


class FakeSnapshot:
    """In-memory snapshot implementation for testing.

    Stores file snapshots as dictionaries in memory for fast, isolated testing.
    """

    def __init__(self) -> None:
        """Initialize fake snapshot storage."""
        self._snapshots: dict[str, dict[str, str]] = {}
        self._files: dict[str, str] = {}  # Current file state (path -> content)

    def create_snapshot(self, paths: list[str], base_dir: str) -> str:
        """Create snapshot of specified paths.

        Args:
            paths: List of file paths to snapshot
            base_dir: Base directory (used for snapshot ID but not path resolution)

        Returns:
            Snapshot ID

        Raises:
            FileNotFoundError: If any path doesn't exist
        """
        # Check all paths exist first
        for path in paths:
            if path not in self._files:
                raise FileNotFoundError(f"Path not found: {path}")

        # Generate snapshot ID
        timestamp = datetime.now(UTC).timestamp()
        snapshot_id = f"snapshot-{int(timestamp * 1000000)}"

        # Copy file contents for these paths
        snapshot_data = {path: self._files[path] for path in paths}
        self._snapshots[snapshot_id] = snapshot_data

        return snapshot_id

    def restore_snapshot(self, snapshot_id: str) -> None:
        """Restore files from snapshot.

        Args:
            snapshot_id: Snapshot ID

        Raises:
            ValueError: If snapshot doesn't exist
        """
        if snapshot_id not in self._snapshots:
            raise ValueError(f"Snapshot not found: {snapshot_id}")

        # Restore files from snapshot
        snapshot_data = self._snapshots[snapshot_id]
        for path, content in snapshot_data.items():
            self._files[path] = content

    def delete_snapshot(self, snapshot_id: str) -> None:
        """Delete a snapshot.

        Args:
            snapshot_id: Snapshot ID

        Raises:
            ValueError: If snapshot doesn't exist
        """
        if snapshot_id not in self._snapshots:
            raise ValueError(f"Snapshot not found: {snapshot_id}")

        del self._snapshots[snapshot_id]

    def snapshot_exists(self, snapshot_id: str) -> bool:
        """Check if snapshot exists.

        Args:
            snapshot_id: Snapshot ID

        Returns:
            True if snapshot exists
        """
        return snapshot_id in self._snapshots

    def list_snapshots(self) -> list[str]:
        """List all snapshot IDs.

        Returns:
            List of snapshot IDs, newest first
        """
        return sorted(self._snapshots.keys(), reverse=True)

    def set_file_content(self, path: str, content: str) -> None:
        """Set file content in fake filesystem.

        Helper method for testing.

        Args:
            path: File path
            content: File content
        """
        self._files[path] = content

    def get_file_content(self, path: str) -> str | None:
        """Get file content from fake filesystem.

        Helper method for testing.

        Args:
            path: File path

        Returns:
            File content or None if doesn't exist
        """
        return self._files.get(path)

    def clear(self) -> None:
        """Clear all snapshots and files."""
        self._snapshots.clear()
        self._files.clear()
