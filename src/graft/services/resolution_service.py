"""Dependency resolution service.

Service functions for resolving knowledge base dependencies.

Uses flat-only resolution model (Decision 0007): only direct dependencies
declared in graft.yaml are resolved. There is no transitive resolution.
"""

import os
from datetime import UTC, datetime
from pathlib import Path

from graft.domain.config import GraftConfig
from graft.domain.dependency import DependencyResolution, DependencySpec, DependencyStatus
from graft.domain.exceptions import DependencyResolutionError
from graft.domain.lock_entry import LockEntry
from graft.services.dependency_context import DependencyContext

DEFAULT_DEPS_DIRECTORY = ".graft"


def _create_symlink_if_needed(
    deps_directory: str,
    name: str,
    local_path: str,
) -> str | None:
    """Create symlink in .graft/ if using a custom deps_directory.

    When deps_directory is not the default (.graft), creates a symlink
    at .graft/<name> pointing to the actual checkout location. This
    ensures stable paths for linking regardless of where deps are stored.

    Args:
        deps_directory: The configured deps directory
        name: Dependency name
        local_path: Absolute path to the actual checkout

    Returns:
        Path to created symlink, or None if not needed
    """
    # Only create symlink if using a non-default directory
    if deps_directory == DEFAULT_DEPS_DIRECTORY:
        return None

    # Ensure .graft directory exists
    graft_dir = Path(DEFAULT_DEPS_DIRECTORY)
    graft_dir.mkdir(parents=True, exist_ok=True)

    symlink_path = graft_dir / name

    # Remove existing symlink if it exists
    if symlink_path.is_symlink():
        symlink_path.unlink()
    elif symlink_path.exists():
        # Path exists but isn't a symlink - don't overwrite
        return None

    # Only create symlink if target actually exists (avoids test artifacts)
    target = Path(local_path)
    if not target.exists():
        return None

    # Create relative symlink from .graft/<name> to actual location
    # Use relative path for portability
    symlink_path.symlink_to(os.path.relpath(target, graft_dir))

    return str(symlink_path.absolute())


def resolve_dependency(
    ctx: DependencyContext,
    spec: DependencySpec,
) -> DependencyResolution:
    """Resolve a single dependency.

    Clones repository if it doesn't exist, or fetches if it does.

    Args:
        ctx: Dependency context
        spec: Dependency specification

    Returns:
        DependencyResolution with status and local path

    Example:
        >>> from graft.adapters.git import SubprocessGitOperations
        >>> from graft.adapters.filesystem import RealFileSystem
        >>> ctx = DependencyContext(
        ...     filesystem=RealFileSystem(),
        ...     git=SubprocessGitOperations(),
        ...     deps_directory=".."
        ... )
        >>> spec = DependencySpec(...)
        >>> resolution = resolve_dependency(ctx, spec)
        >>> resolution.status
        <DependencyStatus.RESOLVED: 'resolved'>
    """
    # Create resolution object
    resolution = DependencyResolution(
        spec=spec,
        status=DependencyStatus.PENDING,
    )

    # Determine local path (resolve to absolute for clarity)
    local_path = str((Path(ctx.deps_directory) / spec.name).resolve())

    try:
        # Mark as cloning
        resolution.mark_cloning()

        # Check if already exists
        if ctx.filesystem.exists(local_path):
            # Repository exists - fetch and checkout
            if ctx.git.is_repository(local_path):
                ctx.git.fetch(local_path, spec.git_ref.ref)
            else:
                # Path exists but isn't a git repo - error
                raise DependencyResolutionError(
                    spec.name,
                    f"Path exists but is not a git repository: {local_path}",
                )
        else:
            # Clone fresh
            ctx.git.clone(
                url=spec.git_url.url,
                destination=local_path,
                ref=spec.git_ref.ref,
            )

        # Mark as resolved
        resolution.mark_resolved(local_path)

        # Create symlink if using custom deps_directory
        symlink_path = _create_symlink_if_needed(
            ctx.deps_directory, spec.name, local_path
        )
        if symlink_path:
            resolution.symlink_path = symlink_path

    except DependencyResolutionError as e:
        resolution.mark_failed(e.reason)

    except Exception as e:
        resolution.mark_failed(f"Unexpected error: {e}")

    return resolution


def resolve_all_dependencies(
    ctx: DependencyContext,
    config: GraftConfig,
) -> list[DependencyResolution]:
    """Resolve all dependencies from configuration.

    Uses flat-only resolution model (Decision 0007): only direct dependencies
    declared in graft.yaml are resolved. There is no transitive resolution.

    Resolves each dependency in sequence.
    Continues on failure to attempt all dependencies.

    Args:
        ctx: Dependency context
        config: Parsed configuration

    Returns:
        List of dependency resolutions

    Example:
        >>> ctx = DependencyContext(...)
        >>> config = parse_graft_yaml(ctx, "graft.yaml")
        >>> resolutions = resolve_all_dependencies(ctx, config)
        >>> all(r.status == DependencyStatus.RESOLVED for r in resolutions)
        True
    """
    resolutions: list[DependencyResolution] = []

    for spec in config.dependencies.values():
        resolution = resolve_dependency(ctx, spec)
        resolutions.append(resolution)

    return resolutions


def resolve_to_lock_entries(
    ctx: DependencyContext,
    config: GraftConfig,
) -> dict[str, LockEntry]:
    """Resolve all dependencies and return as lock entries.

    Uses flat-only resolution model (Decision 0007): only direct dependencies
    declared in graft.yaml are resolved. There is no transitive resolution.

    Args:
        ctx: Dependency context
        config: Parsed configuration with dependencies

    Returns:
        Dictionary mapping dependency name to LockEntry

    Raises:
        DependencyResolutionError: If resolution fails

    Example:
        >>> ctx = DependencyContext(...)
        >>> config = parse_graft_yaml(ctx, "graft.yaml")
        >>> entries = resolve_to_lock_entries(ctx, config)
        >>> entries["my-dep"].ref
        'v1.0.0'
    """
    lock_entries: dict[str, LockEntry] = {}
    consumed_at = datetime.now(UTC)

    for name, spec in config.dependencies.items():
        # Determine local path
        local_path = str(Path(ctx.deps_directory) / name)

        try:
            # Clone or fetch
            if ctx.filesystem.exists(local_path):
                if ctx.git.is_repository(local_path):
                    ctx.git.fetch(local_path, spec.git_ref.ref)
                else:
                    raise DependencyResolutionError(
                        name,
                        f"Path exists but is not a git repository: {local_path}",
                    )
            else:
                ctx.git.clone(
                    url=spec.git_url.url,
                    destination=local_path,
                    ref=spec.git_ref.ref,
                )

            # Get commit hash by resolving the ref
            commit_hash = ctx.git.resolve_ref(local_path, spec.git_ref.ref)

            # Create symlink if using custom deps_directory
            _create_symlink_if_needed(ctx.deps_directory, name, local_path)

            # Create lock entry
            lock_entry = LockEntry(
                source=spec.git_url.url,
                ref=spec.git_ref.ref,
                commit=commit_hash,
                consumed_at=consumed_at,
            )

            lock_entries[name] = lock_entry

        except DependencyResolutionError:
            raise
        except Exception as e:
            raise DependencyResolutionError(
                name, f"Failed to resolve dependency: {e}"
            ) from e

    return lock_entries
