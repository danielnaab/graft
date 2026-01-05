"""Tests for query service."""

from datetime import UTC, datetime

import pytest

from graft.domain.change import Change
from graft.domain.command import Command
from graft.domain.config import GraftConfig
from graft.domain.lock_entry import LockEntry
from graft.services import query_service
from tests.fakes.fake_lock_file import FakeLockFile


@pytest.fixture
def fake_lock_file() -> FakeLockFile:
    """Create fake lock file for testing."""
    return FakeLockFile()


@pytest.fixture
def sample_config() -> GraftConfig:
    """Create sample config for testing."""
    return GraftConfig(
        api_version="graft/v0",
        changes={
            "v1.1.0": Change(
                ref="v1.1.0",
                type="feature",
                description="Added caching",
            ),
            "v2.0.0": Change(
                ref="v2.0.0",
                type="breaking",
                description="Renamed API",
                migration="migrate-v2",
                verify="verify-v2",
            ),
            "v2.1.0": Change(
                ref="v2.1.0",
                type="fix",
                description="Fixed bug",
            ),
        },
        commands={
            "migrate-v2": Command(
                name="migrate-v2",
                run="python migrate.py",
                description="Migrate to v2",
            ),
            "verify-v2": Command(
                name="verify-v2",
                run="pytest",
                description="Verify migration",
            ),
        },
    )


class TestGetAllStatus:
    """Tests for get_all_status function."""

    def test_get_all_status_from_populated_lock_file(
        self, fake_lock_file: FakeLockFile
    ) -> None:
        """Should return status for all dependencies."""
        # Setup lock file
        entries = {
            "dep1": LockEntry(
                source="git@github.com:org/dep1.git",
                ref="v1.0.0",
                commit="a" * 40,
                consumed_at=datetime(2025, 1, 1, tzinfo=UTC),
            ),
            "dep2": LockEntry(
                source="git@github.com:org/dep2.git",
                ref="v2.0.0",
                commit="b" * 40,
                consumed_at=datetime(2025, 1, 2, tzinfo=UTC),
            ),
        }
        fake_lock_file.write_lock_file("/test/graft.lock", entries)

        # Execute
        statuses = query_service.get_all_status(fake_lock_file, "/test/graft.lock")

        # Verify
        assert len(statuses) == 2
        assert statuses[0].name == "dep1"
        assert statuses[0].current_ref == "v1.0.0"
        assert statuses[0].commit == "a" * 40
        assert statuses[1].name == "dep2"
        assert statuses[1].current_ref == "v2.0.0"

    def test_get_all_status_from_empty_lock_file(
        self, fake_lock_file: FakeLockFile
    ) -> None:
        """Should return empty list for empty lock file."""
        fake_lock_file.write_lock_file("/test/graft.lock", {})

        statuses = query_service.get_all_status(fake_lock_file, "/test/graft.lock")

        assert statuses == []

    def test_get_all_status_when_lock_file_not_exists(
        self, fake_lock_file: FakeLockFile
    ) -> None:
        """Should return empty list if lock file doesn't exist."""
        statuses = query_service.get_all_status(fake_lock_file, "/test/graft.lock")

        assert statuses == []


class TestGetDependencyStatus:
    """Tests for get_dependency_status function."""

    def test_get_existing_dependency_status(
        self, fake_lock_file: FakeLockFile
    ) -> None:
        """Should return status for existing dependency."""
        # Setup
        entry = LockEntry(
            source="git@github.com:org/dep.git",
            ref="v1.5.0",
            commit="c" * 40,
            consumed_at=datetime(2025, 1, 3, 10, 30, tzinfo=UTC),
        )
        fake_lock_file.write_lock_file("/test/graft.lock", {"my-dep": entry})

        # Execute
        status = query_service.get_dependency_status(
            fake_lock_file, "/test/graft.lock", "my-dep"
        )

        # Verify
        assert status is not None
        assert status.name == "my-dep"
        assert status.current_ref == "v1.5.0"
        assert status.commit == "c" * 40
        assert status.consumed_at == datetime(2025, 1, 3, 10, 30, tzinfo=UTC)

    def test_get_nonexistent_dependency_status(
        self, fake_lock_file: FakeLockFile
    ) -> None:
        """Should return None for nonexistent dependency."""
        fake_lock_file.write_lock_file("/test/graft.lock", {})

        status = query_service.get_dependency_status(
            fake_lock_file, "/test/graft.lock", "nonexistent"
        )

        assert status is None

    def test_get_dependency_status_when_lock_file_not_exists(
        self, fake_lock_file: FakeLockFile
    ) -> None:
        """Should return None if lock file doesn't exist."""
        status = query_service.get_dependency_status(
            fake_lock_file, "/test/graft.lock", "my-dep"
        )

        assert status is None


