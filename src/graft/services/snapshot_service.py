"""Snapshot service functions.

Service layer for snapshot and rollback operations.
Follows functional service pattern with dependency injection.
"""

from graft.protocols.snapshot import Snapshot


def create_workspace_snapshot(
    snapshot: Snapshot,
    paths: list[str],
    base_dir: str,
) -> str:
    """Create snapshot of workspace files.

    Args:
        snapshot: Snapshot protocol implementation
        paths: List of file/directory paths to snapshot
        base_dir: Base directory to resolve paths from

    Returns:
        Snapshot ID for later restoration

    Raises:
        OSError: If unable to create snapshot
        FileNotFoundError: If any path doesn't exist
    """
    return snapshot.create_snapshot(paths, base_dir)


def restore_workspace_snapshot(
    snapshot: Snapshot,
    snapshot_id: str,
) -> None:
    """Restore workspace from snapshot.

    Args:
        snapshot: Snapshot protocol implementation
        snapshot_id: ID returned from create_workspace_snapshot()

    Raises:
        ValueError: If snapshot_id doesn't exist
        OSError: If unable to restore snapshot
    """
    snapshot.restore_snapshot(snapshot_id)


def cleanup_snapshot(
    snapshot: Snapshot,
    snapshot_id: str,
) -> None:
    """Delete snapshot to free disk space.

    Args:
        snapshot: Snapshot protocol implementation
        snapshot_id: Snapshot ID to delete

    Raises:
        ValueError: If snapshot_id doesn't exist
    """
    snapshot.delete_snapshot(snapshot_id)


def cleanup_old_snapshots(
    snapshot: Snapshot,
    keep_count: int = 5,
) -> list[str]:
    """Delete old snapshots, keeping only the most recent.

    Args:
        snapshot: Snapshot protocol implementation
        keep_count: Number of recent snapshots to keep

    Returns:
        List of deleted snapshot IDs
    """
    all_snapshots = snapshot.list_snapshots()

    # Snapshots are already sorted newest first
    to_delete = all_snapshots[keep_count:]

    deleted = []
    for snapshot_id in to_delete:
        try:
            snapshot.delete_snapshot(snapshot_id)
            deleted.append(snapshot_id)
        except ValueError:
            # Snapshot might have been deleted by another process
            pass

    return deleted


def get_snapshot_paths_for_dependency(
    dep_name: str,
) -> list[str]:
    """Get typical paths to snapshot for a dependency upgrade.

    Args:
        dep_name: Name of dependency being upgraded

    Returns:
        List of paths that should be snapshotted
    """
    # Standard paths that might be modified during upgrade
    return [
        f".graft/deps/{dep_name}",  # Dependency files
        "graft.lock",  # Lock file
    ]
