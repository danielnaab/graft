"""Integration tests for adapters.

Tests using real adapter implementations (not fakes).
"""

import pytest

from graft.adapters.repository import InMemoryRepository
from graft.domain.entities import Entity
from graft.domain.value_objects import EntityName, EntityValue


class TestInMemoryRepository:
    """Integration tests for InMemoryRepository."""

    @pytest.fixture
    def repository(self) -> InMemoryRepository[Entity]:
        """Provide fresh repository for each test."""
        return InMemoryRepository[Entity]()

    def test_save_and_get_entity(
        self,
        repository: InMemoryRepository[Entity],
    ) -> None:
        """Should save and retrieve entity."""
        # Create entity
        entity = Entity(
            name=EntityName(text="Test"),
            value=EntityValue(amount=100),
        )

        # Save
        repository.save(entity)

        # Retrieve
        retrieved = repository.get(entity.id)

        assert retrieved == entity
        assert retrieved is entity  # Same object

    def test_get_nonexistent_returns_none(
        self,
        repository: InMemoryRepository[Entity],
    ) -> None:
        """Should return None for nonexistent ID."""
        result = repository.get("nonexistent-id")
        assert result is None

    def test_list_all_when_empty(
        self,
        repository: InMemoryRepository[Entity],
    ) -> None:
        """Should return empty list when repository is empty."""
        entities = repository.list_all()
        assert entities == []

    def test_list_all_with_entities(
        self,
        repository: InMemoryRepository[Entity],
    ) -> None:
        """Should return all saved entities."""
        # Create and save multiple entities
        entities = [
            Entity(name=EntityName(text=f"Entity {i}"), value=EntityValue(amount=i * 10))
            for i in range(3)
        ]

        for entity in entities:
            repository.save(entity)

        # Retrieve all
        all_entities = repository.list_all()

        assert len(all_entities) == 3
        for entity in entities:
            assert entity in all_entities

    def test_delete_existing_entity(
        self,
        repository: InMemoryRepository[Entity],
    ) -> None:
        """Should delete existing entity."""
        # Create and save entity
        entity = Entity(
            name=EntityName(text="Test"),
            value=EntityValue(amount=100),
        )
        repository.save(entity)

        # Delete
        result = repository.delete(entity.id)

        assert result is True
        assert repository.get(entity.id) is None

    def test_delete_nonexistent_returns_false(
        self,
        repository: InMemoryRepository[Entity],
    ) -> None:
        """Should return False when deleting nonexistent entity."""
        result = repository.delete("nonexistent-id")
        assert result is False

    def test_save_is_idempotent(
        self,
        repository: InMemoryRepository[Entity],
    ) -> None:
        """Should handle saving same entity multiple times."""
        entity = Entity(
            name=EntityName(text="Test"),
            value=EntityValue(amount=100),
        )

        # Save multiple times
        repository.save(entity)
        repository.save(entity)
        repository.save(entity)

        # Should only have one copy
        all_entities = repository.list_all()
        assert len(all_entities) == 1

    def test_clear_removes_all_entities(
        self,
        repository: InMemoryRepository[Entity],
    ) -> None:
        """Should clear all entities from repository."""
        # Create and save multiple entities
        for i in range(3):
            entity = Entity(
                name=EntityName(text=f"Entity {i}"),
                value=EntityValue(amount=i * 10),
            )
            repository.save(entity)

        # Clear
        repository.clear()

        # Verify empty
        assert repository.list_all() == []

    def test_repository_satisfies_protocol(
        self,
        repository: InMemoryRepository[Entity],
    ) -> None:
        """Repository should work as Protocol type.

        This test verifies structural typing works correctly.
        """
        from graft.protocols.repository import Repository

        # Should be assignable to Protocol type
        repo: Repository[Entity] = repository

        # Should work through protocol interface
        entity = Entity(
            name=EntityName(text="Test"),
            value=EntityValue(amount=100),
        )

        repo.save(entity)
        retrieved = repo.get(entity.id)
        assert retrieved == entity
