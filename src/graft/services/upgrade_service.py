"""Upgrade service functions.

Service layer for atomic dependency upgrades with rollback support.
"""

from dataclasses import dataclass

from graft.domain.config import GraftConfig
from graft.protocols.command_executor import CommandExecutor, CommandResult
from graft.protocols.lock_file import LockFile
from graft.protocols.snapshot import Snapshot
from graft.services.command_service import execute_command_by_name
from graft.services.lock_service import update_dependency_lock
from graft.services.snapshot_service import (
    cleanup_snapshot,
    create_workspace_snapshot,
    get_snapshot_paths_for_dependency,
    restore_workspace_snapshot,
)


@dataclass(frozen=True)
class UpgradeResult:
    """Result of an upgrade operation.

    Attributes:
        success: Whether upgrade succeeded
        snapshot_id: ID of snapshot created (for rollback)
        migration_result: Result of migration command (if run)
        verify_result: Result of verification command (if run)
        error: Error message if failed
    """

    success: bool
    snapshot_id: str | None = None
    migration_result: CommandResult | None = None
    verify_result: CommandResult | None = None
    error: str | None = None


def upgrade_dependency(
    snapshot: Snapshot,
    executor: CommandExecutor,
    lock_file: LockFile,
    config: GraftConfig,
    dep_name: str,
    to_ref: str,
    source: str,
    commit: str,
    base_dir: str,
    lock_path: str,
    skip_migration: bool = False,
    skip_verify: bool = False,
    auto_cleanup: bool = True,
) -> UpgradeResult:
    """Upgrade dependency to new version with atomic rollback.

    This is the main upgrade operation that:
    1. Creates snapshot for rollback
    2. Runs migration command (if defined)
    3. Runs verification command (if defined)
    4. Updates lock file
    5. On failure: rolls back all changes

    Args:
        snapshot: Snapshot protocol for creating backups
        executor: Command executor protocol
        lock_file: Lock file operations protocol
        config: Dependency's GraftConfig
        dep_name: Name of dependency
        to_ref: Target ref to upgrade to
        source: Git URL or path
        commit: Resolved commit hash for to_ref
        base_dir: Base directory for command execution
        lock_path: Path to graft.lock
        skip_migration: Skip migration command
        skip_verify: Skip verification command
        auto_cleanup: Automatically delete snapshot on success

    Returns:
        UpgradeResult with success status and details

    Raises:
        ValueError: If target ref not found in config
    """
    # Get change details
    if not config.has_change(to_ref):
        return UpgradeResult(
            success=False,
            error=f"Change not found: {to_ref}",
        )

    change = config.get_change(to_ref)

    # Step 1: Create snapshot for rollback
    snapshot_paths = get_snapshot_paths_for_dependency(dep_name)
    try:
        snapshot_id = create_workspace_snapshot(snapshot, snapshot_paths, base_dir)
    except (FileNotFoundError, OSError) as e:
        return UpgradeResult(
            success=False,
            error=f"Failed to create snapshot: {e}",
        )

    try:
        # Step 2: Run migration command (if defined and not skipped)
        migration_result = None
        if change.migration and not skip_migration:
            try:
                migration_result = execute_command_by_name(
                    executor,
                    config.commands,
                    change.migration,
                    base_dir=base_dir,
                )
                if not migration_result.success:
                    raise RuntimeError(
                        f"Migration command failed with exit code {migration_result.exit_code}"
                    )
            except (KeyError, RuntimeError, OSError) as e:
                # Rollback on migration failure
                restore_workspace_snapshot(snapshot, snapshot_id)
                return UpgradeResult(
                    success=False,
                    snapshot_id=snapshot_id,
                    migration_result=migration_result,
                    error=f"Migration failed: {e}",
                )

        # Step 3: Run verification command (if defined and not skipped)
        verify_result = None
        if change.verify and not skip_verify:
            try:
                verify_result = execute_command_by_name(
                    executor,
                    config.commands,
                    change.verify,
                    base_dir=base_dir,
                )
                if not verify_result.success:
                    raise RuntimeError(
                        f"Verification command failed with exit code {verify_result.exit_code}"
                    )
            except (KeyError, RuntimeError, OSError) as e:
                # Rollback on verification failure
                restore_workspace_snapshot(snapshot, snapshot_id)
                return UpgradeResult(
                    success=False,
                    snapshot_id=snapshot_id,
                    migration_result=migration_result,
                    verify_result=verify_result,
                    error=f"Verification failed: {e}",
                )

        # Step 4: Update lock file
        try:
            update_dependency_lock(
                lock_file,
                lock_path,
                dep_name,
                source,
                to_ref,
                commit,
            )
        except OSError as e:
            # Rollback on lock file update failure
            restore_workspace_snapshot(snapshot, snapshot_id)
            return UpgradeResult(
                success=False,
                snapshot_id=snapshot_id,
                migration_result=migration_result,
                verify_result=verify_result,
                error=f"Failed to update lock file: {e}",
            )

        # Success! Optionally cleanup snapshot
        if auto_cleanup:
            try:
                cleanup_snapshot(snapshot, snapshot_id)
                snapshot_id = None  # Mark as cleaned up
            except ValueError:
                # Snapshot cleanup failed, but upgrade succeeded
                pass

        return UpgradeResult(
            success=True,
            snapshot_id=snapshot_id,
            migration_result=migration_result,
            verify_result=verify_result,
        )

    except Exception as e:
        # Catch-all rollback for unexpected errors
        try:
            restore_workspace_snapshot(snapshot, snapshot_id)
        except (ValueError, OSError):
            # Rollback failed - snapshot may be corrupted
            pass

        return UpgradeResult(
            success=False,
            snapshot_id=snapshot_id,
            error=f"Unexpected error: {e}",
        )


def rollback_upgrade(
    snapshot: Snapshot,
    snapshot_id: str,
) -> bool:
    """Manually rollback an upgrade using snapshot.

    Args:
        snapshot: Snapshot protocol
        snapshot_id: Snapshot ID to restore

    Returns:
        True if rollback succeeded

    Raises:
        ValueError: If snapshot doesn't exist
        OSError: If unable to restore snapshot
    """
    restore_workspace_snapshot(snapshot, snapshot_id)
    return True
