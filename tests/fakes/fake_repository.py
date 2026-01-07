"""Fake repository for testing.

In-memory implementation with test helpers.
"""

from typing import Generic, TypeVar

T = TypeVar("T")


class FakeRepository(Generic[T]):
    """Fake repository for testing.

    Implements Repository[T] protocol with additional test helpers.
    Stores entities in memory using dict.

    Advantages over mocks:
    - Real behavior (not just assertions)
    - Can inspect state
    - Reusable across tests
    - Clear and maintainable

    Example:
        >>> from graft.domain.entities import Entity
        >>> repo = FakeRepository[Entity]()
        >>> entity = Entity(name=EntityName("test"), value=EntityValue(100))
        >>> repo.save(entity)
        >>> repo.assert_saved(entity.id)  # Test helper
        >>> repo.get_saved_count()  # Test helper
        1
    """

    def __init__(self) -> None:
        """Initialize empty fake repository."""
        self._storage: dict[str, T] = {}
        self._save_calls: list[T] = []
        self._get_calls: list[str] = []
        self._delete_calls: list[str] = []

    def save(self, entity: T) -> None:
        """Save entity and record the call.

        Args:
            entity: Entity to save (must have 'id' attribute)
        """
        entity_id = entity.id
        self._storage[entity_id] = entity
        self._save_calls.append(entity)

    def get(self, entity_id: str) -> T | None:
        """Get entity and record the call.

        Args:
            entity_id: Entity identifier

        Returns:
            Entity if found, None otherwise
        """
        self._get_calls.append(entity_id)
        return self._storage.get(entity_id)

    def list_all(self) -> list[T]:
        """List all entities.

        Returns:
            List of all stored entities
        """
        return list(self._storage.values())

    def delete(self, entity_id: str) -> bool:
        """Delete entity and record the call.

        Args:
            entity_id: Entity identifier

        Returns:
            True if deleted, False if not found
        """
        self._delete_calls.append(entity_id)
        if entity_id in self._storage:
            del self._storage[entity_id]
            return True
        return False

    # Test helpers below

    def reset(self) -> None:
        """Clear all data and call history.

        Useful for cleanup between tests.
        """
        self._storage.clear()
        self._save_calls.clear()
        self._get_calls.clear()
        self._delete_calls.clear()

    def get_saved_count(self) -> int:
        """Get number of entities currently saved.

        Returns:
            Count of entities in storage
        """
        return len(self._storage)

    def assert_saved(self, entity_id: str) -> None:
        """Assert that entity with ID was saved.

        Args:
            entity_id: Expected entity ID

        Raises:
            AssertionError: If entity not found
        """
        assert entity_id in self._storage, f"Entity {entity_id} not found in repository"

    def assert_not_saved(self, entity_id: str) -> None:
        """Assert that entity with ID was NOT saved.

        Args:
            entity_id: Entity ID that shouldn't exist

        Raises:
            AssertionError: If entity found
        """
        assert entity_id not in self._storage, f"Entity {entity_id} should not be in repository"

    def assert_save_called_with(self, entity: T) -> None:
        """Assert that save was called with specific entity.

        Args:
            entity: Expected entity

        Raises:
            AssertionError: If entity not in save calls
        """
        assert entity in self._save_calls, f"Entity {entity} was not saved"

    def get_save_call_count(self) -> int:
        """Get number of times save() was called.

        Returns:
            Save call count
        """
        return len(self._save_calls)

    def get_get_call_count(self) -> int:
        """Get number of times get() was called.

        Returns:
            Get call count
        """
        return len(self._get_calls)

    def was_get_called_with(self, entity_id: str) -> bool:
        """Check if get() was called with specific ID.

        Args:
            entity_id: Entity ID to check

        Returns:
            True if get() was called with this ID
        """
        return entity_id in self._get_calls
