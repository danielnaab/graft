"""Lock file service.

Service functions for working with graft.lock files.
"""

from datetime import UTC, datetime
from pathlib import Path

from graft.domain.lock_entry import LockEntry
from graft.protocols.lock_file import LockFile


def get_lock_entry(
    lock_file: LockFile, lock_path: str, dep_name: str
) -> LockEntry | None:
    """Get lock entry for a dependency.

    Args:
        lock_file: Lock file operations protocol
        lock_path: Path to graft.lock
        dep_name: Name of dependency

    Returns:
        LockEntry if found, None if dependency not in lock file

    Raises:
        FileNotFoundError: If lock file doesn't exist
    """
    entries = lock_file.read_lock_file(lock_path)
    return entries.get(dep_name)


def update_dependency_lock(
    lock_file: LockFile,
    lock_path: str,
    dep_name: str,
    source: str,
    ref: str,
    commit: str,
) -> None:
    """Update lock file entry for a dependency.

    Creates or updates the lock entry with new version info.

    Args:
        lock_file: Lock file operations protocol
        lock_path: Path to graft.lock
        dep_name: Name of dependency
        source: Git URL or path
        ref: Git ref being consumed
        commit: Resolved commit hash

    Raises:
        IOError: If unable to write lock file
    """
    entry = LockEntry(
        source=source,
        ref=ref,
        commit=commit,
        consumed_at=datetime.now(UTC),
    )

    # If lock file doesn't exist, create it with this entry
    if not lock_file.lock_file_exists(lock_path):
        lock_file.write_lock_file(lock_path, {dep_name: entry})
    else:
        lock_file.update_lock_entry(lock_path, dep_name, entry)


def get_all_lock_entries(
    lock_file: LockFile, lock_path: str
) -> dict[str, LockEntry]:
    """Get all lock entries.

    Args:
        lock_file: Lock file operations protocol
        lock_path: Path to graft.lock

    Returns:
        Dictionary mapping dependency name to LockEntry.
        Returns empty dict if lock file doesn't exist.
    """
    if not lock_file.lock_file_exists(lock_path):
        return {}

    return lock_file.read_lock_file(lock_path)


def find_lock_file(lock_file: LockFile, directory: str) -> str | None:
    """Find graft.lock in directory.

    Args:
        lock_file: Lock file operations protocol
        directory: Directory to search

    Returns:
        Path to graft.lock if found, None otherwise
    """
    lock_path = str(Path(directory) / "graft.lock")
    if lock_file.lock_file_exists(lock_path):
        return lock_path
    return None


def create_empty_lock_file(lock_file: LockFile, lock_path: str) -> None:
    """Create an empty lock file.

    Args:
        lock_file: Lock file operations protocol
        lock_path: Path where to create graft.lock

    Raises:
        IOError: If unable to write file
    """
    lock_file.write_lock_file(lock_path, {})
