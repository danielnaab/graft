"""Lock entry domain model.

Represents an entry in the graft.lock file tracking consumed dependency state.

Lock file format v3 uses a flat-only dependency model (see Decision 0007).
All dependencies are direct - there is no transitive resolution.
"""

import re
from dataclasses import dataclass
from datetime import datetime

from graft.domain.exceptions import ValidationError


@dataclass(frozen=True)
class LockEntry:
    """Represents a dependency entry in graft.lock file.

    Tracks the exact state of a consumed dependency for reproducibility
    and integrity verification.

    Lock file v3 uses a flat-only model: all dependencies are declared
    directly in graft.yaml. There is no transitive dependency resolution.

    Attributes:
        source: Git URL or path to dependency repository
        ref: Consumed git ref (tag, branch, or commit)
        commit: Full commit hash (40-char SHA-1)
        consumed_at: ISO 8601 timestamp when version was consumed

    Example:
        >>> from datetime import datetime, timezone
        >>> entry = LockEntry(
        ...     source="git@github.com:org/repo.git",
        ...     ref="v1.5.0",
        ...     commit="abc123def456789012345678901234567890abcd",
        ...     consumed_at=datetime.now(timezone.utc),
        ... )
        >>> entry.ref
        'v1.5.0'
        >>> entry.is_valid_commit_hash()
        True
    """

    source: str
    ref: str
    commit: str
    consumed_at: datetime

    # Pattern for validating 40-character SHA-1 commit hash
    _COMMIT_HASH_PATTERN = re.compile(r"^[0-9a-f]{40}$")

    def __post_init__(self) -> None:
        """Validate lock entry."""
        # Validate source
        if not self.source:
            raise ValidationError("Lock entry source cannot be empty")
        if not self.source.strip():
            raise ValidationError("Lock entry source cannot be only whitespace")

        # Validate ref
        if not self.ref:
            raise ValidationError("Lock entry ref cannot be empty")
        if not self.ref.strip():
            raise ValidationError("Lock entry ref cannot be only whitespace")

        # Validate commit hash format
        if not self.commit:
            raise ValidationError("Lock entry commit cannot be empty")
        if not self._COMMIT_HASH_PATTERN.match(self.commit):
            raise ValidationError(
                f"Invalid commit hash format: {self.commit}. "
                f"Must be 40-character SHA-1 hash (lowercase hex)"
            )

        # consumed_at is validated by type system (must be datetime)

    def is_valid_commit_hash(self) -> bool:
        """Check if commit hash is valid format.

        Returns:
            True if commit is a valid 40-character SHA-1 hash
        """
        return bool(self._COMMIT_HASH_PATTERN.match(self.commit))

    def to_dict(self) -> dict[str, str]:
        """Convert to dictionary suitable for YAML serialization.

        Returns:
            Dictionary with string values for YAML

        Example:
            >>> from datetime import datetime, timezone
            >>> entry = LockEntry(
            ...     source="git@github.com:org/repo.git",
            ...     ref="v1.0.0",
            ...     commit="a" * 40,
            ...     consumed_at=datetime(2026, 1, 1, 10, 30, 0, tzinfo=timezone.utc),
            ... )
            >>> result = entry.to_dict()
            >>> result['source']
            'git@github.com:org/repo.git'
            >>> result['ref']
            'v1.0.0'
        """
        return {
            "source": self.source,
            "ref": self.ref,
            "commit": self.commit,
            "consumed_at": self.consumed_at.isoformat(),
        }

    @classmethod
    def from_dict(cls, data: dict[str, str | bool | list[str]]) -> "LockEntry":
        """Create LockEntry from dictionary (from YAML).

        Args:
            data: Dictionary with required keys (source, ref, commit, consumed_at).
                  Legacy v2 fields (direct, requires, required_by) are ignored
                  for backward compatibility.

        Returns:
            New LockEntry instance

        Raises:
            ValidationError: If required fields missing or invalid format

        Example:
            >>> data = {
            ...     "source": "git@github.com:org/repo.git",
            ...     "ref": "v1.0.0",
            ...     "commit": "a" * 40,
            ...     "consumed_at": "2026-01-01T10:30:00+00:00"
            ... }
            >>> entry = LockEntry.from_dict(data)
            >>> entry.source
            'git@github.com:org/repo.git'
            >>> # Legacy v2 fields are silently ignored
            >>> data_v2 = {
            ...     "source": "git@github.com:org/repo.git",
            ...     "ref": "v1.0.0",
            ...     "commit": "a" * 40,
            ...     "consumed_at": "2026-01-01T10:30:00+00:00",
            ...     "direct": False,
            ...     "requires": ["dep1"],
            ...     "required_by": ["parent-dep"]
            ... }
            >>> entry_v2 = LockEntry.from_dict(data_v2)
            >>> entry_v2.source
            'git@github.com:org/repo.git'
        """
        # Check required fields
        required_fields = ["source", "ref", "commit", "consumed_at"]
        missing = [f for f in required_fields if f not in data]
        if missing:
            raise ValidationError(
                f"Lock entry missing required fields: {', '.join(missing)}"
            )

        # Parse timestamp
        try:
            consumed_at = datetime.fromisoformat(str(data["consumed_at"]))
        except (ValueError, TypeError) as e:
            raise ValidationError(
                f"Invalid timestamp format in lock entry: {data.get('consumed_at')}"
            ) from e

        # Note: Legacy v2 fields (direct, requires, required_by) are ignored
        # for backward compatibility when reading old lock files

        return cls(
            source=str(data["source"]),
            ref=str(data["ref"]),
            commit=str(data["commit"]),
            consumed_at=consumed_at,
        )
