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
        self._checkout_calls: list[tuple[str, str]] = []  # (path, ref)
        self._should_fail: dict[str, str] = {}  # url -> error message
        self._refs: dict[tuple[str, str], str] = {}  # (repo_path, ref) -> commit hash
        self._current_commits: dict[str, str] = {}  # repo_path -> current commit
        self._working_directory_clean: dict[str, bool] = {}  # repo_path -> is_clean
        # Submodule tracking
        self._submodules: dict[str, dict[str, str]] = {}  # path -> {url, ref, status}
        self._add_submodule_calls: list[tuple[str, str, str | None]] = []  # (url, path, ref)
        self._update_submodule_calls: list[tuple[str, bool, bool]] = []  # (path, init, recursive)
        self._remove_submodule_calls: list[str] = []  # paths
        self._submodule_branches: dict[str, str] = {}  # path -> branch
        # Worktree tracking
        self._worktrees: dict[str, str] = {}  # worktree_path -> commit
        self._add_worktree_calls: list[tuple[str, str, str]] = []  # (repo_path, worktree_path, commit)
        self._remove_worktree_calls: list[tuple[str, str]] = []  # (repo_path, worktree_path)

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
        """Check if path is a known repository (includes submodules).

        Args:
            path: Path to check

        Returns:
            True if path is a known repository or submodule
        """
        return path in self._cloned_repos or path in self._submodules

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

        # If repo doesn't exist (neither as clone nor submodule), error
        if repo_path not in self._cloned_repos and repo_path not in self._submodules:
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
        # Must be a known repository or submodule
        if repo_path not in self._cloned_repos and repo_path not in self._submodules:
            raise DependencyResolutionError(
                dependency_name=repo_path,
                reason="Not a git repository",
            )

        # Simulate successful fetch (no-op for fake)
        # In real git, this would update remote-tracking branches
        # Note: For submodules, the configured refs via configure_ref() will be used
        # when resolving refs like "origin/main"

    def checkout(self, repo_path: str, ref: str) -> None:
        """Checkout a specific ref in a repository (fake).

        Args:
            repo_path: Path to git repository
            ref: Git reference to checkout

        Raises:
            DependencyResolutionError: If repository doesn't exist
        """
        self._checkout_calls.append((repo_path, ref))

        # Must be a known repository or submodule
        is_cloned = repo_path in self._cloned_repos
        is_submodule = repo_path in self._submodules

        if not is_cloned and not is_submodule:
            raise DependencyResolutionError(
                dependency_name=repo_path,
                reason="Not a git repository",
            )

        # Determine the commit hash for this checkout
        if len(ref) == 40 and all(c in "0123456789abcdef" for c in ref):
            # Direct commit hash
            commit = ref
        else:
            # Try to resolve ref to commit (handles configured refs)
            try:
                commit = self.resolve_ref(repo_path, ref)
            except ValueError:
                # Generate a deterministic hash if not configured
                commit = self._generate_commit_hash(f"{repo_path}:{ref}")

        # Update current commit
        self._current_commits[repo_path] = commit

        # Update the stored ref for cloned repos
        if is_cloned:
            url, _ = self._cloned_repos[repo_path]
            self._cloned_repos[repo_path] = (url, ref)

        # Update submodule commit - always sync with current_commits
        if is_submodule:
            self._submodules[repo_path]["commit"] = commit

    def get_current_commit(self, repo_path: str) -> str:
        """Get the current commit hash of the repository (fake).

        Args:
            repo_path: Path to git repository

        Returns:
            Full 40-character commit hash of HEAD

        Raises:
            ValueError: If unable to get commit hash
        """
        # Check if it's a known repository or submodule
        if repo_path not in self._cloned_repos and repo_path not in self._submodules:
            raise ValueError(f"Not a git repository: {repo_path}")

        if repo_path in self._current_commits:
            return self._current_commits[repo_path]

        # Check if it's a submodule
        if repo_path in self._submodules:
            return self._submodules[repo_path]["commit"]

        # Return a deterministic fake commit hash for cloned repos
        url, ref = self._cloned_repos[repo_path]
        return self.resolve_ref(repo_path, ref)

    def configure_ref(self, repo_path: str, ref: str, commit: str) -> None:
        """Configure a ref to resolve to a specific commit (test helper).

        Args:
            repo_path: Repository path
            ref: Git reference
            commit: Commit hash to return
        """
        self._refs[(repo_path, ref)] = commit

    def configure_current_commit(self, repo_path: str, commit: str) -> None:
        """Configure the current commit for a repository (test helper).

        Args:
            repo_path: Repository path
            commit: Current commit hash
        """
        self._current_commits[repo_path] = commit

    def reset(self) -> None:
        """Reset all state (test helper).

        Clears all repositories and call history.
        """
        self._cloned_repos.clear()
        self._clone_calls.clear()
        self._fetch_calls.clear()
        self._checkout_calls.clear()
        self._should_fail.clear()
        self._refs.clear()
        self._current_commits.clear()
        self._working_directory_clean.clear()
        self._submodules.clear()
        self._add_submodule_calls.clear()
        self._update_submodule_calls.clear()
        self._remove_submodule_calls.clear()
        self._submodule_branches.clear()
        self._worktrees.clear()
        self._add_worktree_calls.clear()
        self._remove_worktree_calls.clear()

    def configure_working_directory_clean(self, repo_path: str, is_clean: bool) -> None:
        """Configure whether a working directory should appear clean (test helper).

        Args:
            repo_path: Repository path
            is_clean: True if directory should appear clean
        """
        self._working_directory_clean[repo_path] = is_clean

    # Submodule operations

    def add_submodule(self, url: str, path: str, ref: str | None = None) -> None:
        """Add a git submodule (fake).

        Args:
            url: Git repository URL for the submodule
            path: Local path where submodule should be added
            ref: Optional branch/tag to track

        Raises:
            DependencyResolutionError: If configured to fail
        """
        self._add_submodule_calls.append((url, path, ref))

        # Simulate failure if configured
        if url in self._should_fail:
            raise DependencyResolutionError(
                dependency_name=path,
                reason=self._should_fail[url],
            )

        self._submodules[path] = {
            "url": url,
            "ref": ref or "HEAD",
            "status": " ",  # Normal status
            "commit": self._generate_commit_hash(f"{url}:{ref or 'HEAD'}"),
        }
        if ref:
            self._submodule_branches[path] = ref

    def update_submodule(self, path: str, init: bool = True, recursive: bool = False) -> None:
        """Update submodule (fake).

        Args:
            path: Path to the submodule
            init: Whether to initialize the submodule
            recursive: Whether to update nested submodules
        """
        self._update_submodule_calls.append((path, init, recursive))

        if init and path not in self._submodules:
            # Initialize with default values
            self._submodules[path] = {
                "url": f"https://example.com/{path}.git",
                "ref": "HEAD",
                "status": " ",
                "commit": self._generate_commit_hash(f"{path}:HEAD"),
            }

    def remove_submodule(self, path: str) -> None:
        """Remove a submodule (fake).

        Args:
            path: Path to the submodule to remove
        """
        self._remove_submodule_calls.append(path)
        if path in self._submodules:
            del self._submodules[path]
        if path in self._submodule_branches:
            del self._submodule_branches[path]

    def get_submodule_status(self, path: str) -> dict[str, str]:
        """Get status info for a submodule (fake).

        Args:
            path: Path to the submodule

        Returns:
            Dictionary with status information
        """
        if path not in self._submodules:
            raise ValueError(f"No submodule at {path}")

        info = self._submodules[path]
        return {
            "commit": info["commit"],
            "branch": self._submodule_branches.get(path, ""),
            "status": info["status"],
        }

    def sync_submodule(self, path: str) -> None:
        """Sync submodule URL (fake no-op).

        Args:
            path: Path to the submodule
        """
        if path not in self._submodules:
            raise ValueError(f"No submodule at {path}")
        # No-op for fake

    def set_submodule_branch(self, path: str, branch: str) -> None:
        """Set the branch for a submodule (fake).

        Args:
            path: Path to the submodule
            branch: Branch name to track
        """
        if path not in self._submodules:
            raise ValueError(f"No submodule at {path}")
        self._submodule_branches[path] = branch

    def deinit_submodule(self, path: str, force: bool = False) -> None:
        """Deinitialize a submodule (fake).

        Args:
            path: Path to the submodule
            force: Force deinit even with changes
        """
        if path in self._submodules:
            self._submodules[path]["status"] = "-"  # Uninitialized

    def is_submodule(self, path: str) -> bool:
        """Check if a path is a registered git submodule (fake).

        Args:
            path: Path to check

        Returns:
            True if path is a registered submodule
        """
        return path in self._submodules

    def is_working_directory_clean(self, repo_path: str) -> bool:
        """Check if the working directory has uncommitted changes (fake).

        Args:
            repo_path: Path to git repository

        Returns:
            True if working directory is clean (configured via test helper)
        """
        # By default, assume clean unless configured otherwise
        return self._working_directory_clean.get(repo_path, True)

    # Worktree operations

    def add_worktree(self, repo_path: str, worktree_path: str, commit: str) -> None:
        """Create a git worktree at a specific commit (fake).

        Args:
            repo_path: Path to main git repository
            worktree_path: Path where worktree should be created
            commit: Git commit hash to checkout in worktree

        Raises:
            ValueError: If repository doesn't exist
        """
        self._add_worktree_calls.append((repo_path, worktree_path, commit))

        # Must be a known repository
        if repo_path not in self._cloned_repos and repo_path not in self._submodules:
            raise ValueError(f"Not a git repository: {repo_path}")

        # Record worktree
        self._worktrees[worktree_path] = commit

    def remove_worktree(self, repo_path: str, worktree_path: str) -> None:
        """Remove a git worktree (fake).

        Args:
            repo_path: Path to main git repository
            worktree_path: Path to worktree to remove

        Raises:
            ValueError: If worktree doesn't exist
        """
        self._remove_worktree_calls.append((repo_path, worktree_path))

        # Remove worktree if it exists (ignore if not)
        if worktree_path in self._worktrees:
            del self._worktrees[worktree_path]

    # Submodule test helpers

    def was_submodule_added(self, url: str, path: str, ref: str | None = None) -> bool:
        """Check if add_submodule was called with specific args.

        Args:
            url: Expected URL
            path: Expected path
            ref: Expected ref (optional)

        Returns:
            True if add_submodule was called with these arguments
        """
        return (url, path, ref) in self._add_submodule_calls

    def get_submodule_count(self) -> int:
        """Get number of registered submodules.

        Returns:
            Number of submodules
        """
        return len(self._submodules)

    def get_add_submodule_calls(self) -> list[tuple[str, str, str | None]]:
        """Get all add_submodule calls.

        Returns:
            List of (url, path, ref) tuples
        """
        return self._add_submodule_calls.copy()

    def get_update_submodule_calls(self) -> list[tuple[str, bool, bool]]:
        """Get all update_submodule calls.

        Returns:
            List of (path, init, recursive) tuples
        """
        return self._update_submodule_calls.copy()

    def get_remove_submodule_calls(self) -> list[str]:
        """Get all remove_submodule calls.

        Returns:
            List of removed paths
        """
        return self._remove_submodule_calls.copy()

    def _generate_commit_hash(self, seed: str) -> str:
        """Generate a deterministic fake commit hash.

        Args:
            seed: Seed string for hash generation

        Returns:
            40-character hex string
        """
        import hashlib
        return hashlib.sha1(seed.encode()).hexdigest()

    # Worktree test helpers

    def was_worktree_added(self, repo_path: str, worktree_path: str, commit: str) -> bool:
        """Check if add_worktree was called with specific args.

        Args:
            repo_path: Expected repository path
            worktree_path: Expected worktree path
            commit: Expected commit

        Returns:
            True if add_worktree was called with these arguments
        """
        return (repo_path, worktree_path, commit) in self._add_worktree_calls

    def was_worktree_removed(self, repo_path: str, worktree_path: str) -> bool:
        """Check if remove_worktree was called with specific args.

        Args:
            repo_path: Expected repository path
            worktree_path: Expected worktree path

        Returns:
            True if remove_worktree was called with these arguments
        """
        return (repo_path, worktree_path) in self._remove_worktree_calls

    def get_worktree_count(self) -> int:
        """Get number of active worktrees.

        Returns:
            Number of worktrees
        """
        return len(self._worktrees)

    def get_add_worktree_calls(self) -> list[tuple[str, str, str]]:
        """Get all add_worktree calls.

        Returns:
            List of (repo_path, worktree_path, commit) tuples
        """
        return self._add_worktree_calls.copy()

    def get_remove_worktree_calls(self) -> list[tuple[str, str]]:
        """Get all remove_worktree calls.

        Returns:
            List of (repo_path, worktree_path) tuples
        """
        return self._remove_worktree_calls.copy()
