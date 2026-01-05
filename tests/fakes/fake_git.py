"""Fake git operations for testing.

In-memory git operations implementation with test helpers.
"""

from graft.domain.exceptions import DependencyResolutionError


class FakeGitOperations:
    """Fake git operations for testing.

    Implements GitOperations protocol with in-memory simulation.
    Records all operations for verification.

    Advantages over mocks:
    - Real behavior (not just assertions)
    - Can inspect state
    - Reusable across tests
    - Clear and maintainable

    Example:
        >>> git = FakeGitOperations()
        >>> git.clone("https://github.com/user/repo.git", "/tmp/repo", "main")
        >>> git.was_cloned("https://github.com/user/repo.git", "/tmp/repo", "main")
        True
    """

    def __init__(self) -> None:
        """Initialize fake git operations."""
        self._cloned_repos: dict[str, tuple[str, str]] = {}  # path -> (url, ref)
        self._clone_calls: list[tuple[str, str, str]] = []  # (url, dest, ref)
        self._fetch_calls: list[tuple[str, str]] = []  # (path, ref)
        self._should_fail: dict[str, str] = {}  # url -> error message
        self._refs: dict[tuple[str, str], str] = {}  # (repo_path, ref) -> commit hash

    def clone(self, url: str, destination: str, ref: str) -> None:
        """Fake clone operation.

        Records clone call and simulates success/failure.

        Args:
            url: Git repository URL
            destination: Local path to clone into
            ref: Git reference to checkout

        Raises:
            DependencyResolutionError: If configured to fail
        """
        self._clone_calls.append((url, destination, ref))

        # Simulate failure if configured
        if url in self._should_fail:
            raise DependencyResolutionError(
                dependency_name=destination,
                reason=self._should_fail[url],
            )

        # Record successful clone
        self._cloned_repos[destination] = (url, ref)

    def fetch(self, repo_path: str, ref: str) -> None:
        """Fake fetch operation.

        Records fetch call and updates ref.

        Args:
            repo_path: Path to existing git repository
            ref: Git reference to checkout

        Raises:
            DependencyResolutionError: If repository doesn't exist
        """
        self._fetch_calls.append((repo_path, ref))

        # Must be a known repository
        if repo_path not in self._cloned_repos:
            raise DependencyResolutionError(
                dependency_name=repo_path,
                reason="Not a git repository",
            )

        # Update ref
        url, _ = self._cloned_repos[repo_path]
        self._cloned_repos[repo_path] = (url, ref)

    def is_repository(self, path: str) -> bool:
        """Check if path is a known repository.

        Args:
            path: Path to check

        Returns:
            True if path is a known repository
        """
        return path in self._cloned_repos

    def resolve_ref(self, repo_path: str, ref: str) -> str:
        """Resolve git ref to commit hash.

        Args:
            repo_path: Path to git repository
            ref: Git reference

        Returns:
            40-character commit hash

        Raises:
            ValueError: If ref not found
        """
        # Check if we have a configured ref
        key = (repo_path, ref)
        if key in self._refs:
            return self._refs[key]

        # If repo doesn't exist, error
        if repo_path not in self._cloned_repos:
            raise ValueError(f"Not a git repository: {repo_path}")

        # Return a fake but valid commit hash based on ref
        # Use deterministic hash for consistent testing
        import hashlib
        hash_input = f"{repo_path}:{ref}".encode()
        commit_hash = hashlib.sha1(hash_input).hexdigest()
        return commit_hash

    # Test helpers below

    def configure_failure(self, url: str, error: str) -> None:
        """Configure clone to fail for specific URL (test helper).

        Args:
            url: URL that should fail
            error: Error message to raise
        """
        self._should_fail[url] = error

    def was_cloned(self, url: str, destination: str, ref: str) -> bool:
        """Check if clone was called with specific args (test helper).

        Args:
            url: Expected git URL
            destination: Expected destination path
            ref: Expected git ref

        Returns:
            True if clone was called with these arguments
        """
        return (url, destination, ref) in self._clone_calls

    def get_clone_count(self) -> int:
        """Get number of clone calls (test helper).

        Returns:
            Total number of clone calls
        """
        return len(self._clone_calls)

    def get_fetch_count(self) -> int:
        """Get number of fetch calls (test helper).

        Returns:
            Total number of fetch calls
        """
        return len(self._fetch_calls)

    def get_cloned_repos(self) -> dict[str, tuple[str, str]]:
        """Get all cloned repositories (test helper).

        Returns:
            Dictionary of path -> (url, ref)
        """
        return self._cloned_repos.copy()

    def fetch_all(self, repo_path: str) -> None:
        """Fetch all refs from remote without checking out (fake).

        Args:
            repo_path: Path to existing git repository

        Raises:
            DependencyResolutionError: If repository doesn't exist
        """
        # Must be a known repository
        if repo_path not in self._cloned_repos:
            raise DependencyResolutionError(
                dependency_name=repo_path,
                reason="Not a git repository",
            )

        # Simulate successful fetch (no-op for fake)
        # In real git, this would update remote-tracking branches

    def configure_ref(self, repo_path: str, ref: str, commit: str) -> None:
        """Configure a ref to resolve to a specific commit (test helper).

        Args:
            repo_path: Repository path
            ref: Git reference
            commit: Commit hash to return
        """
        self._refs[(repo_path, ref)] = commit

    def reset(self) -> None:
        """Reset all state (test helper).

        Clears all repositories and call history.
        """
        self._cloned_repos.clear()
        self._clone_calls.clear()
        self._fetch_calls.clear()
        self._should_fail.clear()
        self._refs.clear()
