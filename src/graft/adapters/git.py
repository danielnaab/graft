"""Git operations adapter using subprocess.

Real git operations using git CLI via subprocess.
"""

import subprocess
from pathlib import Path

from graft.domain.exceptions import (
    GitAuthenticationError,
    GitCloneError,
    GitFetchError,
    GitNotFoundError,
    SubmoduleOperationError,
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

    def resolve_ref(self, repo_path: str, ref: str) -> str:
        """Resolve git ref to commit hash.

        Args:
            repo_path: Path to git repository
            ref: Git reference (branch, tag, or commit)

        Returns:
            Full 40-character commit hash

        Raises:
            ValueError: If ref doesn't exist or repo is invalid
        """
        try:
            # Use git rev-parse to resolve ref to commit hash
            result = subprocess.run(
                ["git", "rev-parse", ref],
                cwd=repo_path,
                capture_output=True,
                text=True,
                check=False,
            )

            if result.returncode != 0:
                raise ValueError(
                    f"Ref '{ref}' not found in repository {repo_path}: {result.stderr.strip()}"
                )

            commit = result.stdout.strip()

            # Verify it's a valid 40-char hash
            if len(commit) != 40 or not all(c in "0123456789abcdef" for c in commit):
                raise ValueError(
                    f"Invalid commit hash returned for ref '{ref}': {commit}"
                )

            return commit

        except subprocess.SubprocessError as e:
            raise ValueError(
                f"Failed to resolve ref '{ref}' in {repo_path}: {e}"
            ) from e

    def fetch_all(self, repo_path: str) -> None:
        """Fetch all refs from remote without checking out.

        Updates remote-tracking branches without modifying working directory.

        Args:
            repo_path: Path to existing git repository

        Raises:
            GitFetchError: If fetch fails
            GitAuthenticationError: If authentication fails
        """
        try:
            # Fetch all refs from origin
            fetch_cmd = ["git", "-C", repo_path, "fetch", "origin"]
            result = subprocess.run(fetch_cmd, capture_output=True, text=True, check=False)

            if result.returncode != 0:
                dep_name = Path(repo_path).name
                stderr = result.stderr.strip()

                # Detect authentication errors
                if "Permission denied" in stderr or "publickey" in stderr:
                    raise GitAuthenticationError(
                        dependency_name=dep_name,
                        url=repo_path,
                        suggestion="Check SSH keys and repository access",
                    )
                else:
                    raise GitFetchError(
                        dependency_name=dep_name,
                        repo_path=repo_path,
                        ref="(all)",
                        stderr=stderr,
                        returncode=result.returncode,
                    )

        except subprocess.SubprocessError as e:
            raise GitFetchError(
                dependency_name=Path(repo_path).name,
                repo_path=repo_path,
                ref="(all)",
                stderr=f"Subprocess error: {e}",
                returncode=1,
            ) from e

    def checkout(self, repo_path: str, ref: str) -> None:
        """Checkout a specific ref in a repository.

        Args:
            repo_path: Path to git repository
            ref: Git reference to checkout (branch, tag, or commit hash)

        Raises:
            GitFetchError: If checkout fails
        """
        try:
            checkout_cmd = ["git", "-C", repo_path, "checkout", ref]
            result = subprocess.run(checkout_cmd, capture_output=True, text=True, check=False)

            if result.returncode != 0:
                dep_name = Path(repo_path).name
                stderr = result.stderr.strip()
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

    def get_current_commit(self, repo_path: str) -> str:
        """Get the current commit hash of the repository.

        Args:
            repo_path: Path to git repository

        Returns:
            Full 40-character commit hash of HEAD

        Raises:
            ValueError: If unable to get commit hash
        """
        try:
            result = subprocess.run(
                ["git", "rev-parse", "HEAD"],
                cwd=repo_path,
                capture_output=True,
                text=True,
                check=False,
            )

            if result.returncode != 0:
                raise ValueError(
                    f"Failed to get current commit in {repo_path}: {result.stderr.strip()}"
                )

            commit = result.stdout.strip()

            # Verify it's a valid 40-char hash
            if len(commit) != 40 or not all(c in "0123456789abcdef" for c in commit):
                raise ValueError(
                    f"Invalid commit hash returned: {commit}"
                )

            return commit

        except subprocess.SubprocessError as e:
            raise ValueError(
                f"Failed to get current commit in {repo_path}: {e}"
            ) from e

    def add_submodule(self, url: str, path: str, ref: str | None = None) -> None:
        """Add a git submodule at the specified path.

        Args:
            url: Git repository URL for the submodule
            path: Local path where submodule should be added
            ref: Optional branch/tag to track (defaults to remote HEAD)

        Raises:
            GitCloneError: If submodule addition fails
        """
        try:
            # Note: We don't use --branch here because it only works with branches,
            # not tags or commits. The caller should checkout the specific ref
            # after adding the submodule.
            cmd = ["git", "submodule", "add", url, path]

            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                check=False,
            )

            if result.returncode != 0:
                stderr = result.stderr.strip()
                dep_name = Path(path).name

                # Check for specific errors and provide helpful messages
                if "already exists in the index" in stderr:
                    # Submodule already added, not an error
                    return
                elif "Permission denied" in stderr or "publickey" in stderr:
                    raise GitAuthenticationError(
                        dependency_name=dep_name,
                        url=url,
                        suggestion="Configure SSH keys for submodule access",
                    )
                elif "is outside repository" in stderr:
                    raise GitCloneError(
                        dependency_name=dep_name,
                        url=url,
                        ref=ref or "HEAD",
                        stderr=f"Path is outside repository: {path}. "
                               "Submodules must be inside the git repository. "
                               "Check your deps_directory setting.",
                        returncode=result.returncode,
                    )
                elif "not a commit" in stderr or "could not be found" in stderr:
                    raise GitCloneError(
                        dependency_name=dep_name,
                        url=url,
                        ref=ref or "HEAD",
                        stderr=f"Branch/ref '{ref}' not found: {stderr}",
                        returncode=result.returncode,
                    )
                elif "unable to connect" in stderr.lower() or "could not resolve" in stderr.lower():
                    raise GitCloneError(
                        dependency_name=dep_name,
                        url=url,
                        ref=ref or "HEAD",
                        stderr=f"Network error: {stderr}. Check your internet connection.",
                        returncode=result.returncode,
                    )
                elif "repository not found" in stderr.lower():
                    raise GitCloneError(
                        dependency_name=dep_name,
                        url=url,
                        ref=ref or "HEAD",
                        stderr=f"Repository not found: {url}. Check the URL is correct.",
                        returncode=result.returncode,
                    )
                else:
                    raise GitCloneError(
                        dependency_name=dep_name,
                        url=url,
                        ref=ref or "HEAD",
                        stderr=f"Failed to add submodule: {stderr}",
                        returncode=result.returncode,
                    )

        except subprocess.SubprocessError as e:
            raise GitCloneError(
                dependency_name=Path(path).name,
                url=url,
                ref=ref or "HEAD",
                stderr=f"Subprocess error: {e}",
                returncode=1,
            ) from e

    def update_submodule(self, path: str, init: bool = True, recursive: bool = False) -> None:
        """Update submodule to match .gitmodules configuration.

        Args:
            path: Path to the submodule
            init: Whether to initialize the submodule if not already
            recursive: Whether to update nested submodules

        Raises:
            GitFetchError: If submodule update fails
        """
        try:
            cmd = ["git", "submodule", "update"]

            if init:
                cmd.append("--init")

            if recursive:
                cmd.append("--recursive")

            cmd.append(path)

            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                check=False,
            )

            if result.returncode != 0:
                stderr = result.stderr.strip()
                dep_name = Path(path).name

                if "Permission denied" in stderr or "publickey" in stderr:
                    raise GitAuthenticationError(
                        dependency_name=dep_name,
                        url=path,
                        suggestion="Check SSH keys for submodule access",
                    )
                elif "not initialized" in stderr.lower():
                    raise GitFetchError(
                        dependency_name=dep_name,
                        repo_path=path,
                        ref="(submodule)",
                        stderr=f"Submodule not initialized: {path}. "
                               "Run 'git submodule update --init' first.",
                        returncode=result.returncode,
                    )
                elif "unable to connect" in stderr.lower() or "could not resolve" in stderr.lower():
                    raise GitFetchError(
                        dependency_name=dep_name,
                        repo_path=path,
                        ref="(submodule)",
                        stderr=f"Network error updating submodule: {stderr}",
                        returncode=result.returncode,
                    )
                else:
                    raise GitFetchError(
                        dependency_name=dep_name,
                        repo_path=path,
                        ref="(submodule)",
                        stderr=f"Failed to update submodule: {stderr}",
                        returncode=result.returncode,
                    )

        except subprocess.SubprocessError as e:
            raise GitFetchError(
                dependency_name=Path(path).name,
                repo_path=path,
                ref="(submodule)",
                stderr=f"Subprocess error: {e}",
                returncode=1,
            ) from e

    def remove_submodule(self, path: str) -> None:
        """Remove a submodule and clean up .gitmodules.

        Args:
            path: Path to the submodule to remove

        Raises:
            SubmoduleOperationError: If submodule removal fails
        """
        import shutil

        try:
            # Get repository root for absolute path construction
            root_result = subprocess.run(
                ["git", "rev-parse", "--show-toplevel"],
                capture_output=True,
                text=True,
                check=False,
            )
            if root_result.returncode != 0:
                raise SubmoduleOperationError(
                    path=path,
                    operation="remove",
                    reason="Not in a git repository",
                )
            repo_root = Path(root_result.stdout.strip())

            # Step 1: Deinitialize the submodule
            deinit_cmd = ["git", "submodule", "deinit", "-f", path]
            result = subprocess.run(deinit_cmd, capture_output=True, text=True, check=False)

            if result.returncode != 0 and "not a valid submodule" not in result.stderr:
                raise SubmoduleOperationError(
                    path=path,
                    operation="deinit",
                    reason=result.stderr.strip(),
                )

            # Step 2: Remove from index and .gitmodules
            rm_modules_cmd = ["git", "rm", "-f", path]
            result = subprocess.run(rm_modules_cmd, capture_output=True, text=True, check=False)

            if result.returncode != 0 and "did not match any files" not in result.stderr:
                raise SubmoduleOperationError(
                    path=path,
                    operation="remove",
                    reason=result.stderr.strip(),
                )

            # Step 3: Remove the .git/modules/<name> directory using absolute path
            modules_path = repo_root / ".git" / "modules" / Path(path).name
            if modules_path.exists():
                shutil.rmtree(modules_path, ignore_errors=True)

        except SubmoduleOperationError:
            raise
        except subprocess.SubprocessError as e:
            raise SubmoduleOperationError(
                path=path,
                operation="remove",
                reason=str(e),
            ) from e

    def get_submodule_status(self, path: str) -> dict[str, str]:
        """Get status info for a submodule.

        Args:
            path: Path to the submodule

        Returns:
            Dictionary with status information

        Raises:
            ValueError: If unable to get submodule status
        """
        try:
            # Get submodule status
            status_cmd = ["git", "submodule", "status", path]
            result = subprocess.run(status_cmd, capture_output=True, text=True, check=False)

            if result.returncode != 0:
                raise ValueError(f"Failed to get submodule status: {result.stderr.strip()}")

            output = result.stdout.strip()
            if not output:
                raise ValueError(f"No submodule found at {path}")

            # Parse status output (format: <status><commit> <path> (<description>))
            parts = output.split()
            if len(parts) < 2:
                raise ValueError(f"Invalid submodule status output: {output}")

            status_info = {}

            # First part may have status prefix (+, -, U, etc.)
            commit_part = parts[0]
            if commit_part[0] in "+-U ":
                status_info["status"] = commit_part[0]
                status_info["commit"] = commit_part[1:]
            else:
                status_info["status"] = " "
                status_info["commit"] = commit_part

            # Get branch if configured
            branch_cmd = ["git", "config", "-f", ".gitmodules", f"submodule.{Path(path).name}.branch"]
            branch_result = subprocess.run(branch_cmd, capture_output=True, text=True, check=False)

            if branch_result.returncode == 0:
                status_info["branch"] = branch_result.stdout.strip()
            else:
                status_info["branch"] = ""

            return status_info

        except subprocess.SubprocessError as e:
            raise ValueError(f"Failed to get submodule status for {path}: {e}") from e

    def sync_submodule(self, path: str) -> None:
        """Sync submodule URL with .gitmodules.

        Args:
            path: Path to the submodule

        Raises:
            ValueError: If sync fails
        """
        try:
            sync_cmd = ["git", "submodule", "sync", path]
            result = subprocess.run(sync_cmd, capture_output=True, text=True, check=False)

            if result.returncode != 0:
                raise ValueError(f"Failed to sync submodule: {result.stderr.strip()}")

        except subprocess.SubprocessError as e:
            raise ValueError(f"Failed to sync submodule {path}: {e}") from e

    def set_submodule_branch(self, path: str, branch: str) -> None:
        """Set the branch for a submodule in .gitmodules.

        Args:
            path: Path to the submodule
            branch: Branch name to track

        Raises:
            ValueError: If unable to set branch
        """
        try:
            # Set branch in .gitmodules
            config_key = f"submodule.{Path(path).name}.branch"
            config_cmd = ["git", "config", "-f", ".gitmodules", config_key, branch]
            result = subprocess.run(config_cmd, capture_output=True, text=True, check=False)

            if result.returncode != 0:
                raise ValueError(f"Failed to set submodule branch: {result.stderr.strip()}")

        except subprocess.SubprocessError as e:
            raise ValueError(f"Failed to set branch for submodule {path}: {e}") from e

    def deinit_submodule(self, path: str, force: bool = False) -> None:
        """Deinitialize a submodule without removing it.

        Args:
            path: Path to the submodule
            force: Force deinit even if submodule has local changes

        Raises:
            ValueError: If deinit fails
        """
        try:
            cmd = ["git", "submodule", "deinit"]
            if force:
                cmd.append("-f")
            cmd.append(path)

            result = subprocess.run(cmd, capture_output=True, text=True, check=False)

            if result.returncode != 0:
                raise ValueError(f"Failed to deinit submodule: {result.stderr.strip()}")

        except subprocess.SubprocessError as e:
            raise ValueError(f"Failed to deinit submodule {path}: {e}") from e

    def is_submodule(self, path: str) -> bool:
        """Check if a path is a registered git submodule.

        Args:
            path: Path to check

        Returns:
            True if path is a registered submodule
        """
        try:
            # Check if path is in git submodule status output
            status_cmd = ["git", "submodule", "status", path]
            result = subprocess.run(status_cmd, capture_output=True, text=True, check=False)

            # If return code is 0 and there's output, it's a submodule
            return result.returncode == 0 and bool(result.stdout.strip())

        except subprocess.SubprocessError:
            return False

    def is_working_directory_clean(self, repo_path: str) -> bool:
        """Check if the working directory has uncommitted changes.

        Args:
            repo_path: Path to git repository

        Returns:
            True if working directory is clean (no uncommitted changes)
        """
        try:
            result = subprocess.run(
                ["git", "status", "--porcelain"],
                cwd=repo_path,
                capture_output=True,
                text=True,
                check=False,
            )
            # Empty output means clean working directory
            return result.returncode == 0 and not result.stdout.strip()

        except subprocess.SubprocessError:
            # If we can't check status, assume not clean for safety
            return False
