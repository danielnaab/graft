"""Real filesystem adapter.

Filesystem operations using pathlib.Path.
"""

import os
from pathlib import Path


class RealFileSystem:
    """Real filesystem operations.

    Implements FileSystem protocol.
    Wraps standard library path operations.

    Example:
        >>> fs = RealFileSystem()
        >>> fs.exists("graft.yaml")
        True
        >>> content = fs.read_text("graft.yaml")
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
        return Path(path).read_text()

    def exists(self, path: str) -> bool:
        """Check if path exists.

        Args:
            path: Path to check

        Returns:
            True if path exists
        """
        return Path(path).exists()

    def is_dir(self, path: str) -> bool:
        """Check if path is a directory.

        Args:
            path: Path to check

        Returns:
            True if path is a directory
        """
        return Path(path).is_dir()

    def mkdir(self, path: str, parents: bool = False) -> None:
        """Create directory.

        Args:
            path: Directory path
            parents: Create parent directories if needed

        Raises:
            FileExistsError: If directory exists and parents=False
        """
        Path(path).mkdir(parents=parents, exist_ok=parents)

    def get_cwd(self) -> str:
        """Get current working directory.

        Returns:
            Absolute path to current directory
        """
        return os.getcwd()
