"""Unit tests for git submodule operations."""

import pytest

from graft.domain.exceptions import GitCloneError
from tests.fakes.fake_git import FakeGitOperations


class TestFakeGitSubmoduleOperations:
    """Test submodule operations in FakeGitOperations."""

    def test_add_submodule_tracks_call(self):
        """Test that add_submodule tracks the call."""
        git = FakeGitOperations()

        git.add_submodule("https://example.com/repo.git", ".graft/my-dep", "main")

        assert git.was_submodule_added("https://example.com/repo.git", ".graft/my-dep", "main")
        assert git.get_submodule_count() == 1

    def test_add_submodule_without_ref(self):
        """Test adding submodule without explicit ref."""
        git = FakeGitOperations()

        git.add_submodule("https://example.com/repo.git", ".graft/my-dep")

        assert git.was_submodule_added("https://example.com/repo.git", ".graft/my-dep", None)
        assert git.is_submodule(".graft/my-dep")

    def test_is_submodule_returns_false_for_non_submodule(self):
        """Test is_submodule returns False for non-submodules."""
        git = FakeGitOperations()

        assert not git.is_submodule(".graft/not-a-submodule")

    def test_is_submodule_returns_true_for_submodule(self):
        """Test is_submodule returns True for registered submodules."""
        git = FakeGitOperations()
        git.add_submodule("https://example.com/repo.git", ".graft/my-dep", "main")

        assert git.is_submodule(".graft/my-dep")

    def test_update_submodule_tracks_call(self):
        """Test that update_submodule tracks the call."""
        git = FakeGitOperations()
        git.add_submodule("https://example.com/repo.git", ".graft/my-dep", "main")

        git.update_submodule(".graft/my-dep", init=True, recursive=False)

        calls = git.get_update_submodule_calls()
        assert len(calls) == 1
        assert calls[0] == (".graft/my-dep", True, False)

    def test_update_submodule_with_init_creates_submodule(self):
        """Test update_submodule with init=True creates submodule if missing."""
        git = FakeGitOperations()

        git.update_submodule(".graft/new-dep", init=True)

        assert git.is_submodule(".graft/new-dep")

    def test_remove_submodule_removes_from_registry(self):
        """Test remove_submodule removes from registry."""
        git = FakeGitOperations()
        git.add_submodule("https://example.com/repo.git", ".graft/my-dep", "main")

        git.remove_submodule(".graft/my-dep")

        assert not git.is_submodule(".graft/my-dep")
        assert git.get_submodule_count() == 0

    def test_remove_submodule_tracks_call(self):
        """Test that remove_submodule tracks the call."""
        git = FakeGitOperations()
        git.add_submodule("https://example.com/repo.git", ".graft/my-dep", "main")

        git.remove_submodule(".graft/my-dep")

        calls = git.get_remove_submodule_calls()
        assert calls == [".graft/my-dep"]

    def test_get_submodule_status_returns_info(self):
        """Test get_submodule_status returns correct info."""
        git = FakeGitOperations()
        git.add_submodule("https://example.com/repo.git", ".graft/my-dep", "main")

        status = git.get_submodule_status(".graft/my-dep")

        assert "commit" in status
        assert "branch" in status
        assert "status" in status
        assert status["branch"] == "main"
        assert status["status"] == " "  # Normal status

    def test_get_submodule_status_raises_for_non_submodule(self):
        """Test get_submodule_status raises for non-submodule."""
        git = FakeGitOperations()

        with pytest.raises(ValueError, match="No submodule"):
            git.get_submodule_status(".graft/not-a-submodule")

    def test_set_submodule_branch_updates_branch(self):
        """Test set_submodule_branch updates the branch."""
        git = FakeGitOperations()
        git.add_submodule("https://example.com/repo.git", ".graft/my-dep", "main")

        git.set_submodule_branch(".graft/my-dep", "develop")

        status = git.get_submodule_status(".graft/my-dep")
        assert status["branch"] == "develop"

    def test_set_submodule_branch_raises_for_non_submodule(self):
        """Test set_submodule_branch raises for non-submodule."""
        git = FakeGitOperations()

        with pytest.raises(ValueError, match="No submodule"):
            git.set_submodule_branch(".graft/not-a-submodule", "main")

    def test_sync_submodule_succeeds_for_submodule(self):
        """Test sync_submodule succeeds for valid submodule."""
        git = FakeGitOperations()
        git.add_submodule("https://example.com/repo.git", ".graft/my-dep", "main")

        # Should not raise
        git.sync_submodule(".graft/my-dep")

    def test_sync_submodule_raises_for_non_submodule(self):
        """Test sync_submodule raises for non-submodule."""
        git = FakeGitOperations()

        with pytest.raises(ValueError, match="No submodule"):
            git.sync_submodule(".graft/not-a-submodule")

    def test_deinit_submodule_changes_status(self):
        """Test deinit_submodule changes submodule status."""
        git = FakeGitOperations()
        git.add_submodule("https://example.com/repo.git", ".graft/my-dep", "main")

        git.deinit_submodule(".graft/my-dep")

        status = git.get_submodule_status(".graft/my-dep")
        assert status["status"] == "-"  # Uninitialized

    def test_reset_clears_submodule_state(self):
        """Test reset clears all submodule state."""
        git = FakeGitOperations()
        git.add_submodule("https://example.com/repo.git", ".graft/my-dep", "main")

        git.reset()

        assert git.get_submodule_count() == 0
        assert not git.is_submodule(".graft/my-dep")
        assert len(git.get_add_submodule_calls()) == 0