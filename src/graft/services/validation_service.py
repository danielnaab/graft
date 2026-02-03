"""Validation service for graft.yaml and graft.lock.

Provides functions for validating configuration files and lock state.
"""

from dataclasses import dataclass
from pathlib import Path

from graft.domain.config import GraftConfig
from graft.domain.lock_entry import LockEntry
from graft.protocols.filesystem import FileSystem
from graft.protocols.git import GitOperations


@dataclass(frozen=True)
class ValidationError:
    """Represents a validation error.

    Attributes:
        message: Human-readable error message
        severity: 'error' or 'warning'
    """

    message: str
    severity: str = "error"  # 'error' or 'warning'


def validate_config_schema(config: GraftConfig) -> list[ValidationError]:
    """Validate graft.yaml schema and structure.

    Note: Most schema validation (API version, command references) happens in
    GraftConfig.__post_init__, so this function only validates business rules
    that can't be enforced at the domain level.

    Args:
        config: Parsed GraftConfig

    Returns:
        List of validation errors (empty if valid)
    """
    errors = []

    # Check dependencies exist (business rule: require at least one dependency)
    if not config.dependencies:
        errors.append(
            ValidationError(
                "No dependencies defined (at least one dependency required)"
            )
        )

    return errors


def validate_refs_exist(
    config: GraftConfig, git: GitOperations, dep_path: str
) -> list[ValidationError]:
    """Validate that all refs in changes exist in git repository.

    Args:
        config: Parsed GraftConfig
        git: Git operations
        dep_path: Path to dependency's git repository

    Returns:
        List of validation errors
    """
    errors = []

    for ref in config.changes:
        try:
            # Try to resolve ref to commit
            git.resolve_ref(dep_path, ref)
        except Exception as e:
            errors.append(
                ValidationError(f"Ref '{ref}' does not exist in git repository: {e}")
            )

    return errors


def validate_lock_entry(
    entry: LockEntry, git: GitOperations, dep_path: str
) -> list[ValidationError]:
    """Validate a single lock entry.

    Checks that:
    - Ref still exists in git
    - Commit hash is valid format (already validated by LockEntry.__post_init__)

    Args:
        entry: Lock entry to validate
        git: Git operations
        dep_path: Path to dependency's git repository

    Returns:
        List of validation errors and warnings
    """
    errors = []

    # Check ref exists
    try:
        current_commit = git.resolve_ref(dep_path, entry.ref)

        # Warn if commit has changed (ref has moved)
        if current_commit != entry.commit:
            errors.append(
                ValidationError(
                    f"Ref '{entry.ref}' has moved: expected commit {entry.commit[:7]}, "
                    f"now at {current_commit[:7]}",
                    severity="warning",
                )
            )
    except Exception as e:
        errors.append(
            ValidationError(f"Ref '{entry.ref}' does not exist in repository: {e}")
        )

    return errors


def get_validation_summary(errors: list[ValidationError]) -> tuple[list[str], list[str]]:
    """Separate errors and warnings from validation results.

    Args:
        errors: List of ValidationError objects

    Returns:
        Tuple of (errors, warnings) as string lists
    """
    error_messages = [e.message for e in errors if e.severity == "error"]
    warning_messages = [e.message for e in errors if e.severity == "warning"]

    return (error_messages, warning_messages)


@dataclass(frozen=True)
class IntegrityResult:
    """Result of integrity validation for a single dependency.

    Attributes:
        name: Dependency name
        valid: Whether integrity check passed
        expected_commit: Commit from lock file
        actual_commit: Commit found in .graft/ (None if missing)
        message: Human-readable status message
    """

    name: str
    valid: bool
    expected_commit: str
    actual_commit: str | None
    message: str


def validate_integrity(
    filesystem: FileSystem,
    git: GitOperations,
    deps_directory: str,
    lock_entries: dict[str, LockEntry],
) -> list[IntegrityResult]:
    """Validate that .graft/ matches the lock file.

    Compares actual commit hashes in .graft/ against expected
    commits in graft.lock.

    Args:
        filesystem: Filesystem operations
        git: Git operations
        deps_directory: Path to deps directory (e.g., ".graft")
        lock_entries: Dictionary of lock entries

    Returns:
        List of integrity results, one per dependency
    """
    results = []

    for name, entry in sorted(lock_entries.items()):
        local_path = str(Path(deps_directory) / name)

        # Check if dependency exists
        if not filesystem.exists(local_path):
            results.append(
                IntegrityResult(
                    name=name,
                    valid=False,
                    expected_commit=entry.commit,
                    actual_commit=None,
                    message="Dependency not found in .graft/",
                )
            )
            continue

        # Check if it's a git repository
        if not git.is_repository(local_path):
            results.append(
                IntegrityResult(
                    name=name,
                    valid=False,
                    expected_commit=entry.commit,
                    actual_commit=None,
                    message="Path exists but is not a git repository",
                )
            )
            continue

        # Check if it's registered as a submodule
        is_submodule = git.is_submodule(local_path)
        # (Note: legacy clones are still valid, we just track if it's a submodule)

        # Get current commit
        try:
            actual_commit = git.get_current_commit(local_path)
        except Exception as e:
            results.append(
                IntegrityResult(
                    name=name,
                    valid=False,
                    expected_commit=entry.commit,
                    actual_commit=None,
                    message=f"Failed to get commit: {e}",
                )
            )
            continue

        # Compare commits
        if actual_commit == entry.commit:
            message = "Commit matches"
            if not is_submodule:
                message += " (legacy clone - delete .graft/ and re-run resolve)"
            results.append(
                IntegrityResult(
                    name=name,
                    valid=True,
                    expected_commit=entry.commit,
                    actual_commit=actual_commit,
                    message=message,
                )
            )
        else:
            message = f"Commit mismatch: expected {entry.commit[:7]}, got {actual_commit[:7]}"
            if not is_submodule:
                message += " (legacy clone)"
            results.append(
                IntegrityResult(
                    name=name,
                    valid=False,
                    expected_commit=entry.commit,
                    actual_commit=actual_commit,
                    message=message,
                )
            )

    return results


