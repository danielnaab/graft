"""Service context - dependency injection container.

ServiceContext holds all dependencies needed by service functions.
Implemented as a frozen dataclass for immutability and type safety.
"""

from dataclasses import dataclass

from graft.domain.entities import Entity
from graft.protocols.repository import Repository


@dataclass(frozen=True)
class ServiceContext:
    """Service layer dependency container.

    Contains all external dependencies needed by service functions.
    Immutable (frozen=True) to ensure thread safety and predictable behavior.

    Pattern:
        - Production: Inject real adapters (PostgresRepository, HTTPClient, etc.)
        - Testing: Inject fakes (FakeRepository, FakeHTTPClient, etc.)

    Attributes:
        repository: Entity persistence (Protocol-based)

    Example:
        # Production context
        ctx = ServiceContext(
            repository=PostgresRepository(connection_string=os.getenv("DATABASE_URL"))
        )

        # Test context
        ctx = ServiceContext(
            repository=FakeRepository()
        )
    """

    repository: Repository[Entity]
