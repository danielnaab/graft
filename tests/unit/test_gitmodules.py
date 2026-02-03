"""Unit tests for GitModulesFile class."""

import pytest

from graft.adapters.gitmodules import GitModulesFile
from graft.domain.gitmodule import GitModuleEntry
from tests.fakes.fake_filesystem import FakeFileSystem


class TestGitModulesFile:
    """Test GitModulesFile operations."""

    def test_read_empty_file_returns_empty_dict(self):
        """Test reading non-existent .gitmodules returns empty dict."""
        fs = FakeFileSystem()
        gitmodules = GitModulesFile(fs)

        entries = gitmodules.read_gitmodules()

        assert entries == {}

    def test_write_and_read_single_entry(self):
        """Test writing and reading a single submodule entry."""
        fs = FakeFileSystem()
        gitmodules = GitModulesFile(fs)

        entry = GitModuleEntry(
            name="my-dep",
            path=".graft/my-dep",
            url="https://github.com/user/repo.git",
            branch="main",
        )

        gitmodules.write_gitmodules({"my-dep": entry})
        entries = gitmodules.read_gitmodules()

        assert len(entries) == 1
        assert "my-dep" in entries
        assert entries["my-dep"].path == ".graft/my-dep"
        assert entries["my-dep"].url == "https://github.com/user/repo.git"
        assert entries["my-dep"].branch == "main"

    def test_write_and_read_multiple_entries(self):
        """Test writing and reading multiple submodule entries."""
        fs = FakeFileSystem()
        gitmodules = GitModulesFile(fs)

        entries_to_write = {
            "dep1": GitModuleEntry(
                name="dep1",
                path=".graft/dep1",
                url="https://github.com/user/dep1.git",
                branch="main",
            ),
            "dep2": GitModuleEntry(
                name="dep2",
                path=".graft/dep2",
                url="https://github.com/user/dep2.git",
                branch=None,  # No branch specified
            ),
        }

        gitmodules.write_gitmodules(entries_to_write)
        entries = gitmodules.read_gitmodules()

        assert len(entries) == 2
        assert "dep1" in entries
        assert "dep2" in entries
        assert entries["dep1"].branch == "main"
        assert entries["dep2"].branch is None

    def test_add_entry_to_empty_file(self):
        """Test adding entry to empty .gitmodules."""
        fs = FakeFileSystem()
        gitmodules = GitModulesFile(fs)

        gitmodules.add_entry(
            "my-dep",
            ".graft/my-dep",
            "https://github.com/user/repo.git",
            "develop",
        )

        entries = gitmodules.read_gitmodules()
        assert len(entries) == 1
        assert entries["my-dep"].branch == "develop"

    def test_add_entry_to_existing_file(self):
        """Test adding entry to existing .gitmodules."""
        fs = FakeFileSystem()
        gitmodules = GitModulesFile(fs)

        # Add first entry
        gitmodules.add_entry(
            "dep1",
            ".graft/dep1",
            "https://github.com/user/dep1.git",
            "main",
        )

        # Add second entry
        gitmodules.add_entry(
            "dep2",
            ".graft/dep2",
            "https://github.com/user/dep2.git",
            "develop",
        )

        entries = gitmodules.read_gitmodules()
        assert len(entries) == 2
        assert "dep1" in entries
        assert "dep2" in entries

    def test_add_entry_updates_existing(self):
        """Test adding entry with same name updates existing."""
        fs = FakeFileSystem()
        gitmodules = GitModulesFile(fs)

        # Add initial entry
        gitmodules.add_entry(
            "my-dep",
            ".graft/my-dep",
            "https://github.com/user/repo.git",
            "main",
        )

        # Update with different URL and branch
        gitmodules.add_entry(
            "my-dep",
            ".graft/my-dep",
            "https://github.com/other/repo.git",
            "develop",
        )

        entries = gitmodules.read_gitmodules()
        assert len(entries) == 1
        assert entries["my-dep"].url == "https://github.com/other/repo.git"
        assert entries["my-dep"].branch == "develop"

    def test_remove_entry(self):
        """Test removing a submodule entry."""
        fs = FakeFileSystem()
        gitmodules = GitModulesFile(fs)

        # Add entries
        gitmodules.add_entry("dep1", ".graft/dep1", "url1", "main")
        gitmodules.add_entry("dep2", ".graft/dep2", "url2", "main")

        # Remove one
        gitmodules.remove_entry("dep1")

        entries = gitmodules.read_gitmodules()
        assert len(entries) == 1
        assert "dep1" not in entries
        assert "dep2" in entries

    def test_remove_nonexistent_entry_is_noop(self):
        """Test removing non-existent entry is no-op."""
        fs = FakeFileSystem()
        gitmodules = GitModulesFile(fs)

        gitmodules.add_entry("dep1", ".graft/dep1", "url1", "main")

        # Remove non-existent - should not raise
        gitmodules.remove_entry("non-existent")

        entries = gitmodules.read_gitmodules()
        assert len(entries) == 1
        assert "dep1" in entries

    def test_update_entry_branch(self):
        """Test updating just the branch of an entry."""
        fs = FakeFileSystem()
        gitmodules = GitModulesFile(fs)

        gitmodules.add_entry(
            "my-dep",
            ".graft/my-dep",
            "https://github.com/user/repo.git",
            "main",
        )

        gitmodules.update_entry("my-dep", branch="develop")

        entries = gitmodules.read_gitmodules()
        assert entries["my-dep"].branch == "develop"
        assert entries["my-dep"].url == "https://github.com/user/repo.git"  # Unchanged

    def test_update_entry_url(self):
        """Test updating just the URL of an entry."""
        fs = FakeFileSystem()
        gitmodules = GitModulesFile(fs)

        gitmodules.add_entry(
            "my-dep",
            ".graft/my-dep",
            "https://github.com/user/repo.git",
            "main",
        )

        gitmodules.update_entry("my-dep", url="https://github.com/other/repo.git")

        entries = gitmodules.read_gitmodules()
        assert entries["my-dep"].url == "https://github.com/other/repo.git"
        assert entries["my-dep"].branch == "main"  # Unchanged

    def test_update_nonexistent_entry_raises(self):
        """Test updating non-existent entry raises KeyError."""
        fs = FakeFileSystem()
        gitmodules = GitModulesFile(fs)

        with pytest.raises(KeyError, match="not found"):
            gitmodules.update_entry("non-existent", branch="main")

    def test_entry_exists_returns_true_for_existing(self):
        """Test entry_exists returns True for existing entries."""
        fs = FakeFileSystem()
        gitmodules = GitModulesFile(fs)

        gitmodules.add_entry("my-dep", ".graft/my-dep", "url", "main")

        assert gitmodules.entry_exists("my-dep")

    def test_entry_exists_returns_false_for_nonexistent(self):
        """Test entry_exists returns False for non-existent entries."""
        fs = FakeFileSystem()
        gitmodules = GitModulesFile(fs)

        assert not gitmodules.entry_exists("non-existent")

    def test_custom_path(self):
        """Test using custom path for .gitmodules file."""
        fs = FakeFileSystem()
        gitmodules = GitModulesFile(fs, path="custom/.gitmodules")

        gitmodules.add_entry("my-dep", ".graft/my-dep", "url", "main")

        # Verify it was written to custom path
        assert fs.exists("custom/.gitmodules")
        assert not fs.exists(".gitmodules")


