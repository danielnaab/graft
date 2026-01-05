"""Dependency resolution service.

Service functions for resolving knowledge base dependencies.
"""

from datetime import UTC, datetime
from pathlib import Path

from graft.domain.config import GraftConfig
from graft.domain.dependency import DependencyResolution, DependencySpec, DependencyStatus
from graft.domain.exceptions import DependencyResolutionError
from graft.domain.lock_entry import LockEntry
from graft.services.config_service import parse_graft_yaml
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

    # Determine local path
    local_path = str(Path(ctx.deps_directory) / spec.name)

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


def resolve_all_recursive(
    ctx: DependencyContext,
    config: GraftConfig,
) -> dict[str, LockEntry]:
    """Resolve all dependencies recursively with conflict detection.

    Recursively resolves transitive dependencies, building a complete
    dependency graph. Detects and raises errors for version conflicts.

    This function implements the v2 dependency resolution algorithm:
    1. Clone/fetch all dependencies (direct + transitive)
    2. Build dependency graph with requires/required_by relationships
    3. Detect conflicts when same dependency has different versions
    4. Return flat map suitable for v2 graft.lock format

    Args:
        ctx: Dependency context
        config: Parsed configuration with direct dependencies

    Returns:
        Dictionary mapping dependency name to LockEntry with v2 fields:
        - direct: bool
        - requires: tuple[str, ...]
        - required_by: tuple[str, ...]

    Raises:
        DependencyResolutionError: If version conflict detected or
            resolution fails

    Example:
        >>> ctx = DependencyContext(...)
        >>> config = parse_graft_yaml(ctx, "graft.yaml")
        >>> entries = resolve_all_recursive(ctx, config)
        >>> # entries contains both direct and transitive dependencies
        >>> meta_kb = entries["meta-kb"]
        >>> meta_kb.direct
        True
        >>> # Transitive deps from meta-kb
        >>> standards_kb = entries["standards-kb"]
        >>> standards_kb.direct
        False
        >>> "meta-kb" in standards_kb.required_by
        True
    """
    # Track resolved dependencies: name -> (spec, commit, requires, required_by)
    resolved: dict[str, tuple[DependencySpec, str, list[str], list[str]]] = {}

    # Queue of dependencies to process: (name, spec, is_direct, parent_name)
    queue: list[tuple[str, DependencySpec, bool, str | None]] = []

    # Initialize with direct dependencies
    for name, spec in config.dependencies.items():
        queue.append((name, spec, True, None))

    # Process queue
    while queue:
        dep_name, spec, is_direct, parent_name = queue.pop(0)

        # Check if already resolved
        if dep_name in resolved:
            existing_spec, existing_commit, existing_requires, existing_required_by = (
                resolved[dep_name]
            )

            # Check for conflict: same name, different source or ref
            if (
                existing_spec.git_url.url != spec.git_url.url
                or existing_spec.git_ref.ref != spec.git_ref.ref
            ):
                # Conflict detected!
                parent_info = f" (required by {parent_name})" if parent_name else ""
                existing_parents = (
                    ", ".join(existing_required_by) if existing_required_by else "root"
                )

                raise DependencyResolutionError(
                    dep_name,
                    f"Dependency conflict detected:\n"
                    f"  Dependency: {dep_name}\n"
                    f"  Already resolved: {existing_spec.git_url.url}#{existing_spec.git_ref.ref}\n"
                    f"  Required by: {existing_parents}\n"
                    f"  Conflicting requirement{parent_info}: {spec.git_url.url}#{spec.git_ref.ref}\n"
                    f"\n"
                    f"These versions are incompatible. Please:\n"
                    f"  1. Check if newer versions align\n"
                    f"  2. Contact maintainers about compatibility\n"
                    f"  3. Use only one of the conflicting dependencies",
                )

            # Same version - just update required_by if new parent
            if parent_name and parent_name not in existing_required_by:
                existing_required_by.append(parent_name)

            continue

        # Resolve this dependency (clone/fetch)
        local_path = str(Path(ctx.deps_directory) / dep_name)

        try:
            # Clone or fetch
            if ctx.filesystem.exists(local_path):
                if ctx.git.is_repository(local_path):
                    ctx.git.fetch(local_path, spec.git_ref.ref)
                else:
                    raise DependencyResolutionError(
                        dep_name,
                        f"Path exists but is not a git repository: {local_path}",
                    )
            else:
                ctx.git.clone(
                    url=spec.git_url.url,
                    destination=local_path,
                    ref=spec.git_ref.ref,
                )

            # Get commit hash
            commit_hash = ctx.git.get_commit_hash(local_path)

            # Read transitive dependencies from this dep's graft.yaml
            dep_config_path = str(Path(local_path) / "graft.yaml")
            transitive_deps: dict[str, DependencySpec] = {}

            if ctx.filesystem.exists(dep_config_path):
                try:
                    dep_config = parse_graft_yaml(ctx, dep_config_path)
                    transitive_deps = dep_config.dependencies or {}
                except Exception:
                    # If parsing fails, treat as leaf dependency (no transitive deps)
                    pass

            # Record resolution
            requires_list = list(transitive_deps.keys())
            required_by_list = [parent_name] if parent_name else []

            resolved[dep_name] = (spec, commit_hash, requires_list, required_by_list)

            # Queue transitive dependencies
            for trans_name, trans_spec in transitive_deps.items():
                queue.append((trans_name, trans_spec, False, dep_name))

        except DependencyResolutionError:
            raise
        except Exception as e:
            raise DependencyResolutionError(
                dep_name, f"Failed to resolve dependency: {e}"
            ) from e

    # Build LockEntry dictionary with v2 fields
    lock_entries: dict[str, LockEntry] = {}
    consumed_at = datetime.now(UTC)

    for dep_name, (spec, commit_hash, requires_list, required_by_list) in resolved.items():
        # Determine if this is a direct dependency
        is_direct = dep_name in config.dependencies

        lock_entry = LockEntry(
            source=spec.git_url.url,
            ref=spec.git_ref.ref,
            commit=commit_hash,
            consumed_at=consumed_at,
            direct=is_direct,
            requires=tuple(requires_list),
            required_by=tuple(required_by_list),
        )

        lock_entries[dep_name] = lock_entry

    return lock_entries
