"""Query service for read-only dependency operations.

Service functions for querying dependency status, changes, and details.
"""

from dataclasses import dataclass
from datetime import datetime

from graft.domain.change import Change
from graft.domain.command import Command
from graft.domain.config import GraftConfig
from graft.protocols.lock_file import LockFile


@dataclass(frozen=True)
class DependencyStatus:
    """Status of a single dependency.

    Attributes:
        name: Dependency name
        current_ref: Currently consumed ref
        consumed_at: Timestamp when consumed
        commit: Current commit hash
    """

    name: str
    current_ref: str
    consumed_at: datetime
    commit: str


def get_all_status(
    lock_file: LockFile, lock_path: str
) -> list[DependencyStatus]:
    """Get status for all dependencies.

    Args:
        lock_file: Lock file operations protocol
        lock_path: Path to graft.lock

    Returns:
        List of DependencyStatus for all dependencies
        Empty list if lock file doesn't exist
    """
    # Read lock file
    if not lock_file.lock_file_exists(lock_path):
        return []

    entries = lock_file.read_lock_file(lock_path)

    # Convert to status objects
    statuses = []
    for name, entry in entries.items():
        status = DependencyStatus(
            name=name,
            current_ref=entry.ref,
            consumed_at=entry.consumed_at,
            commit=entry.commit,
        )
        statuses.append(status)

    return statuses


def get_dependency_status(
    lock_file: LockFile, lock_path: str, dep_name: str
) -> DependencyStatus | None:
    """Get status for a single dependency.

    Args:
        lock_file: Lock file operations protocol
        lock_path: Path to graft.lock
        dep_name: Name of dependency

    Returns:
        DependencyStatus if found, None otherwise
    """
    # Read lock file
    if not lock_file.lock_file_exists(lock_path):
        return None

    entries = lock_file.read_lock_file(lock_path)

    if dep_name not in entries:
        return None

    entry = entries[dep_name]
    return DependencyStatus(
        name=dep_name,
        current_ref=entry.ref,
        consumed_at=entry.consumed_at,
        commit=entry.commit,
    )


def get_changes_for_dependency(config: GraftConfig) -> list[Change]:
    """Get all changes from a dependency's config.

    Args:
        config: Parsed graft.yaml configuration

    Returns:
        List of changes in declaration order
    """
    return list(config.changes.values())


def get_changes_in_range(
    config: GraftConfig,
    from_ref: str | None = None,
    to_ref: str | None = None,
) -> list[Change]:
    """Get changes in a ref range.

    Note: This simplified implementation returns all changes.
    A full implementation would need git integration to determine
    which changes fall between from_ref and to_ref.

    Args:
        config: Parsed graft.yaml configuration
        from_ref: Starting ref (exclusive)
        to_ref: Ending ref (inclusive)

    Returns:
        List of changes in range
    """
    # For now, return all changes
    # TODO: Filter based on git ref ordering once git integration is added
    return get_changes_for_dependency(config)


def filter_changes_by_type(
    changes: list[Change], change_type: str
) -> list[Change]:
    """Filter changes by type.

    Args:
        changes: List of changes to filter
        change_type: Type to filter by (breaking, feature, fix, etc.)

    Returns:
        Filtered list of changes
    """
    return [c for c in changes if c.type == change_type]


def filter_breaking_changes(changes: list[Change]) -> list[Change]:
    """Filter to only breaking changes.

    Args:
        changes: List of changes to filter

    Returns:
        List of breaking changes
    """
    return [c for c in changes if c.is_breaking()]


def get_change_by_ref(config: GraftConfig, ref: str) -> Change | None:
    """Get a specific change by ref.

    Args:
        config: Parsed graft.yaml configuration
        ref: Ref to look up

    Returns:
        Change if found, None otherwise
    """
    return config.changes.get(ref)


@dataclass(frozen=True)
class ChangeDetails:
    """Detailed information about a change.

    Attributes:
        change: The change object
        migration_command: Migration command details (if any)
        verify_command: Verification command details (if any)
    """

    change: Change
    migration_command: Command | None = None
    verify_command: Command | None = None


def get_change_details(config: GraftConfig, ref: str) -> ChangeDetails | None:
    """Get detailed information about a change.

    Args:
        config: Parsed graft.yaml configuration
        ref: Ref of change to get details for

    Returns:
        ChangeDetails if change found, None otherwise

    Note:
        Command existence is validated by GraftConfig.__post_init__,
        so we can safely assume commands exist if referenced.
    """
    # Get change
    change = get_change_by_ref(config, ref)
    if not change:
        return None

    # Get command details (validated by GraftConfig, so safe to use .get())
    migration_command = None
    if change.migration:
        migration_command = config.commands.get(change.migration)

    verify_command = None
    if change.verify:
        verify_command = config.commands.get(change.verify)

    return ChangeDetails(
        change=change,
        migration_command=migration_command,
        verify_command=verify_command,
    )
