"""Tests for upgrade service."""

import pytest

from graft.domain.change import Change
from graft.domain.command import Command
from graft.domain.config import GraftConfig
from graft.services.upgrade_service import rollback_upgrade, upgrade_dependency
from tests.fakes.fake_command_executor import FakeCommandExecutor
from tests.fakes.fake_lock_file import FakeLockFile
from tests.fakes.fake_snapshot import FakeSnapshot


class TestUpgradeDependency:
    """Tests for upgrade_dependency()."""

    @pytest.fixture
    def fake_snapshot(self):
        """Create fake snapshot."""
        snap = FakeSnapshot()
        # Simulate files exist
        snap.set_file_content(".graft/deps/my-dep", "old content")
        snap.set_file_content("graft.lock", "old lock")
        return snap

    @pytest.fixture
    def fake_executor(self):
        """Create fake command executor."""
        return FakeCommandExecutor()

    @pytest.fixture
    def fake_lock_file(self):
        """Create fake lock file."""
        return FakeLockFile()

    @pytest.fixture
    def config_with_change(self):
        """Create config with a change."""
        return GraftConfig(
            api_version="graft/v0",
            commands={
                "migrate": Command(name="migrate", run="echo migrating", description="Migration"),
                "verify": Command(name="verify", run="echo verifying", description="Verification"),
            },
            changes={
                "v2.0.0": Change(
                    ref="v2.0.0",
                    type="breaking",
                    description="Breaking change",
                    migration="migrate",
                    verify="verify",
                ),
            },
        )

    def test_successful_upgrade_with_migration_and_verify(
        self,
        fake_snapshot,
        fake_executor,
        fake_lock_file,
        config_with_change,
    ):
        """Should successfully upgrade with migration and verification."""
        result = upgrade_dependency(
            fake_snapshot,
            fake_executor,
            fake_lock_file,
            config_with_change,
            dep_name="my-dep",
            to_ref="v2.0.0",
            source="git@example.com:repo.git",
            commit="a" * 40,  # Valid 40-char SHA-1
            base_dir="/project",
            lock_path="/project/graft.lock",
        )

        assert result.success is True
        assert result.error is None
        assert result.migration_result is not None
        assert result.verify_result is not None
        assert result.migration_result.success is True
        assert result.verify_result.success is True

    def test_creates_snapshot_before_upgrade(
        self,
        fake_snapshot,
        fake_executor,
        fake_lock_file,
        config_with_change,
    ):
        """Should create snapshot before running any commands."""
        upgrade_dependency(
            fake_snapshot,
            fake_executor,
            fake_lock_file,
            config_with_change,
            dep_name="my-dep",
            to_ref="v2.0.0",
            source="git@example.com:repo.git",
            commit="a" * 40,
            base_dir="/project",
            lock_path="/project/graft.lock",
        )

        snapshots = fake_snapshot.list_snapshots()
        # Should have created and cleaned up snapshot
        assert len(snapshots) == 0  # auto_cleanup=True by default

    def test_updates_lock_file_on_success(
        self,
        fake_snapshot,
        fake_executor,
        fake_lock_file,
        config_with_change,
    ):
        """Should update lock file after successful upgrade."""
        fake_lock_file.write_lock_file("/project/graft.lock", {})

        result = upgrade_dependency(
            fake_snapshot,
            fake_executor,
            fake_lock_file,
            config_with_change,
            dep_name="my-dep",
            to_ref="v2.0.0",
            source="git@example.com:repo.git",
            commit="a" * 40,
            base_dir="/project",
            lock_path="/project/graft.lock",
        )

        assert result.success is True

        # Check lock file updated
        entries = fake_lock_file.read_lock_file("/project/graft.lock")
        assert "my-dep" in entries
        assert entries["my-dep"].ref == "v2.0.0"
        assert entries["my-dep"].commit == "a" * 40

    def test_executes_migration_command(
        self,
        fake_snapshot,
        fake_executor,
        fake_lock_file,
        config_with_change,
    ):
        """Should execute migration command."""
        fake_lock_file.write_lock_file("/project/graft.lock", {})

        upgrade_dependency(
            fake_snapshot,
            fake_executor,
            fake_lock_file,
            config_with_change,
            dep_name="my-dep",
            to_ref="v2.0.0",
            source="git@example.com:repo.git",
            commit="a" * 40,
            base_dir="/project",
            lock_path="/project/graft.lock",
        )

        assert len(fake_executor.executions) == 2
        assert fake_executor.executions[0]["command"] == "echo migrating"

    def test_executes_verify_command(
        self,
        fake_snapshot,
        fake_executor,
        fake_lock_file,
        config_with_change,
    ):
        """Should execute verification command."""
        fake_lock_file.write_lock_file("/project/graft.lock", {})

        upgrade_dependency(
            fake_snapshot,
            fake_executor,
            fake_lock_file,
            config_with_change,
            dep_name="my-dep",
            to_ref="v2.0.0",
            source="git@example.com:repo.git",
            commit="a" * 40,
            base_dir="/project",
            lock_path="/project/graft.lock",
        )

        assert len(fake_executor.executions) == 2
        assert fake_executor.executions[1]["command"] == "echo verifying"

    def test_rollback_on_migration_failure(
        self,
        fake_snapshot,
        fake_executor,
        fake_lock_file,
        config_with_change,
    ):
        """Should rollback all changes if migration fails."""
        fake_lock_file.write_lock_file("/project/graft.lock", {})

        # Make migration fail
        fake_executor.set_next_result(1, "", "migration failed")

        result = upgrade_dependency(
            fake_snapshot,
            fake_executor,
            fake_lock_file,
            config_with_change,
            dep_name="my-dep",
            to_ref="v2.0.0",
            source="git@example.com:repo.git",
            commit="a" * 40,
            base_dir="/project",
            lock_path="/project/graft.lock",
        )

        assert result.success is False
        assert "Migration failed" in result.error
        assert result.migration_result is not None
        assert result.migration_result.success is False

        # Lock file should not be updated
        entries = fake_lock_file.read_lock_file("/project/graft.lock")
        assert "my-dep" not in entries

    def test_rollback_on_verification_failure(
        self,
        fake_snapshot,
        fake_executor,
        fake_lock_file,
    ):
        """Should rollback if verification fails."""
        # Create a config where only verification is defined (no migration)
        # This way we can make just the verify command fail
        config = GraftConfig(
            api_version="graft/v0",
            commands={
                "verify": Command(name="verify", run="echo verifying", description="Verification"),
            },
            changes={
                "v2.0.0": Change(
                    ref="v2.0.0",
                    type="breaking",
                    description="Breaking change",
                    verify="verify",
                ),
            },
        )

        fake_lock_file.write_lock_file("/project/graft.lock", {})

        # Make verify fail
        fake_executor.set_next_result(1, "", "verify failed")

        result = upgrade_dependency(
            fake_snapshot,
            fake_executor,
            fake_lock_file,
            config,
            dep_name="my-dep",
            to_ref="v2.0.0",
            source="git@example.com:repo.git",
            commit="a" * 40,
            base_dir="/project",
            lock_path="/project/graft.lock",
        )

        assert result.success is False
        assert "Verification failed" in result.error
        assert result.verify_result is not None
        assert result.verify_result.success is False

        # Lock file should not be updated
        entries = fake_lock_file.read_lock_file("/project/graft.lock")
        assert "my-dep" not in entries

    def test_skip_migration_option(
        self,
        fake_snapshot,
        fake_executor,
        fake_lock_file,
        config_with_change,
    ):
        """Should skip migration if skip_migration=True."""
        fake_lock_file.write_lock_file("/project/graft.lock", {})

        result = upgrade_dependency(
            fake_snapshot,
            fake_executor,
            fake_lock_file,
            config_with_change,
            dep_name="my-dep",
            to_ref="v2.0.0",
            source="git@example.com:repo.git",
            commit="a" * 40,
            base_dir="/project",
            lock_path="/project/graft.lock",
            skip_migration=True,
        )

        assert result.success is True
        assert result.migration_result is None
        # Only verify should have run
        assert len(fake_executor.executions) == 1
        assert fake_executor.executions[0]["command"] == "echo verifying"

    def test_skip_verify_option(
        self,
        fake_snapshot,
        fake_executor,
        fake_lock_file,
        config_with_change,
    ):
        """Should skip verification if skip_verify=True."""
        fake_lock_file.write_lock_file("/project/graft.lock", {})

        result = upgrade_dependency(
            fake_snapshot,
            fake_executor,
            fake_lock_file,
            config_with_change,
            dep_name="my-dep",
            to_ref="v2.0.0",
            source="git@example.com:repo.git",
            commit="a" * 40,
            base_dir="/project",
            lock_path="/project/graft.lock",
            skip_verify=True,
        )

        assert result.success is True
        assert result.verify_result is None
        # Only migration should have run
        assert len(fake_executor.executions) == 1
        assert fake_executor.executions[0]["command"] == "echo migrating"

    def test_upgrade_without_migration_or_verify(
        self,
        fake_snapshot,
        fake_executor,
        fake_lock_file,
    ):
        """Should upgrade successfully even without migration/verify."""
        config = GraftConfig(
            api_version="graft/v0",
            changes={
                "v1.1.0": Change(
                    ref="v1.1.0",
                    type="feature",
                    description="New feature",
                ),
            },
        )

        fake_lock_file.write_lock_file("/project/graft.lock", {})

        result = upgrade_dependency(
            fake_snapshot,
            fake_executor,
            fake_lock_file,
            config,
            dep_name="my-dep",
            to_ref="v1.1.0",
            source="git@example.com:repo.git",
            commit="d" * 40,
            base_dir="/project",
            lock_path="/project/graft.lock",
        )

        assert result.success is True
        assert result.migration_result is None
        assert result.verify_result is None
        assert len(fake_executor.executions) == 0

    def test_error_if_change_not_found(
        self,
        fake_snapshot,
        fake_executor,
        fake_lock_file,
        config_with_change,
    ):
        """Should return error if target ref not in changes."""
        result = upgrade_dependency(
            fake_snapshot,
            fake_executor,
            fake_lock_file,
            config_with_change,
            dep_name="my-dep",
            to_ref="v999.0.0",  # Non-existent
            source="git@example.com:repo.git",
            commit="a" * 40,
            base_dir="/project",
            lock_path="/project/graft.lock",
        )

        assert result.success is False
        assert "Change not found" in result.error

    def test_preserves_snapshot_id_when_auto_cleanup_false(
        self,
        fake_snapshot,
        fake_executor,
        fake_lock_file,
        config_with_change,
    ):
        """Should preserve snapshot when auto_cleanup=False."""
        fake_lock_file.write_lock_file("/project/graft.lock", {})

        result = upgrade_dependency(
            fake_snapshot,
            fake_executor,
            fake_lock_file,
            config_with_change,
            dep_name="my-dep",
            to_ref="v2.0.0",
            source="git@example.com:repo.git",
            commit="a" * 40,
            base_dir="/project",
            lock_path="/project/graft.lock",
            auto_cleanup=False,
        )

        assert result.success is True
        assert result.snapshot_id is not None
        assert fake_snapshot.snapshot_exists(result.snapshot_id)


class TestRollbackUpgrade:
    """Tests for rollback_upgrade()."""

    def test_restores_snapshot(self):
        """Should restore files from snapshot."""
        fake = FakeSnapshot()
        fake.set_file_content("file.txt", "original")

        # Create snapshot
        snapshot_id = fake.create_snapshot(["file.txt"], "/base")

        # Modify file
        fake.set_file_content("file.txt", "modified")

        # Rollback
        success = rollback_upgrade(fake, snapshot_id)

        assert success is True
        assert fake.get_file_content("file.txt") == "original"

    def test_raises_error_if_snapshot_not_found(self):
        """Should raise ValueError if snapshot doesn't exist."""
        fake = FakeSnapshot()

        with pytest.raises(ValueError, match="Snapshot not found"):
            rollback_upgrade(fake, "nonexistent")