class TestGetChangesForDependency:
    """Tests for get_changes_for_dependency function."""

    def test_get_all_changes(self, sample_config: GraftConfig) -> None:
        """Should return all changes from config."""
        changes = query_service.get_changes_for_dependency(sample_config)

        assert len(changes) == 3
        assert changes[0].ref == "v1.1.0"
        assert changes[1].ref == "v2.0.0"
        assert changes[2].ref == "v2.1.0"

    def test_get_changes_preserves_order(self, sample_config: GraftConfig) -> None:
        """Should preserve declaration order."""
        changes = query_service.get_changes_for_dependency(sample_config)

        # Python 3.7+ dict preserves insertion order
        refs = [c.ref for c in changes]
        assert refs == ["v1.1.0", "v2.0.0", "v2.1.0"]

    def test_get_changes_from_config_with_no_changes(self) -> None:
        """Should return empty list when no changes."""
        config = GraftConfig(api_version="graft/v0")

        changes = query_service.get_changes_for_dependency(config)

        assert changes == []


class TestFilterChangesByType:
    """Tests for filter_changes_by_type function."""

    def test_filter_by_breaking_type(self, sample_config: GraftConfig) -> None:
        """Should filter to breaking changes."""
        all_changes = query_service.get_changes_for_dependency(sample_config)

        breaking = query_service.filter_changes_by_type(all_changes, "breaking")

        assert len(breaking) == 1
        assert breaking[0].ref == "v2.0.0"

    def test_filter_by_feature_type(self, sample_config: GraftConfig) -> None:
        """Should filter to feature changes."""
        all_changes = query_service.get_changes_for_dependency(sample_config)

        features = query_service.filter_changes_by_type(all_changes, "feature")

        assert len(features) == 1
        assert features[0].ref == "v1.1.0"

    def test_filter_by_fix_type(self, sample_config: GraftConfig) -> None:
        """Should filter to fix changes."""
        all_changes = query_service.get_changes_for_dependency(sample_config)

        fixes = query_service.filter_changes_by_type(all_changes, "fix")

        assert len(fixes) == 1
        assert fixes[0].ref == "v2.1.0"

    def test_filter_by_nonexistent_type(self, sample_config: GraftConfig) -> None:
        """Should return empty list for nonexistent type."""
        all_changes = query_service.get_changes_for_dependency(sample_config)

        result = query_service.filter_changes_by_type(all_changes, "nonexistent")

        assert result == []


class TestFilterBreakingChanges:
    """Tests for filter_breaking_changes function."""

    def test_filter_breaking_changes(self, sample_config: GraftConfig) -> None:
        """Should filter to breaking changes only."""
        all_changes = query_service.get_changes_for_dependency(sample_config)

        breaking = query_service.filter_breaking_changes(all_changes)

        assert len(breaking) == 1
        assert breaking[0].ref == "v2.0.0"
        assert breaking[0].is_breaking()

    def test_filter_breaking_changes_when_none(self) -> None:
        """Should return empty list when no breaking changes."""
        config = GraftConfig(
            api_version="graft/v0",
            changes={
                "v1.1.0": Change(ref="v1.1.0", type="feature"),
                "v1.2.0": Change(ref="v1.2.0", type="fix"),
            },
        )

        all_changes = query_service.get_changes_for_dependency(config)
        breaking = query_service.filter_breaking_changes(all_changes)

        assert breaking == []


class TestGetChangeByRef:
    """Tests for get_change_by_ref function."""

    def test_get_existing_change(self, sample_config: GraftConfig) -> None:
        """Should return change for existing ref."""
        change = query_service.get_change_by_ref(sample_config, "v2.0.0")

        assert change is not None
        assert change.ref == "v2.0.0"
        assert change.type == "breaking"
        assert change.description == "Renamed API"

    def test_get_nonexistent_change(self, sample_config: GraftConfig) -> None:
        """Should return None for nonexistent ref."""
        change = query_service.get_change_by_ref(sample_config, "v99.0.0")

        assert change is None


class TestGetChangeDetails:
    """Tests for get_change_details function."""

    def test_get_details_with_commands(self, sample_config: GraftConfig) -> None:
        """Should return details including commands."""
        details = query_service.get_change_details(sample_config, "v2.0.0")

        assert details is not None
        assert details.change.ref == "v2.0.0"
        assert details.change.type == "breaking"

        # Check migration command
        assert details.migration_command is not None
        assert details.migration_command.name == "migrate-v2"
        assert details.migration_command.run == "python migrate.py"

        # Check verify command
        assert details.verify_command is not None
        assert details.verify_command.name == "verify-v2"
        assert details.verify_command.run == "pytest"

    def test_get_details_without_commands(self, sample_config: GraftConfig) -> None:
        """Should return details with None commands when not specified."""
        details = query_service.get_change_details(sample_config, "v1.1.0")

        assert details is not None
        assert details.change.ref == "v1.1.0"
        assert details.migration_command is None
        assert details.verify_command is None

    def test_get_details_for_nonexistent_change(
        self, sample_config: GraftConfig
    ) -> None:
        """Should return None for nonexistent change."""
        details = query_service.get_change_details(sample_config, "v99.0.0")

        assert details is None
