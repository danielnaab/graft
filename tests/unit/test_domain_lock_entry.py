"""Tests for LockEntry domain model."""

from datetime import datetime, timezone

import pytest

from graft.domain.exceptions import ValidationError
from graft.domain.lock_entry import LockEntry


class TestLockEntry:
    """Tests for LockEntry value object."""

    def test_create_valid_lock_entry(self) -> None:
        """Should create lock entry with valid fields."""
        consumed_at = datetime(2026, 1, 1, 10, 30, 0, tzinfo=timezone.utc)
        entry = LockEntry(
            source="git@github.com:org/repo.git",
            ref="v1.5.0",
            commit="abc123def456789012345678901234567890abcd",
            consumed_at=consumed_at,
        )

        assert entry.source == "git@github.com:org/repo.git"
        assert entry.ref == "v1.5.0"
        assert entry.commit == "abc123def456789012345678901234567890abcd"
        assert entry.consumed_at == consumed_at

    def test_empty_source_raises_validation_error(self) -> None:
        """Should raise ValidationError for empty source."""
        with pytest.raises(ValidationError) as exc_info:
            LockEntry(
                source="",
                ref="v1.0.0",
                commit="a" * 40,
                consumed_at=datetime.now(timezone.utc),
            )

        assert "source cannot be empty" in str(exc_info.value)

    def test_whitespace_source_raises_validation_error(self) -> None:
        """Should raise ValidationError for whitespace-only source."""
        with pytest.raises(ValidationError) as exc_info:
            LockEntry(
                source="   ",
                ref="v1.0.0",
                commit="a" * 40,
                consumed_at=datetime.now(timezone.utc),
            )

        assert "source cannot be only whitespace" in str(exc_info.value)

    def test_empty_ref_raises_validation_error(self) -> None:
        """Should raise ValidationError for empty ref."""
        with pytest.raises(ValidationError) as exc_info:
            LockEntry(
                source="git@github.com:org/repo.git",
                ref="",
                commit="a" * 40,
                consumed_at=datetime.now(timezone.utc),
            )

        assert "ref cannot be empty" in str(exc_info.value)

    def test_whitespace_ref_raises_validation_error(self) -> None:
        """Should raise ValidationError for whitespace-only ref."""
        with pytest.raises(ValidationError) as exc_info:
            LockEntry(
                source="git@github.com:org/repo.git",
                ref="   ",
                commit="a" * 40,
                consumed_at=datetime.now(timezone.utc),
            )

        assert "ref cannot be only whitespace" in str(exc_info.value)

    def test_empty_commit_raises_validation_error(self) -> None:
        """Should raise ValidationError for empty commit."""
        with pytest.raises(ValidationError) as exc_info:
            LockEntry(
                source="git@github.com:org/repo.git",
                ref="v1.0.0",
                commit="",
                consumed_at=datetime.now(timezone.utc),
            )

        assert "commit cannot be empty" in str(exc_info.value)

    def test_invalid_commit_hash_format_raises_validation_error(self) -> None:
        """Should raise ValidationError for invalid commit hash format."""
        invalid_hashes = [
            "abc123",  # Too short
            "xyz123def456789012345678901234567890abcd",  # Invalid chars
            "ABC123DEF456789012345678901234567890ABCD",  # Uppercase
            "abc123def456789012345678901234567890abcd1",  # Too long (41 chars)
            "abc123def456789012345678901234567890abc",  # Too short (39 chars)
        ]

        for invalid_hash in invalid_hashes:
            with pytest.raises(ValidationError) as exc_info:
                LockEntry(
                    source="git@github.com:org/repo.git",
                    ref="v1.0.0",
                    commit=invalid_hash,
                    consumed_at=datetime.now(timezone.utc),
                )

            assert "Invalid commit hash format" in str(exc_info.value)
            assert "40-character SHA-1 hash" in str(exc_info.value)

    def test_valid_commit_hash_format(self) -> None:
        """Should accept valid 40-character SHA-1 hash."""
        valid_hash = "abc123def456789012345678901234567890abcd"

        entry = LockEntry(
            source="git@github.com:org/repo.git",
            ref="v1.0.0",
            commit=valid_hash,
            consumed_at=datetime.now(timezone.utc),
        )

        assert entry.commit == valid_hash

    def test_is_valid_commit_hash_returns_true_for_valid_hash(self) -> None:
        """Should return True for valid commit hash."""
        entry = LockEntry(
            source="git@github.com:org/repo.git",
            ref="v1.0.0",
            commit="abc123def456789012345678901234567890abcd",
            consumed_at=datetime.now(timezone.utc),
        )

        assert entry.is_valid_commit_hash() is True

    def test_to_dict_converts_to_serializable_format(self) -> None:
        """Should convert to dict suitable for YAML serialization."""
        consumed_at = datetime(2026, 1, 1, 10, 30, 0, tzinfo=timezone.utc)
        entry = LockEntry(
            source="git@github.com:org/repo.git",
            ref="v1.5.0",
            commit="abc123def456789012345678901234567890abcd",
            consumed_at=consumed_at,
        )

        result = entry.to_dict()

        assert result == {
            "source": "git@github.com:org/repo.git",
            "ref": "v1.5.0",
            "commit": "abc123def456789012345678901234567890abcd",
            "consumed_at": "2026-01-01T10:30:00+00:00",
        }
        # All values should be strings
        assert all(isinstance(v, str) for v in result.values())

    def test_from_dict_creates_lock_entry(self) -> None:
        """Should create LockEntry from dict."""
        data = {
            "source": "git@github.com:org/repo.git",
            "ref": "v1.0.0",
            "commit": "a" * 40,
            "consumed_at": "2026-01-01T10:30:00+00:00",
        }

        entry = LockEntry.from_dict(data)

        assert entry.source == "git@github.com:org/repo.git"
        assert entry.ref == "v1.0.0"
        assert entry.commit == "a" * 40
        assert entry.consumed_at == datetime(2026, 1, 1, 10, 30, 0, tzinfo=timezone.utc)

    def test_from_dict_with_missing_field_raises_validation_error(self) -> None:
        """Should raise ValidationError if required field is missing."""
        required_fields = ["source", "ref", "commit", "consumed_at"]

        for field_to_omit in required_fields:
            data = {
                "source": "git@github.com:org/repo.git",
                "ref": "v1.0.0",
                "commit": "a" * 40,
                "consumed_at": "2026-01-01T10:30:00+00:00",
            }
            del data[field_to_omit]

            with pytest.raises(ValidationError) as exc_info:
                LockEntry.from_dict(data)

            assert "missing required fields" in str(exc_info.value).lower()

    def test_from_dict_with_invalid_timestamp_raises_validation_error(self) -> None:
        """Should raise ValidationError for invalid timestamp format."""
        data = {
            "source": "git@github.com:org/repo.git",
            "ref": "v1.0.0",
            "commit": "a" * 40,
            "consumed_at": "not-a-timestamp",
        }

        with pytest.raises(ValidationError) as exc_info:
            LockEntry.from_dict(data)

        assert "Invalid timestamp format" in str(exc_info.value)

    def test_to_dict_and_from_dict_roundtrip(self) -> None:
        """Should preserve data through to_dict -> from_dict roundtrip."""
        original = LockEntry(
            source="git@github.com:org/repo.git",
            ref="v1.5.0",
            commit="abc123def456789012345678901234567890abcd",
            consumed_at=datetime(2026, 1, 1, 10, 30, 0, tzinfo=timezone.utc),
        )

        data = original.to_dict()
        restored = LockEntry.from_dict(data)

        assert restored.source == original.source
        assert restored.ref == original.ref
        assert restored.commit == original.commit
        assert restored.consumed_at == original.consumed_at

    def test_lock_entries_are_frozen(self) -> None:
        """Should not allow modification after creation."""
        entry = LockEntry(
            source="git@github.com:org/repo.git",
            ref="v1.0.0",
            commit="a" * 40,
            consumed_at=datetime.now(timezone.utc),
        )

        with pytest.raises(Exception):  # FrozenInstanceError or AttributeError
            entry.ref = "v2.0.0"  # type: ignore

    def test_lock_entries_with_same_fields_are_equal(self) -> None:
        """Should consider lock entries equal if all fields match."""
        consumed_at = datetime(2026, 1, 1, 10, 30, 0, tzinfo=timezone.utc)
        entry1 = LockEntry(
            source="git@github.com:org/repo.git",
            ref="v1.0.0",
            commit="a" * 40,
            consumed_at=consumed_at,
        )
        entry2 = LockEntry(
            source="git@github.com:org/repo.git",
            ref="v1.0.0",
            commit="a" * 40,
            consumed_at=consumed_at,
        )

        assert entry1 == entry2

    def test_lock_entries_with_different_fields_are_not_equal(self) -> None:
        """Should not be equal if any field differs."""
        consumed_at = datetime(2026, 1, 1, 10, 30, 0, tzinfo=timezone.utc)
        base = LockEntry(
            source="git@github.com:org/repo.git",
            ref="v1.0.0",
            commit="a" * 40,
            consumed_at=consumed_at,
        )
        different_ref = LockEntry(
            source="git@github.com:org/repo.git",
            ref="v2.0.0",
            commit="a" * 40,
            consumed_at=consumed_at,
        )
        different_commit = LockEntry(
            source="git@github.com:org/repo.git",
            ref="v1.0.0",
            commit="b" * 40,
            consumed_at=consumed_at,
        )

        assert base != different_ref
        assert base != different_commit

    def test_supports_various_git_url_formats(self) -> None:
        """Should accept various git URL formats."""
        url_formats = [
            "git@github.com:org/repo.git",
            "https://github.com/org/repo.git",
            "ssh://git@platform.com:2222/user/repo.git",
            "../local-repo",
            "/absolute/path/to/repo",
        ]

        for url in url_formats:
            entry = LockEntry(
                source=url,
                ref="v1.0.0",
                commit="a" * 40,
                consumed_at=datetime.now(timezone.utc),
            )
            assert entry.source == url

    def test_supports_various_ref_formats(self) -> None:
        """Should accept various ref formats."""
        ref_formats = [
            "v1.0.0",  # Semver tag
            "main",  # Branch
            "release-2026-01",  # Date-based tag
            "abc123",  # Short commit hash
            "a" * 40,  # Full commit hash
        ]

        for ref in ref_formats:
            entry = LockEntry(
                source="git@github.com:org/repo.git",
                ref=ref,
                commit="a" * 40,
                consumed_at=datetime.now(timezone.utc),
            )
            assert entry.ref == ref

    def test_timestamp_with_microseconds(self) -> None:
        """Should handle timestamps with microseconds."""
        consumed_at = datetime(2026, 1, 1, 10, 30, 0, 123456, tzinfo=timezone.utc)
        entry = LockEntry(
            source="git@github.com:org/repo.git",
            ref="v1.0.0",
            commit="a" * 40,
            consumed_at=consumed_at,
        )

        data = entry.to_dict()
        restored = LockEntry.from_dict(data)

        # Microseconds should be preserved
        assert restored.consumed_at == consumed_at
        assert ".123456" in data["consumed_at"]

    def test_from_dict_handles_various_timestamp_formats(self) -> None:
        """Should parse various ISO 8601 timestamp formats."""
        timestamp_formats = [
            "2026-01-01T10:30:00Z",
            "2026-01-01T10:30:00+00:00",
            "2026-01-01T10:30:00.123456+00:00",
            "2026-01-01T10:30:00.123456Z",
        ]

        for timestamp in timestamp_formats:
            data = {
                "source": "git@github.com:org/repo.git",
                "ref": "v1.0.0",
                "commit": "a" * 40,
                "consumed_at": timestamp,
            }

            entry = LockEntry.from_dict(data)
            assert isinstance(entry.consumed_at, datetime)
