"""File system adapter for Graft."""
from __future__ import annotations
from pathlib import Path
from typing import Protocol


class FileSystemPort(Protocol):
    """Port for file system operations."""

    def read_text(self, path: Path) -> str:
        """Read text from a file."""
        ...

    def read_bytes(self, path: Path) -> bytes:
        """Read bytes from a file."""
        ...

    def write_text(self, path: Path, content: str) -> None:
        """Write text to a file."""
        ...

    def write_bytes(self, path: Path, content: bytes) -> None:
        """Write bytes to a file."""
        ...

    def exists(self, path: Path) -> bool:
        """Check if a path exists."""
        ...

    def mkdir(self, path: Path, parents: bool = False, exist_ok: bool = False) -> None:
        """Create a directory."""
        ...


class LocalFileSystem:
    """Local file system implementation."""

    def read_text(self, path: Path) -> str:
        """Read text from a file."""
        return path.read_text(encoding="utf-8")

    def read_bytes(self, path: Path) -> bytes:
        """Read bytes from a file."""
        return path.read_bytes()

    def write_text(self, path: Path, content: str) -> None:
        """Write text to a file."""
        path.write_text(content, encoding="utf-8")

    def write_bytes(self, path: Path, content: bytes) -> None:
        """Write bytes to a file."""
        path.write_bytes(content)

    def exists(self, path: Path) -> bool:
        """Check if a path exists."""
        return path.exists()

    def mkdir(self, path: Path, parents: bool = False, exist_ok: bool = False) -> None:
        """Create a directory."""
        path.mkdir(parents=parents, exist_ok=exist_ok)
