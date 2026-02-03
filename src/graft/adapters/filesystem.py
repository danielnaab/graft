"""Real filesystem adapter.

Filesystem operations using pathlib.Path.
"""

import os
import shutil
from pathlib import Path


class RealFilesystem:
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
        p = Path(path)
        if not p.exists():
            raise FileNotFoundError(f"Directory not found: {path}")
        if not p.is_dir():
            raise NotADirectoryError(f"Not a directory: {path}")
        return [item.name for item in p.iterdir()]

    def write_text(self, path: str, content: str) -> None:
        """Write text to file.

        Args:
            path: File path
            content: Text content to write

        Raises:
            PermissionError: If file not writable
        """
        Path(path).write_text(content)

    def remove_directory(self, path: str) -> None:
        """Remove a directory and all its contents.

        Args:
            path: Directory path to remove

        Raises:
            FileNotFoundError: If directory doesn't exist
            NotADirectoryError: If path is not a directory
        """
        p = Path(path)
        if not p.exists():
            raise FileNotFoundError(f"Directory not found: {path}")
        if not p.is_dir():
            raise NotADirectoryError(f"Not a directory: {path}")
        shutil.rmtree(path)


# Backwards-compatible alias
RealFileSystem = RealFilesystem
