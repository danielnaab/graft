"""Dependency resolution service.

Service functions for resolving knowledge base dependencies.

Uses flat-only resolution model (Decision 0007): only direct dependencies
declared in graft.yaml are resolved. There is no transitive resolution.

Dependency Resolution Workflow
=============================

Dependencies can be managed in two ways, depending on the deps_directory location:

1. **Submodule-based resolution** (preferred, when deps_directory is inside the repo):
   - Dependencies are added as git submodules via `git submodule add`
   - Tracked in .gitmodules file
   - Supports `git clone --recursive` for easy setup

2. **Clone-based resolution** (fallback, when deps_directory is outside the repo):
   - Dependencies are cloned with `git clone`
   - Not tracked in parent repo's git history
   - Used when deps_directory is "../" or another location outside the repo

The workflow automatically detects which approach to use based on whether
deps_directory is inside the current git repository.

Legacy Clone Handling
====================

If an existing git clone (not a submodule) is detected, resolution will fail
with a clear error message. To fix this:

1. Delete the .graft/ directory: `rm -rf .graft/`
2. Run `graft resolve` again
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


def _can_use_submodules(deps_directory: str) -> bool:
    """Check if submodules can be used for the given deps directory.

    Submodules require the deps_directory to be inside the git repository.
    If the deps_directory is outside the repo (e.g., "../"), we must use
    regular git clones instead.

    Args:
        deps_directory: The configured deps directory

    Returns:
        True if submodules can be used, False if regular clones should be used
    """
    import subprocess

    try:
        # Get the git repository root
        result = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            capture_output=True,
            text=True,
            check=False,
        )
        if result.returncode != 0:
            # Not in a git repo - can't use submodules
            return False

        repo_root = Path(result.stdout.strip()).resolve()
        deps_path = Path(deps_directory).resolve()

        # Check if deps_directory is inside or equal to the repo root
        try:
            deps_path.relative_to(repo_root)
            return True
        except ValueError:
            # deps_path is not inside repo_root
            return False

    except Exception:
        # If we can't determine, assume no submodules
        return False


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


def _ensure_submodule_at_ref(
    ctx: DependencyContext,
    name: str,
    url: str,
    local_path: str,
    ref: str,
) -> str:
    """Ensure a submodule exists at the specified ref and return its commit.

    This is the core submodule resolution logic, shared between resolve_dependency
    and resolve_to_lock_entries.

    Args:
        ctx: Dependency context
        name: Dependency name (for error messages)
        url: Git URL for the dependency
        local_path: Path where dependency should be placed
        ref: Git ref to checkout

    Returns:
        The resolved commit hash

    Raises:
        DependencyResolutionError: If resolution fails
    """
    if ctx.git.is_submodule(local_path):
        # Update existing submodule
        ctx.git.update_submodule(local_path, init=True)

        # Fetch all refs from remote to get any new commits
        ctx.git.fetch_all(local_path)

        # Try to resolve the ref - prefer origin/<ref> for branches to get
        # the latest from remote, fall back to <ref> for tags/commits
        try:
            resolved_commit = ctx.git.resolve_ref(local_path, f"origin/{ref}")
        except ValueError:
            # Not a remote branch, try as-is (tag or commit)
            resolved_commit = ctx.git.resolve_ref(local_path, ref)

        current_commit = ctx.git.get_current_commit(local_path)
        if current_commit != resolved_commit:
            ctx.git.checkout(local_path, resolved_commit)

    elif ctx.filesystem.exists(local_path):
        # Exists but not a submodule - error with clear message
        if ctx.git.is_repository(local_path):
            raise DependencyResolutionError(
                name,
                f"Legacy clone detected at {local_path}. "
                f"Delete it and re-run resolve: rm -rf {local_path}",
            )
        else:
            raise DependencyResolutionError(
                name,
                f"Path exists but is not a git repository: {local_path}",
            )
    else:
        # Add new submodule
        ctx.git.add_submodule(url=url, path=local_path, ref=ref)

        # Checkout specific commit for lock file
        resolved_commit = ctx.git.resolve_ref(local_path, ref)
        ctx.git.checkout(local_path, resolved_commit)

    # Return the current commit
    return ctx.git.get_current_commit(local_path)


def _resolve_with_submodule(
    ctx: DependencyContext,
    spec: DependencySpec,
    local_path: str,
) -> None:
    """Resolve dependency using git submodules.

    Args:
        ctx: Dependency context
        spec: Dependency specification
        local_path: Path where dependency should be placed

    Raises:
        DependencyResolutionError: If path exists but is not a submodule
    """
    _ensure_submodule_at_ref(
        ctx=ctx,
        name=spec.name,
        url=spec.git_url.url,
        local_path=local_path,
        ref=spec.git_ref.ref,
    )


def _resolve_with_clone(
    ctx: DependencyContext,
    spec: DependencySpec,
    local_path: str,
) -> None:
    """Resolve dependency using regular git clone.

    Used when deps_directory is outside the repository and submodules
    cannot be used.

    Args:
        ctx: Dependency context
        spec: Dependency specification
        local_path: Path where dependency should be placed
    """
    if ctx.filesystem.exists(local_path):
        if ctx.git.is_repository(local_path):
            # Update existing clone
            ctx.git.fetch(local_path, spec.git_ref.ref)
        else:
            # Path exists but isn't a git repo - error
            raise DependencyResolutionError(
                spec.name,
                f"Path exists but is not a git repository: {local_path}",
            )
    else:
        # Clone new repository
        ctx.git.clone(
            url=spec.git_url.url,
            destination=local_path,
            ref=spec.git_ref.ref,
        )


def resolve_dependency(
    ctx: DependencyContext,
    spec: DependencySpec,
) -> DependencyResolution:
    """Resolve a single dependency.

    Uses git submodules for dependency management when possible. Falls back to
    regular git clones when deps_directory is outside the repository.

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

    # Determine local path
    local_path = f"{ctx.deps_directory}/{spec.name}"

    # Check if we can use submodules (deps_directory must be inside the repo)
    use_submodules = _can_use_submodules(ctx.deps_directory)

    try:
        # Mark as cloning
        resolution.mark_cloning()

        if use_submodules:
            # Submodule-based resolution
            _resolve_with_submodule(ctx, spec, local_path)
        else:
            # Clone-based resolution (for deps outside the repository)
            _resolve_with_clone(ctx, spec, local_path)

        # Mark as resolved (convert to absolute path for resolution)
        absolute_path = str(Path(local_path).resolve())
        resolution.mark_resolved(absolute_path)

        # Create symlink if using custom deps_directory
        symlink_path = _create_symlink_if_needed(
            ctx.deps_directory, spec.name, absolute_path
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
    Uses git submodules as the cloning layer.

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
        local_path = f"{ctx.deps_directory}/{name}"

        try:
            # Use shared helper to ensure submodule is at correct ref
            resolved_commit = _ensure_submodule_at_ref(
                ctx=ctx,
                name=name,
                url=spec.git_url.url,
                local_path=local_path,
                ref=spec.git_ref.ref,
            )

            # Create symlink if using custom deps_directory
            absolute_path = str(Path(local_path).resolve())
            _create_symlink_if_needed(ctx.deps_directory, name, absolute_path)

            # Create lock entry
            lock_entry = LockEntry(
                source=spec.git_url.url,
                ref=spec.git_ref.ref,
                commit=resolved_commit,
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
