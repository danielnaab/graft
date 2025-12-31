"""Git operations adapter using subprocess.

Real git operations using git CLI via subprocess.
"""

import subprocess
from pathlib import Path

from graft.domain.exceptions import (
    DependencyResolutionError,
    GitAuthenticationError,
    GitCloneError,
    GitFetchError,
    GitNotFoundError,
)


class SubprocessGitOperations:
    """Git operations using subprocess.

    Implements GitOperations protocol.
    Uses git CLI via subprocess for reliability.

    Example:
        >>> git = SubprocessGitOperations()
        >>> git.clone("https://github.com/user/repo.git", "/tmp/repo", "main")
    """

    def clone(
        self,
        url: str,
        destination: str,
        ref: str,
    ) -> None:
        """Clone git repository using subprocess.

        Uses shallow clone (--depth 1) for efficiency.

        Args:
            url: Git repository URL
            destination: Local path to clone into
            ref: Git reference to checkout

        Raises:
            GitCloneError: If clone fails
            GitAuthenticationError: If authentication fails
            GitNotFoundError: If repository or ref not found
        """
        try:
            # Clone with specific ref (shallow clone for efficiency)
            cmd = [
                "git",
                "clone",
                "--branch",
                ref,
                "--depth",
                "1",
                url,
                destination,
            ]

            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                check=False,
            )

            if result.returncode != 0:
                dep_name = Path(destination).name
                stderr = result.stderr.strip()

                # Detect specific error types from stderr
                if "Permission denied" in stderr or "publickey" in stderr:
                    raise GitAuthenticationError(
                        dependency_name=dep_name,
                        url=url,
                        suggestion="Configure SSH keys: ssh-keygen -t ed25519",
                    )
                elif "not found" in stderr.lower() or "does not exist" in stderr.lower():
                    raise GitNotFoundError(
                        dependency_name=dep_name,
                        url=url,
                        ref=ref,
                    )
                else:
                    raise GitCloneError(
                        dependency_name=dep_name,
                        url=url,
                        ref=ref,
                        stderr=stderr,
                        returncode=result.returncode,
                    )

        except subprocess.SubprocessError as e:
            raise GitCloneError(
                dependency_name=Path(destination).name,
                url=url,
                ref=ref,
                stderr=f"Subprocess error: {e}",
                returncode=1,
            ) from e

    def fetch(
        self,
        repo_path: str,
        ref: str,
    ) -> None:
        """Fetch and checkout ref in existing repository.

        Args:
            repo_path: Path to existing git repository
            ref: Git reference to checkout

        Raises:
            GitFetchError: If fetch/checkout fails
            GitAuthenticationError: If authentication fails
            GitNotFoundError: If ref not found
        """
        try:
            # Fetch ref
            fetch_cmd = ["git", "-C", repo_path, "fetch", "origin", ref]
            result = subprocess.run(fetch_cmd, capture_output=True, text=True, check=False)

            dep_name = Path(repo_path).name

            if result.returncode != 0:
                stderr = result.stderr.strip()

                # Detect authentication errors
                if "Permission denied" in stderr or "publickey" in stderr:
                    raise GitAuthenticationError(
                        dependency_name=dep_name,
                        url=repo_path,  # Best we have for existing repo
                        suggestion="Check SSH keys and repository access",
                    )
                else:
                    raise GitFetchError(
                        dependency_name=dep_name,
                        repo_path=repo_path,
                        ref=ref,
                        stderr=stderr,
                        returncode=result.returncode,
                    )

            # Checkout ref
            checkout_cmd = ["git", "-C", repo_path, "checkout", ref]
            result = subprocess.run(checkout_cmd, capture_output=True, text=True, check=False)

            if result.returncode != 0:
                stderr = result.stderr.strip()

                # Detect ref not found
                if "did not match" in stderr or "unknown revision" in stderr:
                    raise GitNotFoundError(
                        dependency_name=dep_name,
                        url=repo_path,
                        ref=ref,
                    )
                else:
                    raise GitFetchError(
                        dependency_name=dep_name,
                        repo_path=repo_path,
                        ref=ref,
                        stderr=f"Checkout failed: {stderr}",
                        returncode=result.returncode,
                    )

        except subprocess.SubprocessError as e:
            raise GitFetchError(
                dependency_name=Path(repo_path).name,
                repo_path=repo_path,
                ref=ref,
                stderr=f"Subprocess error: {e}",
                returncode=1,
            ) from e

    def is_repository(self, path: str) -> bool:
        """Check if path is a git repository.

        Checks for presence of .git directory.

        Args:
            path: Path to check

        Returns:
            True if path is a git repository
        """
        git_dir = Path(path) / ".git"
        return git_dir.exists() and git_dir.is_dir()
