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

    def resolve_ref(self, repo_path: str, ref: str) -> str:
        """Resolve a git ref to its commit hash.

        Args:
            repo_path: Path to git repository
            ref: Git reference (branch, tag, or commit hash)

        Returns:
            Full 40-character commit hash

        Raises:
            Exception: If ref doesn't exist or repo is invalid
        """
        ...

    def fetch_all(self, repo_path: str) -> None:
        """Fetch all refs from remote without checking out.

        Updates remote-tracking branches without modifying working directory.

        Args:
            repo_path: Path to existing git repository

        Raises:
            Exception: If fetch fails
        """
        ...

    def checkout(self, repo_path: str, ref: str) -> None:
        """Checkout a specific ref in a repository.

        Args:
            repo_path: Path to git repository
            ref: Git reference to checkout (branch, tag, or commit hash)

        Raises:
            Exception: If checkout fails
        """
        ...

    def get_current_commit(self, repo_path: str) -> str:
        """Get the current commit hash of the repository.

        Args:
            repo_path: Path to git repository

        Returns:
            Full 40-character commit hash of HEAD

        Raises:
            Exception: If unable to get commit hash
        """
        ...

    def add_submodule(self, url: str, path: str, ref: str | None = None) -> None:
        """Add a git submodule at the specified path.

        Args:
            url: Git repository URL for the submodule
            path: Local path where submodule should be added
            ref: Optional branch/tag to track (defaults to remote HEAD)

        Raises:
            Exception: If submodule addition fails
        """
        ...

    def update_submodule(self, path: str, init: bool = True, recursive: bool = False) -> None:
        """Update submodule to match .gitmodules configuration.

        Args:
            path: Path to the submodule
            init: Whether to initialize the submodule if not already
            recursive: Whether to update nested submodules

        Raises:
            Exception: If submodule update fails
        """
        ...

    def remove_submodule(self, path: str) -> None:
        """Remove a submodule and clean up .gitmodules.

        Performs complete submodule removal:
        - Removes from .gitmodules
        - Deinitializes submodule
        - Removes submodule directory
        - Removes from .git/modules

        Args:
            path: Path to the submodule to remove

        Raises:
            Exception: If submodule removal fails
        """
        ...

    def get_submodule_status(self, path: str) -> dict[str, str]:
        """Get status info for a submodule.

        Args:
            path: Path to the submodule

        Returns:
            Dictionary with status information:
            - 'commit': Current commit hash
            - 'branch': Tracking branch (if any)
            - 'status': Status indicator (+modified, -uninitialized, etc.)

        Raises:
            Exception: If unable to get submodule status
        """
        ...

    def sync_submodule(self, path: str) -> None:
        """Sync submodule URL with .gitmodules.

        Updates the submodule's remote URL to match what's in .gitmodules.
        Useful after .gitmodules has been modified.

        Args:
            path: Path to the submodule

        Raises:
            Exception: If sync fails
        """
        ...

    def set_submodule_branch(self, path: str, branch: str) -> None:
        """Set the branch for a submodule in .gitmodules.

        Args:
            path: Path to the submodule
            branch: Branch name to track

        Raises:
            Exception: If unable to set branch
        """
        ...

    def deinit_submodule(self, path: str, force: bool = False) -> None:
        """Deinitialize a submodule without removing it.

        Removes submodule working tree but keeps .gitmodules entry.

        Args:
            path: Path to the submodule
            force: Force deinit even if submodule has local changes

        Raises:
            Exception: If deinit fails
        """
        ...

    def is_submodule(self, path: str) -> bool:
        """Check if a path is a registered git submodule.

        Args:
            path: Path to check

        Returns:
            True if path is a registered submodule in .gitmodules
        """
        ...

    def is_working_directory_clean(self, repo_path: str) -> bool:
        """Check if the working directory has uncommitted changes.

        Args:
            repo_path: Path to git repository

        Returns:
            True if working directory is clean (no uncommitted changes)
        """
        ...
