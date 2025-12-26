"""In-memory repository implementation.

Simple in-memory storage using a dictionary.
Useful for development, testing, and prototyping.
Replace with database implementation for production.
"""

from typing import Generic, Optional, TypeVar

T = TypeVar("T")


class InMemoryRepository(Generic[T]):
    """In-memory repository storing entities in a dict.

    Implements Repository[T] protocol via structural typing.
    No inheritance needed - just implements the required methods.

    Type parameter T represents the entity type.

    Note:
        This is not persistent - data is lost when process ends.
        Use for development/testing only.

    Example:
        >>> from graft.domain.entities import Entity
        >>> repo = InMemoryRepository[Entity]()
        >>> entity = Entity(name=EntityName("test"), value=EntityValue(100))
        >>> repo.save(entity)
        >>> retrieved = repo.get(entity.id)
        >>> retrieved == entity
        True
    """

    def __init__(self) -> None:
        """Initialize empty repository."""
        self._storage: dict[str, T] = {}

    def save(self, entity: T) -> None:
        """Persist entity to in-memory storage.

        Idempotent - saving same entity twice has no additional effect.

        Args:
            entity: Entity to save (must have 'id' attribute)
        """
        # Assumes entity has an 'id' attribute
        entity_id = getattr(entity, "id")
        self._storage[entity_id] = entity

    def get(self, entity_id: str) -> Optional[T]:
        """Retrieve entity by ID.

        Args:
            entity_id: Unique identifier

        Returns:
            Entity if found, None otherwise
        """
        return self._storage.get(entity_id)

    def list_all(self) -> list[T]:
        """Retrieve all entities.

        Returns:
            List of all entities (empty if none)
        """
        return list(self._storage.values())

    def delete(self, entity_id: str) -> bool:
        """Delete entity by ID.

        Args:
            entity_id: Unique identifier

        Returns:
            True if entity was deleted, False if not found
        """
        if entity_id in self._storage:
            del self._storage[entity_id]
            return True
        return False

    def clear(self) -> None:
        """Clear all entities from storage.

        Useful for testing.
        """
        self._storage.clear()
