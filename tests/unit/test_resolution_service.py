"""Unit tests for resolution service.

Tests for resolve_dependency and resolve_all_dependencies service functions.

Rationale:
    Resolution service implements our hybrid error handling strategy:
    - Git operations can fail (exceptions)
    - But resolution continues (converts to failed DependencyResolution)
    - This enables batch processing with partial failures

    These tests verify:
    - Successful resolution flow
    - Exception-to-result conversion
    - Batch operations continue on partial failure
"""

import pytest

from graft.domain.config import GraftConfig
from graft.domain.dependency import DependencySpec, DependencyStatus, GitRef, GitUrl
from graft.services import resolution_service
from graft.services.dependency_context import DependencyContext
from tests.fakes.fake_filesystem import FakeFileSystem
from tests.fakes.fake_git import FakeGitOperations


@pytest.fixture
def fake_git() -> FakeGitOperations:
    """Provide fresh fake git operations."""
    git = FakeGitOperations()
    yield git
    git.reset()


@pytest.fixture
def full_dependency_context(
    fake_filesystem: FakeFileSystem,
    fake_git: FakeGitOperations,
) -> DependencyContext:
    """Provide dependency context with git operations."""
    return DependencyContext(
        filesystem=fake_filesystem,
        git=fake_git,
        deps_directory="/fake/deps",
    )


class TestResolveDependency:
    """Tests for resolve_dependency service function.

    Rationale: This service demonstrates our hybrid error handling.
    Git exceptions are caught and converted to DependencyResolution.FAILED,
    allowing callers to handle errors functionally.
    """

    def test_clone_new_dependency(
        self,
        full_dependency_context: DependencyContext,
        fake_git: FakeGitOperations,
    ) -> None:
        """Should clone dependency that doesn't exist.

        Rationale: Happy path - directory doesn't exist, so clone.
        This is the most common scenario for first-time dependency resolution.
        """
        # Setup
        spec = DependencySpec(
            name="test-repo",
            git_url=GitUrl("https://github.com/user/repo.git"),
            git_ref=GitRef("main"),
        )

        # Exercise
        resolution = resolution_service.resolve_dependency(full_dependency_context, spec)

        # Verify
        assert resolution.status == DependencyStatus.RESOLVED
        assert resolution.local_path == "/fake/deps/test-repo"
        assert fake_git.was_cloned(
            "https://github.com/user/repo.git",
            "/fake/deps/test-repo",
            "main",
        )

    def test_fetch_existing_dependency(
        self,
        full_dependency_context: DependencyContext,
        fake_git: FakeGitOperations,
        fake_filesystem: FakeFileSystem,
    ) -> None:
        """Should fetch dependency that already exists.

        Rationale: When re-running graft resolve, we should update
        existing repositories rather than re-cloning.
        """
        # Setup
        spec = DependencySpec(
            name="test-repo",
            git_url=GitUrl("https://github.com/user/repo.git"),
            git_ref=GitRef("develop"),
        )

        # Simulate existing repo
        fake_filesystem.mkdir("/fake/deps/test-repo")
        fake_git._cloned_repos["/fake/deps/test-repo"] = (
            "https://github.com/user/repo.git",
            "main",
        )

        # Exercise
        resolution = resolution_service.resolve_dependency(full_dependency_context, spec)

        # Verify
        assert resolution.status == DependencyStatus.RESOLVED
        assert fake_git.get_fetch_count() == 1
        assert fake_git.get_clone_count() == 0

    def test_failed_clone_marks_resolution_failed(
        self,
        full_dependency_context: DependencyContext,
        fake_git: FakeGitOperations,
    ) -> None:
        """Should mark resolution as failed if clone fails.

        Rationale: Key behavior - exceptions are converted to failed results.
        This allows batch processing to continue and report all failures.
        Error message should be preserved for user feedback.
        """
        # Setup
        spec = DependencySpec(
            name="test-repo",
            git_url=GitUrl("https://github.com/user/repo.git"),
            git_ref=GitRef("main"),
        )

        # Configure git to fail
        fake_git.configure_failure(
            "https://github.com/user/repo.git",
            "Network error",
        )

        # Exercise
        resolution = resolution_service.resolve_dependency(full_dependency_context, spec)

        # Verify
        assert resolution.status == DependencyStatus.FAILED
        assert "Network error" in resolution.error_message

    def test_non_git_directory_marks_resolution_failed(
        self,
        full_dependency_context: DependencyContext,
        fake_filesystem: FakeFileSystem,
    ) -> None:
        """Should fail if path exists but is not a git repository.

        Rationale: User might have manually created directory or it's from
        another source. We can't safely clone over it or fetch into it.
        Error message should explain the problem.
        """
        # Setup
        spec = DependencySpec(
            name="test-repo",
            git_url=GitUrl("https://github.com/user/repo.git"),
            git_ref=GitRef("main"),
        )

        # Create non-git directory
        fake_filesystem.mkdir("/fake/deps/test-repo")

        # Exercise
        resolution = resolution_service.resolve_dependency(full_dependency_context, spec)

        # Verify
        assert resolution.status == DependencyStatus.FAILED
        assert "not a git repository" in resolution.error_message

    def test_resolution_sets_local_path(
        self,
        full_dependency_context: DependencyContext,
    ) -> None:
        """Should set local_path on successful resolution."""
        # Setup
        spec = DependencySpec(
            name="graft-knowledge",
            git_url=GitUrl("ssh://git@example.com/repo.git"),
            git_ref=GitRef("main"),
        )

        # Exercise
        resolution = resolution_service.resolve_dependency(full_dependency_context, spec)

        # Verify
        assert resolution.local_path == "/fake/deps/graft-knowledge"

    def test_resolution_uses_dependency_name_for_path(
        self,
        full_dependency_context: DependencyContext,
        fake_git: FakeGitOperations,
    ) -> None:
        """Should use dependency name as directory name."""
        # Setup
        spec = DependencySpec(
            name="my-custom-name",
            git_url=GitUrl("https://github.com/user/some-repo.git"),
            git_ref=GitRef("v1.0.0"),
        )

        # Exercise
        resolution_service.resolve_dependency(full_dependency_context, spec)

        # Verify
        assert "/fake/deps/my-custom-name" in fake_git._cloned_repos


