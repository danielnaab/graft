"""Integration tests for submodule workflows.

End-to-end tests for git submodule functionality.
"""

import os
import subprocess
import tempfile
from datetime import UTC, datetime
from pathlib import Path

import pytest

from graft.adapters.filesystem import RealFilesystem
from graft.adapters.git import SubprocessGitOperations
from graft.domain.config import GraftConfig
from graft.domain.dependency import DependencySpec, DependencyStatus, GitRef, GitUrl
from graft.domain.lock_entry import LockEntry
from graft.services import resolution_service, sync_service
from graft.services.dependency_context import DependencyContext


@pytest.fixture(scope="module", autouse=True)
def enable_file_protocol():
    """Enable file:// protocol for git submodules in tests.

    This is a module-scoped fixture that saves and restores the global
    git config to avoid polluting the user's environment.
    """
    # Check current value
    result = subprocess.run(
        ["git", "config", "--global", "protocol.file.allow"],
        capture_output=True,
        text=True,
        check=False,
    )
    original_value = result.stdout.strip() if result.returncode == 0 else None

    # Set to 'always' for tests
    subprocess.run(
        ["git", "config", "--global", "protocol.file.allow", "always"],
        check=True,
    )

    yield

    # Restore original value
    if original_value is None:
        subprocess.run(
            ["git", "config", "--global", "--unset", "protocol.file.allow"],
            check=False,
        )
    else:
        subprocess.run(
            ["git", "config", "--global", "protocol.file.allow", original_value],
            check=False,
        )


def _create_test_repo(path: Path, branch: str = "main") -> str:
    """Create a test git repository with one commit.

    Args:
        path: Where to create the repo
        branch: Branch name to use

    Returns:
        The commit hash
    """
    path.mkdir(parents=True, exist_ok=True)
    subprocess.run(["git", "init", "-b", branch], cwd=path, check=True, capture_output=True)
    subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=path, check=True)
    subprocess.run(["git", "config", "user.name", "Test User"], cwd=path, check=True)

    (path / "README.md").write_text(f"# Test repo at {path.name}")
    subprocess.run(["git", "add", "."], cwd=path, check=True)
    subprocess.run(["git", "commit", "-m", "Initial commit"], cwd=path, check=True, capture_output=True)

    result = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=path,
        capture_output=True,
        text=True,
        check=True,
    )
    return result.stdout.strip()


def _init_project_repo(path: Path) -> None:
    """Initialize a project directory as a git repo."""
    path.mkdir(parents=True, exist_ok=True)
    subprocess.run(["git", "init"], cwd=path, check=True, capture_output=True)
    subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=path, check=True)
    subprocess.run(["git", "config", "user.name", "Test User"], cwd=path, check=True)


