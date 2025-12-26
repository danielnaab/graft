"""Unit tests for service layer.

Tests for service functions using fake dependencies.
"""

import pytest

from graft.domain.exceptions import EntityNotFoundError, ValidationError
from graft.services import example_service
from graft.services.context import ServiceContext
from tests.fakes.fake_repository import FakeRepository


class TestCreateExample:
    """Tests for create_example service function."""

    def test_create_valid_entity(self, test_context: ServiceContext) -> None:
        """Should create and save entity with valid data."""
        entity = example_service.create_example(
            test_context,
            name="Test Entity",
            value=100,
        )

        assert entity.name.text == "Test Entity"
        assert entity.value.amount == 100
        assert entity.id  # ID should be generated

    def test_created_entity_is_persisted(self, test_context: ServiceContext) -> None:
        """Should save entity to repository."""
        entity = example_service.create_example(
            test_context,
            name="Test",
            value=100,
        )

        # Verify it was saved by retrieving it
        saved_entity = test_context.repository.get(entity.id)
        assert saved_entity == entity

    def test_invalid_name_raises_validation_error(
        self,
        test_context: ServiceContext,
    ) -> None:
        """Should raise ValidationError for invalid name."""
        with pytest.raises(ValidationError, match="cannot be empty"):
            example_service.create_example(
                test_context,
                name="",  # Invalid: empty
                value=100,
            )

    def test_invalid_value_raises_validation_error(
        self,
        test_context: ServiceContext,
    ) -> None:
        """Should raise ValidationError for invalid value."""
        with pytest.raises(ValidationError, match="cannot be negative"):
            example_service.create_example(
                test_context,
                name="Test",
                value=-1,  # Invalid: negative
            )

    def test_repository_is_called(
        self,
        fake_repository: FakeRepository,
        test_context: ServiceContext,
    ) -> None:
        """Should call repository.save()."""
        example_service.create_example(
            test_context,
            name="Test",
            value=100,
        )

        # Using fake's test helper
        assert fake_repository.get_save_call_count() == 1


class TestGetExample:
    """Tests for get_example service function."""

    def test_get_existing_entity(self, test_context: ServiceContext) -> None:
        """Should retrieve existing entity by ID."""
        # Setup: Create entity
        created = example_service.create_example(
            test_context,
            name="Test",
            value=100,
        )

        # Exercise: Get entity
        retrieved = example_service.get_example(test_context, created.id)

        # Verify
        assert retrieved == created

    def test_get_nonexistent_entity_raises_error(
        self,
        test_context: ServiceContext,
    ) -> None:
        """Should raise EntityNotFoundError for nonexistent ID."""
        with pytest.raises(EntityNotFoundError, match="not found"):
            example_service.get_example(test_context, "nonexistent-id")

    def test_repository_is_called_with_correct_id(
        self,
        fake_repository: FakeRepository,
        test_context: ServiceContext,
    ) -> None:
        """Should call repository.get() with correct ID."""
        # Expect error since entity doesn't exist
        with pytest.raises(EntityNotFoundError):
            example_service.get_example(test_context, "test-id")

        # Verify repository was called with correct ID
        assert fake_repository.was_get_called_with("test-id")


class TestListExamples:
    """Tests for list_examples service function."""

    def test_list_when_empty(self, test_context: ServiceContext) -> None:
        """Should return empty list when no entities exist."""
        entities = example_service.list_examples(test_context)
        assert entities == []

    def test_list_with_entities(self, test_context: ServiceContext) -> None:
        """Should return all entities."""
        # Setup: Create multiple entities
        entity1 = example_service.create_example(test_context, "Entity 1", 100)
        entity2 = example_service.create_example(test_context, "Entity 2", 200)
        entity3 = example_service.create_example(test_context, "Entity 3", 300)

        # Exercise
        entities = example_service.list_examples(test_context)

        # Verify
        assert len(entities) == 3
        assert entity1 in entities
        assert entity2 in entities
        assert entity3 in entities

    def test_list_returns_correct_count(
        self,
        fake_repository: FakeRepository,
        test_context: ServiceContext,
    ) -> None:
        """Should return correct number of entities."""
        # Create 5 entities
        for i in range(5):
            example_service.create_example(test_context, f"Entity {i}", i * 10)

        entities = example_service.list_examples(test_context)

        assert len(entities) == 5
        assert fake_repository.get_saved_count() == 5
