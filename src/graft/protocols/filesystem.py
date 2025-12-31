"""Filesystem operations protocol.

Protocol for filesystem operations enabling testability with fakes.
"""

from typing import Protocol


class FileSystem(Protocol):
    """Protocol for filesystem operations.

    Any class implementing these methods satisfies this protocol.
    Enables testing with fake implementations.

    Example implementations:
        - RealFileSystem (uses pathlib.Path)
        - FakeFileSystem (in-memory for testing)
    """

    def read_text(self, path: str) -> str:
        """Read text file.

        Args:
            path: File path

        Returns:
            File contents

        Raises:
            FileNotFoundError: If file doesn't exist
            PermissionError: If file not readable
        """
        ...

    def exists(self, path: str) -> bool:
        """Check if path exists.

        Args:
            path: Path to check

        Returns:
            True if path exists
        """
        ...

    def is_dir(self, path: str) -> bool:
        """Check if path is a directory.

        Args:
            path: Path to check

        Returns:
            True if path is a directory
        """
        ...

    def mkdir(self, path: str, parents: bool = False) -> None:
        """Create directory.

        Args:
            path: Directory path
            parents: Create parent directories if needed

        Raises:
            FileExistsError: If directory exists and parents=False
        """
        ...

    def get_cwd(self) -> str:
        """Get current working directory.

        Returns:
            Absolute path to current directory
        """
        ...
