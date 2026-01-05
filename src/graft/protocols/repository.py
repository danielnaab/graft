"""Repository protocol - generic persistence interface.

Defines interface for entity storage using typing.Protocol for structural typing.
"""

from typing import Generic, Protocol, TypeVar

T = TypeVar("T")


class Repository(Protocol, Generic[T]):
    """Generic repository for entity persistence.

    Any class implementing these methods satisfies this protocol.
    No inheritance required - structural typing (duck typing with type safety).

    Type parameter T represents the entity type being stored.

    Example implementations:
        - InMemoryRepository[Entity]
        - PostgresRepository[User]
        - SQLiteRepository[Product]
    """

    def save(self, entity: T) -> None:
        """Persist entity to storage.

        Implementations should be idempotent - saving same entity twice
        should not create duplicates.

        Args:
            entity: Entity to save
        """
        ...

    def get(self, entity_id: str) -> T | None:
        """Retrieve entity by ID.

        Args:
            entity_id: Unique identifier

        Returns:
            Entity if found, None otherwise
        """
        ...

    def list_all(self) -> list[T]:
        """Retrieve all entities.

        Returns:
            List of all entities (empty list if none)
        """
        ...

    def delete(self, entity_id: str) -> bool:
        """Delete entity by ID.

        Args:
            entity_id: Unique identifier

        Returns:
            True if entity was deleted, False if not found
        """
        ...
