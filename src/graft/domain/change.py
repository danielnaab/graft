"""Change domain model.

Represents a semantic change in a dependency, identified by a git ref
and optionally associated with migration and verification operations.
"""

from dataclasses import dataclass, field
from typing import Any

from graft.domain.exceptions import ValidationError


@dataclass(frozen=True)
class Change:
    """Represents a semantic change in a dependency.

    A Change is the atomic unit of semantic evolution, identified by a git ref
    and optionally associated with migration operations.

    Attributes:
        ref: Git ref identifying this change (commit, tag, or branch)
        type: Optional semantic type ("breaking", "feature", "fix", etc.)
        description: Optional brief human-readable summary
        migration: Optional command name for migration
        verify: Optional command name for verification
        metadata: Optional additional custom metadata

    Example:
        >>> change = Change(
        ...     ref="v2.0.0",
        ...     type="breaking",
        ...     description="Renamed getUserData → fetchUserData",
        ...     migration="migrate-v2",
        ...     verify="verify-v2"
        ... )
        >>> change.ref
        'v2.0.0'
        >>> change.needs_migration()
        True
    """

    ref: str
    type: str | None = None
    description: str | None = None
    migration: str | None = None
    verify: str | None = None
    metadata: dict[str, Any] = field(default_factory=dict)

    def __post_init__(self) -> None:
        """Validate change."""
        # Validate ref
        if not self.ref:
            raise ValidationError("Change ref cannot be empty")
        if not self.ref.strip():
            raise ValidationError("Change ref cannot be only whitespace")

        # Validate description length if provided
        if self.description and len(self.description) > 200:
            raise ValidationError(
                f"Change description too long: {len(self.description)} chars (max 200)"
            )

        # Validate command names if provided
        if self.migration and not self.migration.strip():
            raise ValidationError("Migration command name cannot be only whitespace")
        if self.verify and not self.verify.strip():
            raise ValidationError("Verify command name cannot be only whitespace")

    def needs_migration(self) -> bool:
        """Check if this change requires migration.

        Returns:
            True if migration command is defined
        """
        return self.migration is not None

    def needs_verification(self) -> bool:
        """Check if this change requires verification.

        Returns:
            True if verification command is defined
        """
        return self.verify is not None

    def is_breaking(self) -> bool:
        """Check if this is a breaking change.

        Returns:
            True if type is "breaking"
        """
        return self.type == "breaking"

    def with_metadata(self, **kwargs: Any) -> "Change":
        """Create a new Change with additional metadata.

        Args:
            **kwargs: Additional metadata key-value pairs

        Returns:
            New Change instance with merged metadata

        Example:
            >>> change = Change(ref="v1.0.0")
            >>> new_change = change.with_metadata(author="jane@example.com")
            >>> new_change.metadata["author"]
            'jane@example.com'
        """
        new_metadata = {**self.metadata, **kwargs}
        # Use object.__setattr__ for frozen dataclass
        new_change = object.__new__(Change)
        object.__setattr__(new_change, "ref", self.ref)
        object.__setattr__(new_change, "type", self.type)
        object.__setattr__(new_change, "description", self.description)
        object.__setattr__(new_change, "migration", self.migration)
        object.__setattr__(new_change, "verify", self.verify)
        object.__setattr__(new_change, "metadata", new_metadata)
        return new_change
