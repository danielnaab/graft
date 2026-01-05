"""Filesystem snapshot adapter implementation.

Provides filesystem-based snapshot/rollback using file copying.
"""

import shutil
from datetime import UTC, datetime
from pathlib import Path


class FilesystemSnapshot:
    """Filesystem-based snapshot implementation.

    Creates snapshots by copying files to a .graft/snapshots directory.
    Snapshots are identified by timestamp-based IDs.
    """

    def __init__(self, snapshot_dir: str = ".graft/snapshots") -> None:
        """Initialize filesystem snapshot adapter.

        Args:
            snapshot_dir: Directory to store snapshots (relative to project root)
        """
        self._snapshot_dir = snapshot_dir

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
        # Generate snapshot ID using timestamp
        timestamp = datetime.now(UTC).timestamp()
        snapshot_id = f"snapshot-{int(timestamp * 1000000)}"

        # Create snapshot directory
        base_path = Path(base_dir)
        snapshot_path = base_path / self._snapshot_dir / snapshot_id
        snapshot_path.mkdir(parents=True, exist_ok=True)

        # Copy each path to snapshot
        for path_str in paths:
            source = base_path / path_str

            if not source.exists():
                # Clean up partial snapshot
                shutil.rmtree(snapshot_path, ignore_errors=True)
                raise FileNotFoundError(f"Path not found: {path_str}")

            # Determine destination path (preserve structure)
            dest = snapshot_path / path_str

            try:
                if source.is_dir():
                    # Copy directory recursively
                    dest.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copytree(source, dest, symlinks=True)
                else:
                    # Copy file
                    dest.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copy2(source, dest)
            except Exception as e:
                # Clean up partial snapshot
                shutil.rmtree(snapshot_path, ignore_errors=True)
                raise OSError(f"Failed to snapshot {path_str}: {e}") from e

        return snapshot_id

    def restore_snapshot(self, snapshot_id: str) -> None:
        """Restore files from snapshot.

        Args:
            snapshot_id: ID returned from create_snapshot()

        Raises:
            ValueError: If snapshot_id doesn't exist
            OSError: If unable to restore snapshot
        """
        # Find snapshot directory
        snapshot_path = self._get_snapshot_path(snapshot_id)

        if not snapshot_path.exists():
            raise ValueError(f"Snapshot not found: {snapshot_id}")

        # Get base directory (parent of snapshot_dir)
        base_path = snapshot_path.parent.parent.parent

        # Restore each file/directory from snapshot
        try:
            for item in snapshot_path.rglob("*"):
                if item.is_dir():
                    continue

                # Calculate relative path
                rel_path = item.relative_to(snapshot_path)
                dest = base_path / rel_path

                # Restore file
                dest.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(item, dest)
        except Exception as e:
            raise OSError(f"Failed to restore snapshot {snapshot_id}: {e}") from e

    def delete_snapshot(self, snapshot_id: str) -> None:
        """Delete a snapshot to free disk space.

        Args:
            snapshot_id: ID returned from create_snapshot()

        Raises:
            ValueError: If snapshot_id doesn't exist
        """
        snapshot_path = self._get_snapshot_path(snapshot_id)

        if not snapshot_path.exists():
            raise ValueError(f"Snapshot not found: {snapshot_id}")

        shutil.rmtree(snapshot_path)

    def snapshot_exists(self, snapshot_id: str) -> bool:
        """Check if snapshot exists.

        Args:
            snapshot_id: ID to check

        Returns:
            True if snapshot exists
        """
        snapshot_path = self._get_snapshot_path(snapshot_id)
        return snapshot_path.exists()

    def list_snapshots(self) -> list[str]:
        """List all snapshot IDs.

        Returns:
            List of snapshot IDs, newest first
        """
        # Find all snapshot directories
        snapshots_root = Path(self._snapshot_dir)

        if not snapshots_root.exists():
            return []

        # Get all snapshot-* directories
        snapshot_dirs = sorted(
            [d.name for d in snapshots_root.iterdir() if d.is_dir() and d.name.startswith("snapshot-")],
            reverse=True,  # Newest first
        )

        return snapshot_dirs

    def _get_snapshot_path(self, snapshot_id: str) -> Path:
        """Get path to snapshot directory.

        Args:
            snapshot_id: Snapshot ID

        Returns:
            Path to snapshot directory
        """
        return Path(self._snapshot_dir) / snapshot_id