def validate_lock_commits_exist(
    git: GitOperations,
    deps_directory: str,
    lock_entries: dict[str, LockEntry],
) -> list[ValidationError]:
    """Validate that all commits in lock file exist in their repositories.

    This is important because lock file commits may become unavailable if:
    - Force pushes remove commits
    - Repositories are rebased
    - Repositories are deleted and recreated

    Args:
        git: Git operations
        deps_directory: Path to deps directory (e.g., ".graft")
        lock_entries: Dictionary of lock entries

    Returns:
        List of validation errors for missing commits or missing repos
    """
    errors = []

    for name, entry in lock_entries.items():
        local_path = f"{deps_directory}/{name}"

        # Report if repo doesn't exist locally
        if not git.is_repository(local_path):
            errors.append(
                ValidationError(
                    f"Dependency '{name}' not found at {local_path}. "
                    "Run 'graft resolve' to fetch dependencies.",
                    severity="error",
                )
            )
            continue

        try:
            # Try to resolve the locked commit
            git.resolve_ref(local_path, entry.commit)
        except Exception:
            errors.append(
                ValidationError(
                    f"Lock file commit {entry.commit[:8]} for '{name}' is not available. "
                    "The commit may have been removed by a force push or rebase. "
                    "Run 'graft resolve' to update to the latest commit.",
                    severity="error",
                )
            )

    return errors


@dataclass(frozen=True)
class SubmoduleValidationResult:
    """Result of submodule-specific validation.

    Attributes:
        name: Dependency name
        path: Path to dependency
        is_submodule: Whether it's registered as a submodule
        has_uncommitted_changes: Whether submodule has local changes
        is_ahead_of_lock: Whether submodule is ahead of locked commit
        is_behind_ref: Whether submodule is behind tracked ref
        message: Human-readable status message
    """

    name: str
    path: str
    is_submodule: bool
    has_uncommitted_changes: bool | None  # None if couldn't check
    is_ahead_of_lock: bool | None
    is_behind_ref: bool | None
    message: str


def validate_submodule_state(
    git: GitOperations,
    deps_directory: str,
    lock_entries: dict[str, LockEntry],
) -> list[SubmoduleValidationResult]:
    """Validate submodule state against lock file.

    Checks for:
    - Submodule registration status
    - Uncommitted changes in submodules
    - Submodule ahead of locked commit
    - Submodule behind tracked ref

    Args:
        git: Git operations
        deps_directory: Path to deps directory
        lock_entries: Dictionary of lock entries

    Returns:
        List of validation results for each dependency
    """
    results = []

    for name, entry in sorted(lock_entries.items()):
        local_path = f"{deps_directory}/{name}"

        # Check if it's a submodule
        is_submodule = git.is_submodule(local_path)

        if not is_submodule:
            results.append(
                SubmoduleValidationResult(
                    name=name,
                    path=local_path,
                    is_submodule=False,
                    has_uncommitted_changes=None,
                    is_ahead_of_lock=None,
                    is_behind_ref=None,
                    message="Not registered as submodule (legacy clone)",
                )
            )
            continue

        # Check for uncommitted changes
        has_uncommitted_changes = None
        try:
            has_uncommitted_changes = not git.is_working_directory_clean(local_path)
        except Exception:
            pass

        # Check if ahead of locked commit
        is_ahead_of_lock = None
        try:
            current_commit = git.get_current_commit(local_path)
            is_ahead_of_lock = current_commit != entry.commit
        except Exception:
            pass

        # Check if behind tracked ref
        is_behind_ref = None
        try:
            ref_commit = git.resolve_ref(local_path, entry.ref)
            current_commit = git.get_current_commit(local_path)
            is_behind_ref = current_commit != ref_commit
        except Exception:
            pass

        # Build message
        messages = []
        if has_uncommitted_changes:
            messages.append("has uncommitted changes")
        if is_ahead_of_lock:
            messages.append("ahead of lock file")
        if is_behind_ref:
            messages.append(f"behind {entry.ref}")

        if messages:
            message = "; ".join(messages)
        else:
            message = "OK"

        results.append(
            SubmoduleValidationResult(
                name=name,
                path=local_path,
                is_submodule=True,
                has_uncommitted_changes=has_uncommitted_changes,
                is_ahead_of_lock=is_ahead_of_lock,
                is_behind_ref=is_behind_ref,
                message=message,
            )
        )

    return results
