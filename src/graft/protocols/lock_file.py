"""Lock file protocol.

Protocol for lock file operations following the specification.
"""

from typing import Protocol

from graft.domain.lock_entry import LockEntry


class LockFile(Protocol):
    """Protocol for lock file operations.

    Enables reading, writing, and updating graft.lock files
    following the specification format.
    """

    def read_lock_file(self, path: str) -> dict[str, LockEntry]:
        """Read lock file and return dependency entries.

        Args:
            path: Path to graft.lock file

        Returns:
            Dictionary mapping dependency name to LockEntry

        Raises:
            FileNotFoundError: If lock file doesn't exist
            ValueError: If lock file is malformed
        """
        ...

    def write_lock_file(self, path: str, entries: dict[str, LockEntry]) -> None:
        """Write lock file with dependency entries.

        Args:
            path: Path to graft.lock file
            entries: Dictionary mapping dependency name to LockEntry

        Raises:
            IOError: If unable to write file
        """
        ...

    def update_lock_entry(
        self, path: str, dep_name: str, entry: LockEntry
    ) -> None:
        """Update a single dependency entry in lock file.

        Atomic operation that reads, updates, and writes.

        Args:
            path: Path to graft.lock file
            dep_name: Name of dependency to update
            entry: New LockEntry for the dependency

        Raises:
            FileNotFoundError: If lock file doesn't exist
            IOError: If unable to write file
        """
        ...

    def lock_file_exists(self, path: str) -> bool:
        """Check if lock file exists.

        Args:
            path: Path to check

        Returns:
            True if lock file exists
        """
        ...
