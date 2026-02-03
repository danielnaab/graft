"""Git .gitmodules file management.

Handles reading and writing of .gitmodules file for submodule configuration.
"""

import configparser
from pathlib import Path
from typing import Protocol

from graft.domain.gitmodule import GitModuleEntry


class Filesystem(Protocol):
    """Filesystem operations protocol (subset used here)."""

    def read_text(self, path: str) -> str:
        """Read text file contents."""
        ...

    def write_text(self, path: str, content: str) -> None:
        """Write text to file."""
        ...

    def exists(self, path: str) -> bool:
        """Check if path exists."""
        ...


class GitModulesFile:
    """Manages .gitmodules file for submodule configuration.

    Provides high-level interface for reading and writing .gitmodules entries.
    Uses configparser for INI-style format handling.

    Attributes:
        filesystem: Filesystem operations implementation
        path: Path to .gitmodules file (default: ".gitmodules")

    Example:
        >>> gitmodules = GitModulesFile(filesystem)
        >>> entries = gitmodules.read_gitmodules()
        >>> gitmodules.add_entry("my-lib", ".graft/my-lib", "https://...", "main")
    """

    def __init__(self, filesystem: Filesystem, path: str = ".gitmodules"):
        """Initialize GitModulesFile.

        Args:
            filesystem: Filesystem operations implementation
            path: Path to .gitmodules file (default: ".gitmodules")
        """
        self.filesystem = filesystem
        self.path = path

    def read_gitmodules(self) -> dict[str, GitModuleEntry]:
        """Read and parse .gitmodules file.

        Returns:
            Dictionary mapping submodule names to GitModuleEntry objects

        Example:
            >>> entries = gitmodules.read_gitmodules()
            >>> entry = entries["my-lib"]
            >>> print(entry.url)
        """
        if not self.filesystem.exists(self.path):
            return {}

        content = self.filesystem.read_text(self.path)
        parser = configparser.ConfigParser()
        parser.read_string(content)

        entries = {}
        for section in parser.sections():
            # Section format: 'submodule "name"'
            if section.startswith('submodule "') and section.endswith('"'):
                name = section[11:-1]  # Extract name from quotes

                # Get required fields
                if "path" not in parser[section] or "url" not in parser[section]:
                    continue  # Skip invalid entries

                entry = GitModuleEntry(
                    name=name,
                    path=parser[section]["path"],
                    url=parser[section]["url"],
                    branch=parser[section].get("branch"),
                )
                entries[name] = entry

        return entries

    def write_gitmodules(self, entries: dict[str, GitModuleEntry]) -> None:
        """Write .gitmodules file with given entries.

        Overwrites existing file completely.

        Args:
            entries: Dictionary mapping submodule names to GitModuleEntry objects

        Example:
            >>> entries = {"my-lib": GitModuleEntry(...)}
            >>> gitmodules.write_gitmodules(entries)
        """
        parser = configparser.ConfigParser()

        for name, entry in entries.items():
            section = f'submodule "{name}"'
            parser[section] = {
                "path": entry.path,
                "url": entry.url,
            }
            if entry.branch:
                parser[section]["branch"] = entry.branch

        # Write to string first to maintain formatting
        import io
        output = io.StringIO()
        parser.write(output)
        content = output.getvalue()

        self.filesystem.write_text(self.path, content)

    def add_entry(self, name: str, path: str, url: str, branch: str | None = None) -> None:
        """Add or update a submodule entry.

        Reads existing entries, adds/updates the specified entry, and writes back.

        Args:
            name: Submodule name
            path: Path where submodule is located
            url: Git repository URL
            branch: Optional branch to track

        Example:
            >>> gitmodules.add_entry("my-lib", ".graft/my-lib", "https://...", "main")
        """
        entries = self.read_gitmodules()
        entries[name] = GitModuleEntry(name=name, path=path, url=url, branch=branch)
        self.write_gitmodules(entries)

    def remove_entry(self, name: str) -> None:
        """Remove a submodule entry.

        Args:
            name: Name of submodule to remove

        Example:
            >>> gitmodules.remove_entry("my-lib")
        """
        entries = self.read_gitmodules()
        if name in entries:
            del entries[name]
            self.write_gitmodules(entries)

    def update_entry(self, name: str, **kwargs) -> None:
        """Update specific fields of a submodule entry.

        Args:
            name: Name of submodule to update
            **kwargs: Fields to update (path, url, branch)

        Raises:
            KeyError: If submodule doesn't exist

        Example:
            >>> gitmodules.update_entry("my-lib", branch="develop")
        """
        entries = self.read_gitmodules()
        if name not in entries:
            raise KeyError(f"Submodule '{name}' not found in .gitmodules")

        entry = entries[name]
        # Create new entry with updated fields
        updated = GitModuleEntry(
            name=name,
            path=kwargs.get("path", entry.path),
            url=kwargs.get("url", entry.url),
            branch=kwargs.get("branch", entry.branch),
        )
        entries[name] = updated
        self.write_gitmodules(entries)

    def entry_exists(self, name: str) -> bool:
        """Check if a submodule entry exists.

        Args:
            name: Name of submodule to check

        Returns:
            True if submodule entry exists in .gitmodules

        Example:
            >>> if gitmodules.entry_exists("my-lib"):
            ...     print("Submodule already configured")
        """
        entries = self.read_gitmodules()
        return name in entries