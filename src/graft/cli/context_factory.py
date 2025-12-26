"""Service context factory for CLI.

Builds production service context with real adapters.
"""

from graft.adapters.repository import InMemoryRepository
from graft.domain.entities import Entity
from graft.services.context import ServiceContext


def get_context() -> ServiceContext:
    """Build production service context.

    Creates ServiceContext with real adapters.
    Modify this to use production implementations (e.g., PostgresRepository).

    Returns:
        ServiceContext with production dependencies

    Example:
        For production, replace InMemoryRepository with real implementation:

        return ServiceContext(
            repository=PostgresRepository(
                connection_string=os.getenv("DATABASE_URL")
            )
        )
    """
    return ServiceContext(
        repository=InMemoryRepository[Entity](),
    )
