"""Domain entities - objects with identity.

Entities are distinguished by their ID rather than attributes.
Two entities with the same ID are the same entity, even if attributes differ.
"""

from dataclasses import dataclass, field
from uuid import uuid4

from graft.domain.value_objects import EntityName, EntityValue


@dataclass
class Entity:
    """Example entity demonstrating domain modeling patterns.

    Remove or customize this for your domain.

    Key characteristics:
    - Has identity (id field)
    - Mutable (not frozen)
    - Contains business logic
    - Value objects for validated data

    Attributes:
        name: Entity name (value object)
        value: Entity value (value object)
        id: Unique identifier
    """

    name: EntityName
    value: EntityValue
    id: str = field(default_factory=lambda: str(uuid4()))

    def __post_init__(self) -> None:
        """Validate entity after initialization."""
        if not self.id:
            raise ValueError("Entity ID cannot be empty")

    def update_value(self, new_value: EntityValue) -> None:
        """Update entity value.

        Business logic example: value can only increase.

        Args:
            new_value: New value to set

        Raises:
            ValueError: If new value is less than current value
        """
        if new_value.amount < self.value.amount:
            raise ValueError(
                f"Value can only increase (current: {self.value.amount}, "
                f"attempted: {new_value.amount})"
            )
        self.value = new_value

    def rename(self, new_name: EntityName) -> None:
        """Rename entity.

        Args:
            new_name: New name to set
        """
        self.name = new_name

    def __eq__(self, other: object) -> bool:
        """Entities are equal if they have the same ID.

        Args:
            other: Object to compare

        Returns:
            True if same entity (same ID)
        """
        if not isinstance(other, Entity):
            return NotImplemented
        return self.id == other.id

    def __hash__(self) -> int:
        """Hash based on ID for set/dict usage.

        Returns:
            Hash of entity ID
        """
        return hash(self.id)

    def __repr__(self) -> str:
        """Developer-friendly representation.

        Returns:
            String representation showing key attributes
        """
        return (
            f"Entity(id={self.id!r}, name={self.name.text!r}, "
            f"value={self.value.amount})"
        )
