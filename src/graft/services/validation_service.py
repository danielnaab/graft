"""Validation service for graft.yaml and graft.lock.

Provides functions for validating configuration files and lock state.
"""

from dataclasses import dataclass
from enum import Enum

from graft.domain.config import GraftConfig
from graft.domain.lock_entry import LockEntry
from graft.protocols.git import GitOperations


class ErrorType(str, Enum):
    """Types of validation errors."""

    SCHEMA = "schema"
    REF_NOT_FOUND = "ref_not_found"
    REF_MOVED = "ref_moved"
    INTEGRITY_MISMATCH = "integrity_mismatch"
    GENERAL = "general"


@dataclass(frozen=True)
class ValidationError:
    """Represents a validation error.

    Attributes:
        message: Human-readable error message
        severity: 'error' or 'warning'
        error_type: Type of error for programmatic handling
    """

    message: str
    severity: str = "error"  # 'error' or 'warning'
    error_type: ErrorType = ErrorType.GENERAL


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
                "No dependencies defined (at least one dependency required)",
                error_type=ErrorType.SCHEMA,
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
                ValidationError(
                    f"Ref '{ref}' does not exist in git repository: {e}",
                    error_type=ErrorType.REF_NOT_FOUND,
                )
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
                    error_type=ErrorType.REF_MOVED,
                )
            )
    except Exception as e:
        errors.append(
            ValidationError(
                f"Ref '{entry.ref}' does not exist in repository: {e}",
                error_type=ErrorType.REF_NOT_FOUND,
            )
        )

    return errors


def verify_integrity(
    entry: LockEntry, git: GitOperations, dep_path: str
) -> list[ValidationError]:
    """Verify that .graft/ directory matches lock file commit.

    Uses `git rev-parse HEAD` (via resolve_ref) to check the actual commit
    in the cloned dependency directory against the commit recorded in the lock file.

    This is the true integrity check - ensures .graft/ hasn't been modified.

    Args:
        entry: Lock entry with expected commit
        git: Git operations
        dep_path: Path to dependency's git repository in .graft/

    Returns:
        List of validation errors (empty if integrity verified)
    """
    errors = []

    try:
        # Get actual commit in .graft/ directory using git rev-parse HEAD
        actual_commit = git.resolve_ref(dep_path, "HEAD")

        # Compare to lock file
        if actual_commit != entry.commit:
            errors.append(
                ValidationError(
                    f"Integrity mismatch: .graft/ has commit {actual_commit[:7]}, "
                    f"but lock file expects {entry.commit[:7]}",
                    severity="error",
                    error_type=ErrorType.INTEGRITY_MISMATCH,
                )
            )
    except Exception as e:
        errors.append(
            ValidationError(
                f"Failed to verify integrity: {e}",
                error_type=ErrorType.GENERAL,
            )
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