class TestSubmoduleWorkflow:
    """Test submodule workflow with real git repositories."""

    def test_resolve_adds_as_submodule(self):
        """Test that resolve adds dependencies as submodules."""
        with tempfile.TemporaryDirectory() as tmpdir:
            tmpdir = Path(tmpdir)

            # Create dependency repo
            dep_repo = tmpdir / "dep-repo"
            _create_test_repo(dep_repo)

            # Create project
            project_dir = tmpdir / "project"
            _init_project_repo(project_dir)

            original_cwd = os.getcwd()
            os.chdir(project_dir)

            try:
                ctx = DependencyContext(
                    filesystem=RealFilesystem(),
                    git=SubprocessGitOperations(),
                    deps_directory=".graft",
                )

                spec = DependencySpec(
                    name="test-dep",
                    git_url=GitUrl(f"file://{dep_repo}"),
                    git_ref=GitRef("main"),
                )

                resolution = resolution_service.resolve_dependency(ctx, spec)

                assert resolution.status == DependencyStatus.RESOLVED, resolution.error_message
                assert ctx.git.is_submodule(".graft/test-dep")
                assert (project_dir / ".gitmodules").exists()
                assert (project_dir / ".graft" / "test-dep" / "README.md").exists()

            finally:
                os.chdir(original_cwd)

    def test_resolve_multiple_dependencies(self):
        """Test resolving multiple dependencies as submodules."""
        with tempfile.TemporaryDirectory() as tmpdir:
            tmpdir = Path(tmpdir)

            # Create multiple dependency repos
            dep1 = tmpdir / "dep1"
            dep2 = tmpdir / "dep2"
            dep3 = tmpdir / "dep3"

            commit1 = _create_test_repo(dep1)
            commit2 = _create_test_repo(dep2)
            commit3 = _create_test_repo(dep3)

            # Create project
            project_dir = tmpdir / "project"
            _init_project_repo(project_dir)

            original_cwd = os.getcwd()
            os.chdir(project_dir)

            try:
                ctx = DependencyContext(
                    filesystem=RealFilesystem(),
                    git=SubprocessGitOperations(),
                    deps_directory=".graft",
                )

                config = GraftConfig(
                    api_version="graft/v0",
                    dependencies={
                        "dep1": DependencySpec(
                            name="dep1",
                            git_url=GitUrl(f"file://{dep1}"),
                            git_ref=GitRef("main"),
                        ),
                        "dep2": DependencySpec(
                            name="dep2",
                            git_url=GitUrl(f"file://{dep2}"),
                            git_ref=GitRef("main"),
                        ),
                        "dep3": DependencySpec(
                            name="dep3",
                            git_url=GitUrl(f"file://{dep3}"),
                            git_ref=GitRef("main"),
                        ),
                    },
                    changes={},
                    commands={},
                    metadata={},
                )

                entries = resolution_service.resolve_to_lock_entries(ctx, config)

                # All three should be resolved
                assert len(entries) == 3
                assert "dep1" in entries
                assert "dep2" in entries
                assert "dep3" in entries

                # All should be submodules
                assert ctx.git.is_submodule(".graft/dep1")
                assert ctx.git.is_submodule(".graft/dep2")
                assert ctx.git.is_submodule(".graft/dep3")

                # Commits should match
                assert entries["dep1"].commit == commit1
                assert entries["dep2"].commit == commit2
                assert entries["dep3"].commit == commit3

            finally:
                os.chdir(original_cwd)

    def test_sync_updates_submodule_to_lock_commit(self):
        """Test that sync updates submodule to locked commit."""
        with tempfile.TemporaryDirectory() as tmpdir:
            tmpdir = Path(tmpdir)

            # Create dependency repo with two commits
            dep_repo = tmpdir / "dep-repo"
            dep_repo.mkdir()
            subprocess.run(["git", "init"], cwd=dep_repo, check=True, capture_output=True)
            subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=dep_repo, check=True)
            subprocess.run(["git", "config", "user.name", "Test User"], cwd=dep_repo, check=True)

            # First commit
            (dep_repo / "file1.txt").write_text("version 1")
            subprocess.run(["git", "add", "."], cwd=dep_repo, check=True)
            subprocess.run(["git", "commit", "-m", "First commit"], cwd=dep_repo, check=True, capture_output=True)

            result = subprocess.run(
                ["git", "rev-parse", "HEAD"],
                cwd=dep_repo,
                capture_output=True,
                text=True,
                check=True,
            )
            first_commit = result.stdout.strip()

            # Second commit
            (dep_repo / "file2.txt").write_text("version 2")
            subprocess.run(["git", "add", "."], cwd=dep_repo, check=True)
            subprocess.run(["git", "commit", "-m", "Second commit"], cwd=dep_repo, check=True, capture_output=True)

            # Create project
            project_dir = tmpdir / "project"
            _init_project_repo(project_dir)

            original_cwd = os.getcwd()
            os.chdir(project_dir)

            try:
                fs = RealFilesystem()
                git = SubprocessGitOperations()

                # Add as submodule at HEAD (second commit)
                git.add_submodule(f"file://{dep_repo}", ".graft/test-dep", "main")

                # Create lock entry pointing to first commit
                lock_entry = LockEntry(
                    source=f"file://{dep_repo}",
                    ref="main",
                    commit=first_commit,
                    consumed_at=datetime.now(UTC),
                )

                # Run sync
                result = sync_service.sync_dependency(
                    filesystem=fs,
                    git=git,
                    deps_directory=".graft",
                    name="test-dep",
                    entry=lock_entry,
                )

                assert result.success
                assert result.action == "checked_out"

                # Verify submodule is at first commit
                current = git.get_current_commit(".graft/test-dep")
                assert current == first_commit

                # Verify first file exists but not second
                assert (project_dir / ".graft" / "test-dep" / "file1.txt").exists()
                assert not (project_dir / ".graft" / "test-dep" / "file2.txt").exists()

            finally:
                os.chdir(original_cwd)

    def test_resolve_fails_for_legacy_clone(self):
        """Test that resolve fails when a legacy clone exists."""
        with tempfile.TemporaryDirectory() as tmpdir:
            tmpdir = Path(tmpdir)

            # Create dependency repo
            dep_repo = tmpdir / "dep-repo"
            _create_test_repo(dep_repo)

            # Create project
            project_dir = tmpdir / "project"
            _init_project_repo(project_dir)

            # Create a legacy clone (not a submodule)
            graft_dir = project_dir / ".graft"
            graft_dir.mkdir()
            subprocess.run(
                ["git", "clone", str(dep_repo), "test-dep"],
                cwd=graft_dir,
                check=True,
                capture_output=True,
            )

            original_cwd = os.getcwd()
            os.chdir(project_dir)

            try:
                ctx = DependencyContext(
                    filesystem=RealFilesystem(),
                    git=SubprocessGitOperations(),
                    deps_directory=".graft",
                )

                spec = DependencySpec(
                    name="test-dep",
                    git_url=GitUrl(f"file://{dep_repo}"),
                    git_ref=GitRef("main"),
                )

                resolution = resolution_service.resolve_dependency(ctx, spec)

                # Should fail with clear message
                assert resolution.status == DependencyStatus.FAILED
                assert "Legacy clone detected" in resolution.error_message
                assert "rm -rf" in resolution.error_message

            finally:
                os.chdir(original_cwd)

    def test_resolve_with_tag_ref(self):
        """Test resolving a dependency at a specific tag."""
        with tempfile.TemporaryDirectory() as tmpdir:
            tmpdir = Path(tmpdir)

            # Create dependency repo with a tag
            dep_repo = tmpdir / "dep-repo"
            dep_repo.mkdir()
            subprocess.run(["git", "init"], cwd=dep_repo, check=True, capture_output=True)
            subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=dep_repo, check=True)
            subprocess.run(["git", "config", "user.name", "Test User"], cwd=dep_repo, check=True)

            (dep_repo / "v1.txt").write_text("version 1")
            subprocess.run(["git", "add", "."], cwd=dep_repo, check=True)
            subprocess.run(["git", "commit", "-m", "v1.0.0"], cwd=dep_repo, check=True, capture_output=True)
            subprocess.run(["git", "tag", "v1.0.0"], cwd=dep_repo, check=True)

            result = subprocess.run(
                ["git", "rev-parse", "v1.0.0"],
                cwd=dep_repo,
                capture_output=True,
                text=True,
                check=True,
            )
            tag_commit = result.stdout.strip()

            # Add another commit after the tag
            (dep_repo / "v2.txt").write_text("version 2")
            subprocess.run(["git", "add", "."], cwd=dep_repo, check=True)
            subprocess.run(["git", "commit", "-m", "v2.0.0"], cwd=dep_repo, check=True, capture_output=True)

            # Create project
            project_dir = tmpdir / "project"
            _init_project_repo(project_dir)

            original_cwd = os.getcwd()
            os.chdir(project_dir)

            try:
                ctx = DependencyContext(
                    filesystem=RealFilesystem(),
                    git=SubprocessGitOperations(),
                    deps_directory=".graft",
                )

                # Resolve at the tag, not HEAD
                spec = DependencySpec(
                    name="test-dep",
                    git_url=GitUrl(f"file://{dep_repo}"),
                    git_ref=GitRef("v1.0.0"),
                )

                resolution = resolution_service.resolve_dependency(ctx, spec)

                assert resolution.status == DependencyStatus.RESOLVED, resolution.error_message

                # Should be at the tag commit, not HEAD
                current = ctx.git.get_current_commit(".graft/test-dep")
                assert current == tag_commit

                # v1 file should exist, v2 should not
                assert (project_dir / ".graft" / "test-dep" / "v1.txt").exists()
                assert not (project_dir / ".graft" / "test-dep" / "v2.txt").exists()

            finally:
                os.chdir(original_cwd)

    def test_update_existing_submodule(self):
        """Test updating an existing submodule to a new commit."""
        with tempfile.TemporaryDirectory() as tmpdir:
            tmpdir = Path(tmpdir)

            # Create dependency repo with explicit main branch
            dep_repo = tmpdir / "dep-repo"
            dep_repo.mkdir()
            subprocess.run(["git", "init", "-b", "main"], cwd=dep_repo, check=True, capture_output=True)
            subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=dep_repo, check=True)
            subprocess.run(["git", "config", "user.name", "Test User"], cwd=dep_repo, check=True)

            (dep_repo / "v1.txt").write_text("version 1")
            subprocess.run(["git", "add", "."], cwd=dep_repo, check=True)
            subprocess.run(["git", "commit", "-m", "v1"], cwd=dep_repo, check=True, capture_output=True)

            result = subprocess.run(
                ["git", "rev-parse", "HEAD"],
                cwd=dep_repo,
                capture_output=True,
                text=True,
                check=True,
            )
            first_commit = result.stdout.strip()

            # Create project and add submodule
            project_dir = tmpdir / "project"
            _init_project_repo(project_dir)

            original_cwd = os.getcwd()
            os.chdir(project_dir)

            try:
                ctx = DependencyContext(
                    filesystem=RealFilesystem(),
                    git=SubprocessGitOperations(),
                    deps_directory=".graft",
                )

                spec = DependencySpec(
                    name="test-dep",
                    git_url=GitUrl(f"file://{dep_repo}"),
                    git_ref=GitRef("main"),
                )

                # First resolution
                resolution = resolution_service.resolve_dependency(ctx, spec)
                assert resolution.status == DependencyStatus.RESOLVED
                assert ctx.git.get_current_commit(".graft/test-dep") == first_commit

                # Add new commit to dep repo
                os.chdir(original_cwd)
                (dep_repo / "v2.txt").write_text("version 2")
                subprocess.run(["git", "add", "."], cwd=dep_repo, check=True)
                subprocess.run(["git", "commit", "-m", "v2"], cwd=dep_repo, check=True, capture_output=True)

                result = subprocess.run(
                    ["git", "rev-parse", "HEAD"],
                    cwd=dep_repo,
                    capture_output=True,
                    text=True,
                    check=True,
                )
                second_commit = result.stdout.strip()

                # Resolve again - should update
                os.chdir(project_dir)
                resolution = resolution_service.resolve_dependency(ctx, spec)
                assert resolution.status == DependencyStatus.RESOLVED

                # Should now be at second commit
                current = ctx.git.get_current_commit(".graft/test-dep")
                assert current == second_commit
                assert (project_dir / ".graft" / "test-dep" / "v2.txt").exists()

            finally:
                os.chdir(original_cwd)

    def test_submodule_removal(self):
        """Test removing a submodule."""
        with tempfile.TemporaryDirectory() as tmpdir:
            tmpdir = Path(tmpdir)

            # Create dependency repo
            dep_repo = tmpdir / "dep-repo"
            _create_test_repo(dep_repo)

            # Create project
            project_dir = tmpdir / "project"
            _init_project_repo(project_dir)

            original_cwd = os.getcwd()
            os.chdir(project_dir)

            try:
                git = SubprocessGitOperations()

                # Add submodule
                git.add_submodule(f"file://{dep_repo}", ".graft/test-dep", "main")
                assert git.is_submodule(".graft/test-dep")
                assert (project_dir / ".gitmodules").exists()

                # Remove submodule
                git.remove_submodule(".graft/test-dep")

                # Should no longer be a submodule
                assert not git.is_submodule(".graft/test-dep")
                # Directory should be gone
                assert not (project_dir / ".graft" / "test-dep").exists()

            finally:
                os.chdir(original_cwd)