class TestGitModuleEntry:
    """Test GitModuleEntry domain model."""

    def test_to_gitmodules_format_with_branch(self):
        """Test converting entry to .gitmodules format with branch."""
        entry = GitModuleEntry(
            name="my-dep",
            path=".graft/my-dep",
            url="https://github.com/user/repo.git",
            branch="main",
        )

        result = entry.to_gitmodules_format()

        expected = (
            '[submodule "my-dep"]\n'
            '\tpath = .graft/my-dep\n'
            '\turl = https://github.com/user/repo.git\n'
            '\tbranch = main'
        )
        assert result == expected

    def test_to_gitmodules_format_without_branch(self):
        """Test converting entry to .gitmodules format without branch."""
        entry = GitModuleEntry(
            name="my-dep",
            path=".graft/my-dep",
            url="https://github.com/user/repo.git",
            branch=None,
        )

        result = entry.to_gitmodules_format()

        expected = (
            '[submodule "my-dep"]\n'
            '\tpath = .graft/my-dep\n'
            '\turl = https://github.com/user/repo.git'
        )
        assert result == expected

    def test_entry_is_immutable(self):
        """Test that GitModuleEntry is immutable (frozen)."""
        entry = GitModuleEntry(
            name="my-dep",
            path=".graft/my-dep",
            url="https://github.com/user/repo.git",
            branch="main",
        )

        with pytest.raises(AttributeError):
            entry.name = "other-name"  # type: ignore