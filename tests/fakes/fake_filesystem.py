"""Fake filesystem for testing.

In-memory filesystem implementation with test helpers.
"""

from pathlib import Path


class FakeFileSystem:
    """Fake filesystem for testing.

    Implements FileSystem protocol with in-memory storage.
    Provides test helpers for setup and verification.

    Advantages over mocks:
    - Real behavior (not just assertions)
    - Can inspect state
    - Reusable across tests
    - Clear and maintainable

    Example:
        >>> fs = FakeFileSystem()
        >>> fs.create_file("/fake/graft.yaml", "apiVersion: graft/v0...")
        >>> fs.read_text("/fake/graft.yaml")
        'apiVersion: graft/v0...'
        >>> fs.exists("/fake/graft.yaml")
        True
    """

    def __init__(self) -> None:
        """Initialize empty fake filesystem."""
        self._files: dict[str, str] = {}  # path -> content
        self._dirs: set[str] = set()
        self._cwd: str = "/fake/cwd"

    def read_text(self, path: str) -> str:
        """Read text file.

        Args:
            path: File path

        Returns:
            File contents

        Raises:
            FileNotFoundError: If file doesn't exist
        """
        if path not in self._files:
            raise FileNotFoundError(f"File not found: {path}")
        return self._files[path]

    def exists(self, path: str) -> bool:
        """Check if path exists.

        Args:
            path: Path to check

        Returns:
            True if path exists (as file or directory)
        """
        return path in self._files or path in self._dirs

    def is_dir(self, path: str) -> bool:
        """Check if path is a directory.

        Args:
            path: Path to check

        Returns:
            True if path is a directory
        """
        return path in self._dirs

    def mkdir(self, path: str, parents: bool = False) -> None:
        """Create directory.

        Args:
            path: Directory path
            parents: Create parent directories if needed

        Raises:
            FileExistsError: If directory exists and parents=False
        """
        if parents:
            # Create all parent directories
            parts = Path(path).parts
            for i in range(1, len(parts) + 1):
                dir_path = str(Path(*parts[:i]))
                self._dirs.add(dir_path)
        else:
            if path in self._dirs:
                raise FileExistsError(f"Directory already exists: {path}")
            self._dirs.add(path)

    def get_cwd(self) -> str:
        """Get current working directory.

        Returns:
            Current working directory path
        """
        return self._cwd

    # Test helpers below

    def create_file(self, path: str, content: str) -> None:
        """Create file with content (test helper).

        Args:
            path: File path
            content: File content
        """
        self._files[path] = content

    def write_text(self, path: str, content: str) -> None:
        """Write text to file (protocol method).

        Args:
            path: File path
            content: File content
        """
        self._files[path] = content

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
        if not self.exists(path):
            raise FileNotFoundError(f"Directory not found: {path}")
        if not self.is_dir(path):
            raise NotADirectoryError(f"Not a directory: {path}")

        # Find all items that are direct children of this path
        items = []
        path_with_slash = path.rstrip("/") + "/"

        # Check files
        for file_path in self._files:
            if file_path.startswith(path_with_slash):
                # Get relative path from directory
                relative = file_path[len(path_with_slash):]
                # Only include direct children (no slashes in relative path)
                if "/" not in relative:
                    items.append(relative)

        # Check directories
        for dir_path in self._dirs:
            if dir_path.startswith(path_with_slash):
                # Get relative path from directory
                relative = dir_path[len(path_with_slash):]
                # Only include direct children
                if relative and "/" not in relative:
                    items.append(relative)

        return sorted(set(items))

    def remove_directory(self, path: str) -> None:
        """Remove a directory and all its contents.

        Args:
            path: Directory path to remove

        Raises:
            FileNotFoundError: If directory doesn't exist
            NotADirectoryError: If path is not a directory
        """
        if not self.exists(path):
            raise FileNotFoundError(f"Directory not found: {path}")
        if not self.is_dir(path):
            raise NotADirectoryError(f"Not a directory: {path}")

        # Remove the directory
        self._dirs.discard(path)

        # Remove all files in the directory
        path_with_slash = path.rstrip("/") + "/"
        files_to_remove = [f for f in self._files if f.startswith(path_with_slash)]
        for f in files_to_remove:
            del self._files[f]

        # Remove all subdirectories
        dirs_to_remove = [d for d in self._dirs if d.startswith(path_with_slash)]
        for d in dirs_to_remove:
            self._dirs.discard(d)

    def set_cwd(self, path: str) -> None:
        """Set current working directory (test helper).

        Args:
            path: Path to set as cwd
        """
        self._cwd = path

    def reset(self) -> None:
        """Reset all state (test helper).

        Useful for cleanup between tests.
        """
        self._files.clear()
        self._dirs.clear()
        self._cwd = "/fake/cwd"