class TestCloneBasedResolution:
    """Test clone-based resolution when deps_directory is outside repo."""

    def test_resolve_uses_clone_when_outside_repo(self):
        """Test that resolution uses clones when deps_directory is outside the repo."""
        with tempfile.TemporaryDirectory() as tmpdir:
            tmpdir = Path(tmpdir)

            # Create dependency repo
            dep_repo = tmpdir / "dep-repo"
            commit = _create_test_repo(dep_repo)

            # Create project in a subdirectory
            project_dir = tmpdir / "projects" / "myproject"
            _init_project_repo(project_dir)

            # deps_directory is outside the project repo (parent dir)
            deps_dir = tmpdir / "shared-deps"
            deps_dir.mkdir()

            original_cwd = os.getcwd()
            os.chdir(project_dir)

            try:
                ctx = DependencyContext(
                    filesystem=RealFilesystem(),
                    git=SubprocessGitOperations(),
                    deps_directory=str(deps_dir),
                )

                spec = DependencySpec(
                    name="test-dep",
                    git_url=GitUrl(f"file://{dep_repo}"),
                    git_ref=GitRef("main"),
                )

                resolution = resolution_service.resolve_dependency(ctx, spec)

                assert resolution.status == DependencyStatus.RESOLVED, resolution.error_message

                # Should be a clone, not a submodule (since deps_dir is outside repo)
                assert not ctx.git.is_submodule(str(deps_dir / "test-dep"))
                assert ctx.git.is_repository(str(deps_dir / "test-dep"))

                # Files should exist
                assert (deps_dir / "test-dep" / "README.md").exists()

            finally:
                os.chdir(original_cwd)
