"""Pytest configuration and fixtures.

Shared fixtures available to all tests.
"""

import pytest

from graft.domain.entities import Entity
from graft.services.context import ServiceContext
from graft.services.dependency_context import DependencyContext
from tests.fakes.fake_filesystem import FakeFileSystem
from tests.fakes.fake_git import FakeGitOperations
from tests.fakes.fake_repository import FakeRepository


@pytest.fixture
def fake_repository() -> FakeRepository[Entity]:
    """Provide fresh fake repository for each test.

    Automatically reset between tests.

    Returns:
        Empty FakeRepository[Entity]
    """
    repo = FakeRepository[Entity]()
    yield repo
    # Cleanup after test
    repo.reset()


@pytest.fixture
def test_context(fake_repository: FakeRepository[Entity]) -> ServiceContext:
    """Provide test service context with fakes.

    Uses fake implementations instead of real adapters.
    All dependencies injected via context.

    Args:
        fake_repository: Fake repository fixture

    Returns:
        ServiceContext with fake dependencies

    Example:
        def test_my_service(test_context):
            result = my_service(test_context, param="test")
            assert result is not None
    """
    return ServiceContext(
        repository=fake_repository,
    )


@pytest.fixture
def fake_filesystem() -> FakeFileSystem:
    """Provide fresh fake filesystem for each test.

    Automatically reset between tests.

    Returns:
        Empty FakeFileSystem

    Example:
        def test_with_filesystem(fake_filesystem):
            fake_filesystem.create_file("/test.txt", "content")
            assert fake_filesystem.read_text("/test.txt") == "content"
    """
    fs = FakeFileSystem()
    yield fs
    # Cleanup after test
    fs.reset()


@pytest.fixture
def fake_git() -> FakeGitOperations:
    """Provide fresh fake git operations for each test.

    Automatically reset between tests.

    Returns:
        Empty FakeGitOperations

    Example:
        def test_with_git(fake_git):
            fake_git.clone("https://example.com/repo.git", "/tmp/repo", "main")
            assert fake_git.was_cloned("https://example.com/repo.git", "/tmp/repo", "main")
    """
    git = FakeGitOperations()
    yield git
    # Cleanup after test
    git.reset()


@pytest.fixture
def dependency_context(
    fake_filesystem: FakeFileSystem,
    fake_git: FakeGitOperations,
) -> DependencyContext:
    """Provide test dependency context with fakes.

    Uses fake implementations for testing dependency resolution.

    Args:
        fake_filesystem: Fake filesystem fixture
        fake_git: Fake git operations fixture

    Returns:
        DependencyContext with fake dependencies

    Example:
        def test_config_parsing(dependency_context):
            config = parse_graft_yaml(dependency_context, "graft.yaml")
            assert config is not None
    """
    return DependencyContext(
        filesystem=fake_filesystem,
        git=fake_git,
        deps_directory="/fake/deps",
    )