class TestResolveAllDependencies:
    """Tests for resolve_all_dependencies service function.

    Rationale: Batch operation demonstrating partial failure handling.
    If one dependency fails, others should still be attempted.
    """

    def test_resolve_single_dependency(
        self,
        full_dependency_context: DependencyContext,
    ) -> None:
        """Should resolve single dependency.

        Rationale: Basic case - single dependency resolves successfully.
        """
        # Setup
        spec = DependencySpec(
            name="dep1",
            git_url=GitUrl("https://github.com/user/repo1.git"),
            git_ref=GitRef("main"),
        )
        config = GraftConfig(
            api_version="graft/v0",
            dependencies={"dep1": spec},
        )

        # Exercise
        resolutions = resolution_service.resolve_all_dependencies(
            full_dependency_context, config
        )

        # Verify
        assert len(resolutions) == 1
        assert resolutions[0].status == DependencyStatus.RESOLVED
        assert resolutions[0].name == "dep1"

    def test_resolve_multiple_dependencies(
        self,
        full_dependency_context: DependencyContext,
    ) -> None:
        """Should resolve all dependencies.

        Rationale: Multiple dependencies should all resolve successfully.
        Verifies we process all deps from config.
        """
        # Setup
        spec1 = DependencySpec(
            name="dep1",
            git_url=GitUrl("https://github.com/user/repo1.git"),
            git_ref=GitRef("main"),
        )
        spec2 = DependencySpec(
            name="dep2",
            git_url=GitUrl("https://github.com/user/repo2.git"),
            git_ref=GitRef("develop"),
        )
        spec3 = DependencySpec(
            name="dep3",
            git_url=GitUrl("ssh://git@example.com/repo3.git"),
            git_ref=GitRef("v1.0.0"),
        )

        config = GraftConfig(
            api_version="graft/v0",
            dependencies={"dep1": spec1, "dep2": spec2, "dep3": spec3},
        )

        # Exercise
        resolutions = resolution_service.resolve_all_dependencies(
            full_dependency_context, config
        )

        # Verify
        assert len(resolutions) == 3
        assert all(r.status == DependencyStatus.RESOLVED for r in resolutions)
        assert {r.name for r in resolutions} == {"dep1", "dep2", "dep3"}

    def test_continue_on_partial_failure(
        self,
        full_dependency_context: DependencyContext,
        fake_git: FakeGitOperations,
    ) -> None:
        """Should continue resolving after one dependency fails.

        Rationale: CRITICAL behavior - demonstrates hybrid error strategy.
        One failed dependency should NOT prevent others from resolving.
        User gets complete picture of what succeeded and what failed.

        This enables:
        - Fixing one dependency without re-running all
        - Understanding scope of issues
        - Partial progress is preserved
        """
        # Setup
        spec1 = DependencySpec(
            name="dep1",
            git_url=GitUrl("https://github.com/user/repo1.git"),
            git_ref=GitRef("main"),
        )
        spec2 = DependencySpec(
            name="dep2",
            git_url=GitUrl("https://github.com/user/repo2.git"),
            git_ref=GitRef("main"),
        )
        spec3 = DependencySpec(
            name="dep3",
            git_url=GitUrl("https://github.com/user/repo3.git"),
            git_ref=GitRef("main"),
        )

        # Configure dep2 to fail
        fake_git.configure_failure(
            "https://github.com/user/repo2.git",
            "Network error",
        )

        config = GraftConfig(
            api_version="graft/v0",
            dependencies={"dep1": spec1, "dep2": spec2, "dep3": spec3},
        )

        # Exercise
        resolutions = resolution_service.resolve_all_dependencies(
            full_dependency_context, config
        )

        # Verify
        assert len(resolutions) == 3
        assert resolutions[0].status == DependencyStatus.RESOLVED
        assert resolutions[1].status == DependencyStatus.FAILED
        assert resolutions[2].status == DependencyStatus.RESOLVED

    def test_empty_dependencies(
        self,
        full_dependency_context: DependencyContext,
    ) -> None:
        """Should handle empty dependencies.

        Rationale: Edge case - graft.yaml with no dependencies.
        Should succeed gracefully, not error.
        """
        # Setup
        config = GraftConfig(
            api_version="graft/v0",
            dependencies={},
        )

        # Exercise
        resolutions = resolution_service.resolve_all_dependencies(
            full_dependency_context, config
        )

        # Verify
        assert len(resolutions) == 0
