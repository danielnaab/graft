"""Dependency resolution service.

Service functions for resolving knowledge base dependencies.
"""

from pathlib import Path

from graft.domain.config import GraftConfig
from graft.domain.dependency import DependencyResolution, DependencySpec, DependencyStatus
from graft.domain.exceptions import DependencyResolutionError
from graft.services.dependency_context import DependencyContext


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
