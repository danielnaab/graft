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

    def list_directory(self, path: str) -> list[str]:
        """List directory contents.

        Args:
            path: Directory path

        Returns:
            List of file/directory names (not full paths)

        Raises:
            FileNotFoundError: If directory doesn't exist
            NotADirectoryError: If path is not a directory
        """
        ...

    def write_text(self, path: str, content: str) -> None:
        """Write text to file.

        Args:
            path: File path
            content: Text content to write

        Raises:
            PermissionError: If file not writable
        """
        ...

    def remove_directory(self, path: str) -> None:
        """Remove a directory and all its contents.

        Args:
            path: Directory path to remove

        Raises:
            FileNotFoundError: If directory doesn't exist
            NotADirectoryError: If path is not a directory
        """
        ...
