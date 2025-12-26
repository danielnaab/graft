"""Pytest configuration and fixtures.

Shared fixtures available to all tests.
"""

import pytest

from graft.domain.entities import Entity
from graft.services.context import ServiceContext
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
