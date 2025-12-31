"""Git operations protocol.

Protocol for git repository operations enabling testability with fakes.
"""

from typing import Protocol


class GitOperations(Protocol):
    """Protocol for git repository operations.

    Any class implementing these methods satisfies this protocol.
    Enables testing with fake implementations.

    Example implementations:
        - SubprocessGitOperations (uses subprocess to call git CLI)
        - FakeGitOperations (in-memory for testing)
    """

    def clone(
        self,
        url: str,
        destination: str,
        ref: str,
    ) -> None:
        """Clone git repository.

        Clones repository to destination directory and checks out specified ref.

        Args:
            url: Git repository URL
            destination: Local path to clone into
            ref: Git reference to checkout (branch/tag/commit)

        Raises:
            DependencyResolutionError: If clone fails
        """
        ...

    def fetch(
        self,
        repo_path: str,
        ref: str,
    ) -> None:
        """Fetch and checkout reference in existing repository.

        Updates existing repository to specified ref.

        Args:
            repo_path: Path to existing git repository
            ref: Git reference to checkout

        Raises:
            DependencyResolutionError: If fetch/checkout fails
        """
        ...

    def is_repository(self, path: str) -> bool:
        """Check if path is a git repository.

        Args:
            path: Path to check

        Returns:
            True if path is a git repository (contains .git directory)
        """
        ...
