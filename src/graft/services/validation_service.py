"""Validation service for graft.yaml and graft.lock.

Provides functions for validating configuration files and lock state.
"""

from dataclasses import dataclass

from graft.domain.config import GraftConfig
from graft.domain.lock_entry import LockEntry
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

    for ref in config.changes.keys():
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
