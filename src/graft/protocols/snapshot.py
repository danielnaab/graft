"""Snapshot protocol.

Protocol for snapshot and rollback operations to enable atomic upgrades.
"""

from typing import Protocol


class Snapshot(Protocol):
    """Protocol for snapshot and rollback operations.

    Enables creating filesystem snapshots before mutations and
    restoring them on failure to ensure atomic operations.
    """

    def create_snapshot(self, paths: list[str], base_dir: str) -> str:
        """Create snapshot of specified paths.

        Args:
            paths: List of file/directory paths to snapshot (relative to base_dir)
            base_dir: Base directory to resolve paths from

        Returns:
            Snapshot ID for later restoration

        Raises:
            OSError: If unable to create snapshot
            FileNotFoundError: If any path doesn't exist
        """
        ...

    def restore_snapshot(self, snapshot_id: str) -> None:
        """Restore files from snapshot.

        Args:
            snapshot_id: ID returned from create_snapshot()

        Raises:
            ValueError: If snapshot_id doesn't exist
            OSError: If unable to restore snapshot
        """
        ...

    def delete_snapshot(self, snapshot_id: str) -> None:
        """Delete a snapshot to free disk space.

        Args:
            snapshot_id: ID returned from create_snapshot()

        Raises:
            ValueError: If snapshot_id doesn't exist
        """
        ...

    def snapshot_exists(self, snapshot_id: str) -> bool:
        """Check if snapshot exists.

        Args:
            snapshot_id: ID to check

        Returns:
            True if snapshot exists
        """
        ...

    def list_snapshots(self) -> list[str]:
        """List all snapshot IDs.

        Returns:
            List of snapshot IDs, newest first
        """
        ...
