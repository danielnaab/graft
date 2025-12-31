"""Unit tests for git error handling.

Tests that git adapter correctly detects and raises specific error types.

Rationale:
    Git operations fail in predictable ways (auth errors, not found, network issues).
    These tests ensure we detect failure types from git stderr and raise appropriate
    exceptions with helpful context. This enables:
    - Targeted error recovery (retry vs abort)
    - User-friendly error messages
    - Monitoring/alerting on specific error types
"""

from unittest.mock import MagicMock, patch

import pytest

from graft.adapters.git import SubprocessGitOperations
from graft.domain.exceptions import (
    GitAuthenticationError,
    GitCloneError,
    GitFetchError,
    GitNotFoundError,
)


class TestGitCloneErrors:
    """Tests for git clone error detection.

    Rationale: Clone is the primary operation. Different failure modes require
    different user guidance (fix SSH keys vs check URL vs check network).
    """

    @patch("graft.adapters.git.subprocess.run")
    def test_authentication_error_detected(self, mock_run: MagicMock) -> None:
        """Should raise GitAuthenticationError for SSH key issues.

        Rationale: Authentication failures are common and fixable.
        User needs guidance on configuring SSH keys.
        """
        # Setup: Mock git clone failure with publickey error
        mock_run.return_value = MagicMock(
            returncode=128,
            stderr="Permission denied (publickey).",
            stdout="",
        )

        git = SubprocessGitOperations()

        # Exercise & Verify
        with pytest.raises(GitAuthenticationError) as exc_info:
            git.clone("ssh://git@github.com/user/repo.git", "/tmp/repo", "main")

        # Verify error includes helpful context
        assert exc_info.value.url == "ssh://git@github.com/user/repo.git"
        assert "SSH keys" in exc_info.value.suggestion

    @patch("graft.adapters.git.subprocess.run")
    def test_repository_not_found_detected(self, mock_run: MagicMock) -> None:
        """Should raise GitNotFoundError for missing repository.

        Rationale: Repository might not exist or URL might be wrong.
        User needs to verify the URL.
        """
        # Setup: Mock git clone failure with not found error
        mock_run.return_value = MagicMock(
            returncode=128,
            stderr="fatal: repository 'https://github.com/user/repo.git' not found",
            stdout="",
        )

        git = SubprocessGitOperations()

        # Exercise & Verify
        with pytest.raises(GitNotFoundError) as exc_info:
            git.clone("https://github.com/user/repo.git", "/tmp/repo", "main")

        assert exc_info.value.url == "https://github.com/user/repo.git"
        assert exc_info.value.ref == "main"

    @patch("graft.adapters.git.subprocess.run")
    def test_generic_clone_error(self, mock_run: MagicMock) -> None:
        """Should raise GitCloneError for other clone failures.

        Rationale: Not all git errors are predictable. Generic error
        should include full stderr for debugging.
        """
        # Setup: Mock git clone failure with generic error
        mock_run.return_value = MagicMock(
            returncode=1,
            stderr="fatal: unable to access 'https://github.com/': Network unreachable",
            stdout="",
        )

        git = SubprocessGitOperations()

        # Exercise & Verify
        with pytest.raises(GitCloneError) as exc_info:
            git.clone("https://github.com/user/repo.git", "/tmp/repo", "main")

        assert "Network unreachable" in exc_info.value.stderr
        assert exc_info.value.returncode == 1


class TestGitFetchErrors:
    """Tests for git fetch/checkout error detection.

    Rationale: Updating existing repos fails differently than cloning.
    Most common issue is ref not found (branch deleted, tag doesn't exist).
    """

    @patch("graft.adapters.git.subprocess.run")
    def test_ref_not_found_detected(self, mock_run: MagicMock) -> None:
        """Should raise GitNotFoundError for missing ref.

        Rationale: Branch or tag might not exist. User needs to check
        available refs in the repository.
        """
        # Setup: Mock successful fetch but failed checkout
        def side_effect(*args, **kwargs):
            cmd = args[0]
            if "fetch" in cmd:
                # Fetch succeeds
                return MagicMock(returncode=0, stderr="", stdout="")
            else:
                # Checkout fails - ref not found
                return MagicMock(
                    returncode=1,
                    stderr="error: pathspec 'nonexistent-branch' did not match any file(s) known to git",
                    stdout="",
                )

        mock_run.side_effect = side_effect

        git = SubprocessGitOperations()

        # Exercise & Verify
        with pytest.raises(GitNotFoundError) as exc_info:
            git.fetch("/tmp/existing-repo", "nonexistent-branch")

        assert exc_info.value.ref == "nonexistent-branch"

    @patch("graft.adapters.git.subprocess.run")
    def test_fetch_authentication_error_detected(self, mock_run: MagicMock) -> None:
        """Should raise GitAuthenticationError for fetch auth failures.

        Rationale: Permissions might change after initial clone.
        User needs to verify access.
        """
        # Setup: Mock fetch failure with auth error
        mock_run.return_value = MagicMock(
            returncode=128,
            stderr="Permission denied (publickey).",
            stdout="",
        )

        git = SubprocessGitOperations()

        # Exercise & Verify
        with pytest.raises(GitAuthenticationError) as exc_info:
            git.fetch("/tmp/existing-repo", "main")

        assert "SSH keys" in exc_info.value.suggestion

    @patch("graft.adapters.git.subprocess.run")
    def test_checkout_failure_after_fetch(self, mock_run: MagicMock) -> None:
        """Should raise GitFetchError for checkout failures.

        Rationale: Fetch might succeed but checkout can fail for various reasons
        (dirty working tree, etc.). Error should include git's message.
        """
        # Setup: Mock successful fetch but failed checkout
        def side_effect(*args, **kwargs):
            cmd = args[0]
            if "fetch" in cmd:
                return MagicMock(returncode=0, stderr="", stdout="")
            else:
                return MagicMock(
                    returncode=1,
                    stderr="error: Your local changes would be overwritten",
                    stdout="",
                )

        mock_run.side_effect = side_effect

        git = SubprocessGitOperations()

        # Exercise & Verify
        with pytest.raises(GitFetchError) as exc_info:
            git.fetch("/tmp/existing-repo", "main")

        assert "Checkout failed" in exc_info.value.stderr
        assert "overwritten" in exc_info.value.stderr


class TestIsRepository:
    """Tests for repository detection.

    Rationale: We need to distinguish between existing repos and non-git directories.
    This determines whether to clone or fetch.
    """

    def test_detects_git_repository(self, tmp_path):
        """Should return True for directories containing .git.

        Rationale: Standard git repositories have a .git directory.
        """
        # Setup: Create fake .git directory
        git_dir = tmp_path / ".git"
        git_dir.mkdir()

        git = SubprocessGitOperations()

        # Exercise & Verify
        assert git.is_repository(str(tmp_path)) is True

    def test_detects_non_git_directory(self, tmp_path):
        """Should return False for directories without .git.

        Rationale: Regular directories should trigger clone, not fetch.
        """
        git = SubprocessGitOperations()

        # Exercise & Verify
        assert git.is_repository(str(tmp_path)) is False

    def test_detects_missing_directory(self, tmp_path):
        """Should return False for non-existent paths.

        Rationale: Missing directories should trigger clone.
        """
        git = SubprocessGitOperations()
        non_existent = tmp_path / "does-not-exist"

        # Exercise & Verify
        assert git.is_repository(str(non_existent)) is False
