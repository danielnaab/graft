"""Unit tests for domain layer.

Tests for entities, value objects, and business logic.
"""

import pytest

from graft.domain.entities import Entity
from graft.domain.exceptions import ValidationError
from graft.domain.value_objects import EntityName, EntityValue


class TestEntityName:
    """Tests for EntityName value object."""

    def test_create_valid_name(self) -> None:
        """Should create name with valid text."""
        name = EntityName(text="Test Name")
        assert name.text == "Test Name"
        assert str(name) == "Test Name"

    def test_empty_name_raises_error(self) -> None:
        """Should reject empty name."""
        with pytest.raises(ValidationError, match="cannot be empty"):
            EntityName(text="")

    def test_whitespace_only_name_raises_error(self) -> None:
        """Should reject whitespace-only name."""
        with pytest.raises(ValidationError, match="cannot be only whitespace"):
            EntityName(text="   ")

    def test_too_long_name_raises_error(self) -> None:
        """Should reject name longer than 100 characters."""
        long_name = "a" * 101
        with pytest.raises(ValidationError, match="too long"):
            EntityName(text=long_name)

    def test_value_objects_with_same_value_are_equal(self) -> None:
        """Value objects with same value should be equal."""
        name1 = EntityName(text="Test")
        name2 = EntityName(text="Test")
        assert name1 == name2

    def test_value_objects_are_immutable(self) -> None:
        """Value objects should be immutable (frozen)."""
        name = EntityName(text="Test")
        with pytest.raises(AttributeError):
            name.text = "Changed"  # type: ignore[misc]


class TestEntityValue:
    """Tests for EntityValue value object."""

    def test_create_valid_value(self) -> None:
        """Should create value with valid amount."""
        value = EntityValue(amount=100)
        assert value.amount == 100
        assert int(value) == 100
        assert str(value) == "100"

    def test_zero_value_is_valid(self) -> None:
        """Should accept zero as valid value."""
        value = EntityValue(amount=0)
        assert value.amount == 0

    def test_negative_value_raises_error(self) -> None:
        """Should reject negative values."""
        with pytest.raises(ValidationError, match="cannot be negative"):
            EntityValue(amount=-1)

    def test_increment_creates_new_instance(self) -> None:
        """Increment should return new instance (immutability)."""
        value1 = EntityValue(amount=100)
        value2 = value1.increment(50)

        assert value1.amount == 100  # Original unchanged
        assert value2.amount == 150  # New instance
        assert value1 is not value2

    def test_increment_with_negative_delta(self) -> None:
        """Should allow negative delta if result is non-negative."""
        value = EntityValue(amount=100)
        result = value.increment(-50)
        assert result.amount == 50

    def test_increment_raises_error_if_result_negative(self) -> None:
        """Should reject increment that results in negative value."""
        value = EntityValue(amount=50)
        with pytest.raises(ValidationError, match="cannot be negative"):
            value.increment(-100)


class TestEntity:
    """Tests for Entity."""

    def test_create_entity(self) -> None:
        """Should create entity with valid data."""
        name = EntityName(text="Test")
        value = EntityValue(amount=100)
        entity = Entity(name=name, value=value)

        assert entity.name == name
        assert entity.value == value
        assert entity.id  # ID auto-generated

    def test_entities_with_same_id_are_equal(self) -> None:
        """Entities with same ID should be equal (identity)."""
        name1 = EntityName(text="Test1")
        name2 = EntityName(text="Test2")
        value1 = EntityValue(amount=100)
        value2 = EntityValue(amount=200)

        entity1 = Entity(name=name1, value=value1, id="same-id")
        entity2 = Entity(name=name2, value=value2, id="same-id")

        assert entity1 == entity2  # Same ID = same entity

    def test_entities_with_different_ids_are_not_equal(self) -> None:
        """Entities with different IDs should not be equal."""
        name = EntityName(text="Test")
        value = EntityValue(amount=100)

        entity1 = Entity(name=name, value=value, id="id-1")
        entity2 = Entity(name=name, value=value, id="id-2")

        assert entity1 != entity2

    def test_entity_is_hashable(self) -> None:
        """Entities should be usable in sets and dicts."""
        name = EntityName(text="Test")
        value = EntityValue(amount=100)
        entity = Entity(name=name, value=value)

        # Should not raise
        entity_set = {entity}
        assert entity in entity_set

    def test_update_value_with_higher_value(self) -> None:
        """Should allow updating to higher value."""
        name = EntityName(text="Test")
        value1 = EntityValue(amount=100)
        entity = Entity(name=name, value=value1)

        value2 = EntityValue(amount=200)
        entity.update_value(value2)

        assert entity.value == value2

    def test_update_value_with_lower_value_raises_error(self) -> None:
        """Should reject updating to lower value (business rule)."""
        name = EntityName(text="Test")
        value1 = EntityValue(amount=200)
        entity = Entity(name=name, value=value1)

        value2 = EntityValue(amount=100)
        with pytest.raises(ValueError, match="can only increase"):
            entity.update_value(value2)

    def test_rename_entity(self) -> None:
        """Should allow renaming entity."""
        name1 = EntityName(text="Original")
        value = EntityValue(amount=100)
        entity = Entity(name=name1, value=value)

        name2 = EntityName(text="Updated")
        entity.rename(name2)

        assert entity.name == name2

    def test_entity_repr(self) -> None:
        """Entity repr should show key attributes."""
        name = EntityName(text="Test")
        value = EntityValue(amount=100)
        entity = Entity(name=name, value=value, id="test-id")

        repr_str = repr(entity)
        assert "test-id" in repr_str
        assert "Test" in repr_str
        assert "100" in repr_str
