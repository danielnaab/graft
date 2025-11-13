"""Git adapter for Graft."""
from __future__ import annotations
import subprocess
import hashlib
from pathlib import Path
from typing import Protocol, Optional


class GitPort(Protocol):
    """Port for git operations."""

    def get_file_hash_at_rev(self, file_path: Path, rev: str, repo_root: Path) -> Optional[str]:
        """
        Get the git object hash for a file at a specific revision.

        Args:
            file_path: Path to the file (absolute or relative to repo_root)
            rev: Git revision (e.g., 'HEAD', commit hash, branch name)
            repo_root: Root of the git repository

        Returns:
            Git object hash (SHA-1) if file exists at that revision, None otherwise
        """
        ...

    def get_working_file_hash(self, file_path: Path) -> Optional[str]:
        """
        Get the hash of a file in the working directory.

        Args:
            file_path: Path to the file

        Returns:
            Hash of the file content (same algorithm as git), None if file doesn't exist
        """
        ...

    def is_file_modified(self, file_path: Path, rev: str, repo_root: Path) -> bool:
        """
        Check if a file has been modified compared to a specific revision.

        Args:
            file_path: Path to the file (absolute or relative to repo_root)
            rev: Git revision to compare against
            repo_root: Root of the git repository

        Returns:
            True if file is modified, False if unchanged
        """
        ...

    def get_repo_root(self, path: Path) -> Optional[Path]:
        """
        Get the root directory of the git repository containing the given path.

        Args:
            path: Any path within a git repository

        Returns:
            Path to repository root, or None if not in a git repo
        """
        ...


class GitAdapter:
    """Git operations implementation."""

    def get_file_hash_at_rev(self, file_path: Path, rev: str, repo_root: Path) -> Optional[str]:
        """Get the git object hash for a file at a specific revision."""
        try:
            # Make path relative to repo root if it's absolute
            if file_path.is_absolute():
                try:
                    rel_path = file_path.relative_to(repo_root)
                except ValueError:
                    # File is not within repo_root
                    return None
            else:
                rel_path = file_path

            # Use git ls-tree to get the object hash at the specified revision
            result = subprocess.run(
                ["git", "ls-tree", rev, str(rel_path)],
                cwd=repo_root,
                capture_output=True,
                text=True,
                check=False
            )

            if result.returncode != 0 or not result.stdout.strip():
                return None

            # Parse output: "mode type hash\tpath"
            # Example: "100644 blob abc123def456...\tfile.txt"
            parts = result.stdout.strip().split()
            if len(parts) >= 3:
                return parts[2]  # The hash

            return None

        except Exception:
            return None

    def get_working_file_hash(self, file_path: Path) -> Optional[str]:
        """Get the hash of a file in the working directory using git's algorithm."""
        try:
            if not file_path.exists():
                return None

            # Use git hash-object to compute the hash the same way git does
            result = subprocess.run(
                ["git", "hash-object", str(file_path)],
                capture_output=True,
                text=True,
                check=False
            )

            if result.returncode != 0:
                return None

            return result.stdout.strip()

        except Exception:
            return None

    def is_file_modified(self, file_path: Path, rev: str, repo_root: Path) -> bool:
        """Check if a file has been modified compared to a specific revision."""
        hash_at_rev = self.get_file_hash_at_rev(file_path, rev, repo_root)
        if hash_at_rev is None:
            # File doesn't exist at that revision - consider it modified
            return True

        working_hash = self.get_working_file_hash(file_path)
        if working_hash is None:
            # File doesn't exist in working directory - consider it modified
            return True

        return hash_at_rev != working_hash

    def get_repo_root(self, path: Path) -> Optional[Path]:
        """Get the root directory of the git repository."""
        try:
            # Start from the given path if it's a directory, otherwise its parent
            search_path = path if path.is_dir() else path.parent

            result = subprocess.run(
                ["git", "rev-parse", "--show-toplevel"],
                cwd=search_path,
                capture_output=True,
                text=True,
                check=False
            )

            if result.returncode != 0:
                return None

            return Path(result.stdout.strip())

        except Exception:
            return None
