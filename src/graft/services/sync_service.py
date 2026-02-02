"""Sync service for dependency synchronization.

Service functions for syncing .graft/ to match lock file state.
"""

from dataclasses import dataclass
from pathlib import Path

from graft.domain.lock_entry import LockEntry
from graft.protocols.filesystem import FileSystem
from graft.protocols.git import GitOperations


@dataclass
class SyncResult:
    """Result of syncing a single dependency.

    Attributes:
        name: Dependency name
        success: Whether sync succeeded
        action: Action taken ('cloned', 'checked_out', 'up_to_date', 'failed')
        message: Human-readable message
    """

    name: str
    success: bool
    action: str
    message: str


def sync_dependency(
    filesystem: FileSystem,
    git: GitOperations,
    deps_directory: str,
    name: str,
    entry: LockEntry,
) -> SyncResult:
    """Sync a single dependency to match lock file state.

    Clones if missing, or checks out the locked commit if present.

    Args:
        filesystem: Filesystem operations
        git: Git operations
        deps_directory: Path to deps directory (e.g., ".graft")
        name: Dependency name
        entry: Lock entry with expected state

    Returns:
        SyncResult indicating outcome
    """
    local_path = str(Path(deps_directory) / name)

    try:
        if not filesystem.exists(local_path):
            # Clone the repository
            git.clone(
                url=entry.source,
                destination=local_path,
                ref=entry.ref,
            )
            # Checkout the exact commit
            git.checkout(local_path, entry.commit)
            return SyncResult(
                name=name,
                success=True,
                action="cloned",
                message=f"Cloned and checked out {entry.commit[:7]}",
            )

        # Path exists - check if it's a git repo
        if not git.is_repository(local_path):
            return SyncResult(
                name=name,
                success=False,
                action="failed",
                message=f"Path exists but is not a git repository: {local_path}",
            )

        # Get current commit
        try:
            current_commit = git.get_current_commit(local_path)
        except Exception:
            current_commit = None

        if current_commit == entry.commit:
            return SyncResult(
                name=name,
                success=True,
                action="up_to_date",
                message=f"Already at {entry.commit[:7]}",
            )

        # Need to fetch and checkout
        git.fetch(local_path, entry.ref)
        git.checkout(local_path, entry.commit)
        return SyncResult(
            name=name,
            success=True,
            action="checked_out",
            message=f"Checked out {entry.commit[:7]}",
        )

    except Exception as e:
        return SyncResult(
            name=name,
            success=False,
            action="failed",
            message=str(e),
        )


def sync_all_dependencies(
    filesystem: FileSystem,
    git: GitOperations,
    deps_directory: str,
    lock_entries: dict[str, LockEntry],
) -> list[SyncResult]:
    """Sync all dependencies to match lock file state.

    Args:
        filesystem: Filesystem operations
        git: Git operations
        deps_directory: Path to deps directory
        lock_entries: Dictionary of lock entries

    Returns:
        List of sync results
    """
    results = []

    for name, entry in sorted(lock_entries.items()):
        result = sync_dependency(filesystem, git, deps_directory, name, entry)
        results.append(result)

    return results
