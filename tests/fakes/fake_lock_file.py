"""Fake lock file implementation for testing."""

from graft.domain.lock_entry import LockEntry


class FakeLockFile:
    """In-memory lock file implementation for testing.

    Stores lock files in memory for fast, isolated testing.
    """

    def __init__(self) -> None:
        """Initialize fake lock file storage."""
        self._files: dict[str, dict[str, LockEntry]] = {}

    def read_lock_file(self, path: str) -> dict[str, LockEntry]:
        """Read lock file from memory.

        Args:
            path: Path to lock file

        Returns:
            Dictionary mapping dependency name to LockEntry

        Raises:
            FileNotFoundError: If lock file doesn't exist
        """
        if path not in self._files:
            raise FileNotFoundError(f"Lock file not found: {path}")

        return self._files[path].copy()

    def write_lock_file(self, path: str, entries: dict[str, LockEntry]) -> None:
        """Write lock file to memory.

        Args:
            path: Path to lock file
            entries: Dictionary mapping dependency name to LockEntry
        """
        self._files[path] = entries.copy()

    def update_lock_entry(
        self, path: str, dep_name: str, entry: LockEntry
    ) -> None:
        """Update a single dependency entry.

        Args:
            path: Path to lock file
            dep_name: Name of dependency
            entry: New LockEntry

        Raises:
            FileNotFoundError: If lock file doesn't exist
        """
        if path not in self._files:
            raise FileNotFoundError(f"Lock file not found: {path}")

        self._files[path][dep_name] = entry

    def lock_file_exists(self, path: str) -> bool:
        """Check if lock file exists.

        Args:
            path: Path to check

        Returns:
            True if lock file exists in memory
        """
        return path in self._files

    def clear(self) -> None:
        """Clear all lock files from memory."""
        self._files.clear()
