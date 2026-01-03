"""Integration tests for adapters.

Tests using real adapter implementations (not fakes).
"""

from datetime import datetime, timezone
from pathlib import Path

import pytest

from graft.adapters.lock_file import YamlLockFile
from graft.adapters.repository import InMemoryRepository
from graft.domain.entities import Entity
from graft.domain.lock_entry import LockEntry
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


class TestYamlLockFile:
    """Integration tests for YamlLockFile adapter."""

    @pytest.fixture
    def lock_file(self) -> YamlLockFile:
        """Provide fresh YamlLockFile for each test."""
        return YamlLockFile()

    @pytest.fixture
    def temp_lock_path(self, tmp_path: Path) -> str:
        """Provide temporary lock file path."""
        return str(tmp_path / "graft.lock")

    def test_write_and_read_lock_file(
        self, lock_file: YamlLockFile, temp_lock_path: str
    ) -> None:
        """Should write and read lock file with entries."""
        # Create entries
        entries = {
            "dep1": LockEntry(
                source="git@github.com:org/repo1.git",
                ref="v1.0.0",
                commit="a" * 40,
                consumed_at=datetime(2025, 1, 1, 12, 0, 0, tzinfo=timezone.utc),
            ),
            "dep2": LockEntry(
                source="https://github.com/org/repo2.git",
                ref="main",
                commit="b" * 40,
                consumed_at=datetime(2025, 1, 2, 12, 0, 0, tzinfo=timezone.utc),
            ),
        }

        # Write
        lock_file.write_lock_file(temp_lock_path, entries)

        # Verify file exists
        assert Path(temp_lock_path).exists()

        # Read back
        read_entries = lock_file.read_lock_file(temp_lock_path)

        # Verify contents
        assert len(read_entries) == 2
        assert "dep1" in read_entries
        assert "dep2" in read_entries
        assert read_entries["dep1"].source == "git@github.com:org/repo1.git"
        assert read_entries["dep1"].ref == "v1.0.0"
        assert read_entries["dep1"].commit == "a" * 40
        assert read_entries["dep2"].source == "https://github.com/org/repo2.git"

    def test_write_empty_lock_file(
        self, lock_file: YamlLockFile, temp_lock_path: str
    ) -> None:
        """Should write empty lock file."""
        # Write empty
        lock_file.write_lock_file(temp_lock_path, {})

        # Read back
        entries = lock_file.read_lock_file(temp_lock_path)

        # Verify empty
        assert entries == {}

    def test_read_nonexistent_file_raises(self, lock_file: YamlLockFile) -> None:
        """Should raise FileNotFoundError for nonexistent file."""
        with pytest.raises(FileNotFoundError) as exc_info:
            lock_file.read_lock_file("/nonexistent/path/graft.lock")

        assert "Lock file not found" in str(exc_info.value)

    def test_update_lock_entry(
        self, lock_file: YamlLockFile, temp_lock_path: str
    ) -> None:
        """Should update single entry atomically."""
        # Create initial lock file
        initial_entries = {
            "dep1": LockEntry(
                source="git@github.com:org/repo1.git",
                ref="v1.0.0",
                commit="a" * 40,
                consumed_at=datetime(2025, 1, 1, tzinfo=timezone.utc),
            ),
            "dep2": LockEntry(
                source="git@github.com:org/repo2.git",
                ref="v1.0.0",
                commit="b" * 40,
                consumed_at=datetime(2025, 1, 1, tzinfo=timezone.utc),
            ),
        }
        lock_file.write_lock_file(temp_lock_path, initial_entries)

        # Update one entry
        updated_entry = LockEntry(
            source="git@github.com:org/repo1.git",
            ref="v2.0.0",
            commit="c" * 40,
            consumed_at=datetime(2025, 1, 2, tzinfo=timezone.utc),
        )
        lock_file.update_lock_entry(temp_lock_path, "dep1", updated_entry)

        # Read back
        entries = lock_file.read_lock_file(temp_lock_path)

        # Verify dep1 was updated
        assert entries["dep1"].ref == "v2.0.0"
        assert entries["dep1"].commit == "c" * 40

        # Verify dep2 unchanged
        assert entries["dep2"].ref == "v1.0.0"
        assert entries["dep2"].commit == "b" * 40

    def test_update_adds_new_entry(
        self, lock_file: YamlLockFile, temp_lock_path: str
    ) -> None:
        """Should add new entry when updating nonexistent dependency."""
        # Create lock file with one entry
        initial_entries = {
            "dep1": LockEntry(
                source="git@github.com:org/repo1.git",
                ref="v1.0.0",
                commit="a" * 40,
                consumed_at=datetime(2025, 1, 1, tzinfo=timezone.utc),
            ),
        }
        lock_file.write_lock_file(temp_lock_path, initial_entries)

        # Add new entry via update
        new_entry = LockEntry(
            source="git@github.com:org/repo2.git",
            ref="v1.0.0",
            commit="b" * 40,
            consumed_at=datetime(2025, 1, 2, tzinfo=timezone.utc),
        )
        lock_file.update_lock_entry(temp_lock_path, "dep2", new_entry)

        # Read back
        entries = lock_file.read_lock_file(temp_lock_path)

        # Verify both entries exist
        assert len(entries) == 2
        assert "dep1" in entries
        assert "dep2" in entries

    def test_lock_file_exists(
        self, lock_file: YamlLockFile, temp_lock_path: str
    ) -> None:
        """Should check if lock file exists."""
        # Initially doesn't exist
        assert not lock_file.lock_file_exists(temp_lock_path)

        # Write file
        lock_file.write_lock_file(temp_lock_path, {})

        # Now exists
        assert lock_file.lock_file_exists(temp_lock_path)

    def test_lock_file_version_validation(
        self, lock_file: YamlLockFile, temp_lock_path: str
    ) -> None:
        """Should validate lock file version."""
        # Write invalid lock file with wrong version
        Path(temp_lock_path).write_text("version: 999\ndependencies: {}\n")

        # Should raise ValueError
        with pytest.raises(ValueError) as exc_info:
            lock_file.read_lock_file(temp_lock_path)

        assert "Unsupported lock file version" in str(exc_info.value)

    def test_lock_file_missing_version(
        self, lock_file: YamlLockFile, temp_lock_path: str
    ) -> None:
        """Should require version field."""
        # Write lock file without version
        Path(temp_lock_path).write_text("dependencies: {}\n")

        # Should raise ValueError
        with pytest.raises(ValueError) as exc_info:
            lock_file.read_lock_file(temp_lock_path)

        assert "missing 'version' field" in str(exc_info.value)

    def test_yaml_format_is_readable(
        self, lock_file: YamlLockFile, temp_lock_path: str
    ) -> None:
        """Should write human-readable YAML."""
        # Write lock file
        entries = {
            "my-dep": LockEntry(
                source="git@github.com:org/repo.git",
                ref="v1.2.3",
                commit="abc123" + "0" * 34,
                consumed_at=datetime(2025, 1, 3, 10, 30, 0, tzinfo=timezone.utc),
            ),
        }
        lock_file.write_lock_file(temp_lock_path, entries)

        # Read raw file
        content = Path(temp_lock_path).read_text()

        # Verify key elements are present and readable
        assert "version: 1" in content
        assert "dependencies:" in content
        assert "my-dep:" in content
        assert "source: git@github.com:org/repo.git" in content
        assert "ref: v1.2.3" in content
        assert "commit: abc123" in content

    def test_satisfies_protocol(self, lock_file: YamlLockFile) -> None:
        """YamlLockFile should satisfy LockFile protocol.

        This test verifies structural typing works correctly.
        """
        from graft.protocols.lock_file import LockFile

        # Should be assignable to Protocol type
        protocol_lock_file: LockFile = lock_file

        # Type checker should accept this without error
        assert protocol_lock_file is not None
