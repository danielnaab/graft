"""Tests for Change domain model."""

import dataclasses

import pytest

from graft.domain.change import Change
from graft.domain.exceptions import ValidationError


class TestChange:
    """Tests for Change value object."""

    def test_create_minimal_change(self) -> None:
        """Should create change with just ref."""
        change = Change(ref="v1.0.0")

        assert change.ref == "v1.0.0"
        assert change.type is None
        assert change.description is None
        assert change.migration is None
        assert change.verify is None
        assert change.metadata == {}

    def test_create_full_change(self) -> None:
        """Should create change with all fields."""
        change = Change(
            ref="v2.0.0",
            type="breaking",
            description="Renamed getUserData → fetchUserData",
            migration="migrate-v2",
            verify="verify-v2",
            metadata={"author": "jane@example.com"},
        )

        assert change.ref == "v2.0.0"
        assert change.type == "breaking"
        assert change.description == "Renamed getUserData → fetchUserData"
        assert change.migration == "migrate-v2"
        assert change.verify == "verify-v2"
        assert change.metadata == {"author": "jane@example.com"}

    def test_empty_ref_raises_validation_error(self) -> None:
        """Should raise ValidationError for empty ref."""
        with pytest.raises(ValidationError) as exc_info:
            Change(ref="")

        assert "ref cannot be empty" in str(exc_info.value)

    def test_whitespace_ref_raises_validation_error(self) -> None:
        """Should raise ValidationError for whitespace-only ref."""
        with pytest.raises(ValidationError) as exc_info:
            Change(ref="   ")

        assert "ref cannot be only whitespace" in str(exc_info.value)

    def test_too_long_description_raises_validation_error(self) -> None:
        """Should raise ValidationError for description > 200 chars."""
        long_description = "x" * 201

        with pytest.raises(ValidationError) as exc_info:
            Change(ref="v1.0.0", description=long_description)

        assert "description too long" in str(exc_info.value)
        assert "200" in str(exc_info.value)

    def test_whitespace_migration_raises_validation_error(self) -> None:
        """Should raise ValidationError for whitespace-only migration."""
        with pytest.raises(ValidationError) as exc_info:
            Change(ref="v1.0.0", migration="   ")

        assert "Migration command name cannot be only whitespace" in str(exc_info.value)

    def test_whitespace_verify_raises_validation_error(self) -> None:
        """Should raise ValidationError for whitespace-only verify."""
        with pytest.raises(ValidationError) as exc_info:
            Change(ref="v1.0.0", verify="   ")

        assert "Verify command name cannot be only whitespace" in str(exc_info.value)

    def test_needs_migration_when_migration_defined(self) -> None:
        """Should return True if migration is defined."""
        change = Change(ref="v1.0.0", migration="migrate-v1")

        assert change.needs_migration() is True

    def test_needs_migration_when_migration_not_defined(self) -> None:
        """Should return False if migration is not defined."""
        change = Change(ref="v1.0.0")

        assert change.needs_migration() is False

    def test_needs_verification_when_verify_defined(self) -> None:
        """Should return True if verify is defined."""
        change = Change(ref="v1.0.0", verify="verify-v1")

        assert change.needs_verification() is True

    def test_needs_verification_when_verify_not_defined(self) -> None:
        """Should return False if verify is not defined."""
        change = Change(ref="v1.0.0")

        assert change.needs_verification() is False

    def test_is_breaking_when_type_is_breaking(self) -> None:
        """Should return True if type is 'breaking'."""
        change = Change(ref="v2.0.0", type="breaking")

        assert change.is_breaking() is True

    def test_is_breaking_when_type_is_not_breaking(self) -> None:
        """Should return False if type is not 'breaking'."""
        change1 = Change(ref="v1.1.0", type="feature")
        change2 = Change(ref="v1.0.1", type="fix")
        change3 = Change(ref="v1.0.0")

        assert change1.is_breaking() is False
        assert change2.is_breaking() is False
        assert change3.is_breaking() is False

    def test_with_metadata_adds_new_metadata(self) -> None:
        """Should create new change with additional metadata."""
        original = Change(ref="v1.0.0", metadata={"existing": "value"})

        new_change = original.with_metadata(author="jane@example.com", ticket="PROJ-123")

        # Original is unchanged (immutable)
        assert original.metadata == {"existing": "value"}

        # New change has merged metadata
        assert new_change.metadata == {
            "existing": "value",
            "author": "jane@example.com",
            "ticket": "PROJ-123",
        }

        # Other fields are preserved
        assert new_change.ref == original.ref

    def test_with_metadata_overwrites_existing_metadata(self) -> None:
        """Should overwrite existing metadata keys."""
        original = Change(ref="v1.0.0", metadata={"key": "old_value"})

        new_change = original.with_metadata(key="new_value")

        assert new_change.metadata == {"key": "new_value"}

    def test_changes_are_frozen(self) -> None:
        """Should not allow modification after creation."""
        change = Change(ref="v1.0.0")

        with pytest.raises(dataclasses.FrozenInstanceError):
            change.ref = "v2.0.0"  # type: ignore

    def test_changes_with_same_ref_are_equal(self) -> None:
        """Should consider changes equal if all fields match."""
        change1 = Change(
            ref="v1.0.0",
            type="feature",
            description="Test",
            migration="migrate",
            verify="verify",
        )
        change2 = Change(
            ref="v1.0.0",
            type="feature",
            description="Test",
            migration="migrate",
            verify="verify",
        )

        assert change1 == change2

    def test_changes_with_different_fields_are_not_equal(self) -> None:
        """Should not be equal if any field differs."""
        base = Change(ref="v1.0.0", type="feature")
        different_ref = Change(ref="v2.0.0", type="feature")
        different_type = Change(ref="v1.0.0", type="fix")

        assert base != different_ref
        assert base != different_type

    def test_change_repr(self) -> None:
        """Should have helpful repr."""
        change = Change(ref="v1.0.0", type="feature")

        repr_str = repr(change)

        assert "Change" in repr_str
        assert "v1.0.0" in repr_str

    def test_supports_various_change_types(self) -> None:
        """Should support standard and custom change types."""
        standard_types = [
            "breaking",
            "feature",
            "fix",
            "deprecation",
            "security",
            "performance",
            "docs",
            "internal",
        ]

        for change_type in standard_types:
            change = Change(ref="v1.0.0", type=change_type)
            assert change.type == change_type

        # Custom type
        custom = Change(ref="v1.0.0", type="custom-type")
        assert custom.type == "custom-type"

    def test_metadata_default_is_empty_dict(self) -> None:
        """Should default to empty dict for metadata."""
        change = Change(ref="v1.0.0")

        assert change.metadata == {}
        assert isinstance(change.metadata, dict)

    def test_complex_metadata(self) -> None:
        """Should support complex metadata structures."""
        metadata = {
            "author": "jane@example.com",
            "jira_ticket": "PROJ-123",
            "review_url": "https://github.com/org/repo/pull/42",
            "breaking_apis": ["getUserData", "setUserData"],
            "estimated_duration": 30,
        }

        change = Change(ref="v3.0.0", metadata=metadata)

        assert change.metadata == metadata
        assert change.metadata["breaking_apis"] == ["getUserData", "setUserData"]
        assert change.metadata["estimated_duration"] == 30
