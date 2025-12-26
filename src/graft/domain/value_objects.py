"""Domain value objects - immutable values with validation.

Value objects are distinguished by their attributes, not identity.
Two value objects with the same attributes are considered equal.
"""

from dataclasses import dataclass

from graft.domain.exceptions import ValidationError


@dataclass(frozen=True)
class EntityName:
    """Value object for entity names.

    Immutable value with validation.

    Attributes:
        text: The name text (non-empty, max 100 chars)
    """

    text: str

    def __post_init__(self) -> None:
        """Validate name after initialization.

        Raises:
            ValidationError: If name is invalid
        """
        if not self.text:
            raise ValidationError("Name cannot be empty")
        if len(self.text) > 100:
            raise ValidationError(f"Name too long: {len(self.text)} chars (max 100)")
        if not self.text.strip():
            raise ValidationError("Name cannot be only whitespace")

    def __str__(self) -> str:
        """String representation is the text value.

        Returns:
            The name text
        """
        return self.text


@dataclass(frozen=True)
class EntityValue:
    """Value object for entity values.

    Immutable value with validation.

    Attributes:
        amount: The numeric value (must be non-negative)
    """

    amount: int

    def __post_init__(self) -> None:
        """Validate value after initialization.

        Raises:
            ValidationError: If value is invalid
        """
        if self.amount < 0:
            raise ValidationError(f"Value cannot be negative: {self.amount}")

    def __str__(self) -> str:
        """String representation is the amount.

        Returns:
            String representation of amount
        """
        return str(self.amount)

    def __int__(self) -> int:
        """Integer conversion returns the amount.

        Returns:
            The numeric amount
        """
        return self.amount

    def increment(self, delta: int) -> "EntityValue":
        """Create new value object with incremented amount.

        Value objects are immutable, so this returns a new instance.

        Args:
            delta: Amount to add (can be negative)

        Returns:
            New EntityValue with updated amount

        Raises:
            ValidationError: If result would be negative
        """
        new_amount = self.amount + delta
        return EntityValue(amount=new_amount)
