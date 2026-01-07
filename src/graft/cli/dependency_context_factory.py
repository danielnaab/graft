"""Dependency context factory for CLI.

Factory function for creating production dependency contexts.
"""

from graft.adapters.filesystem import RealFileSystem
from graft.adapters.git import SubprocessGitOperations
from graft.services.dependency_context import DependencyContext


def get_dependency_context(deps_directory: str = ".graft") -> DependencyContext:
    """Build production dependency context.

    Creates DependencyContext with real adapters for production use.

    Args:
        deps_directory: Base directory for dependencies (relative to cwd)
            Default: ".graft" (project-local directory)

    Returns:
        DependencyContext with production dependencies

    Example:
        >>> ctx = get_dependency_context()
        >>> # Use in services
        >>> config = parse_graft_yaml(ctx, "graft.yaml")
    """
    return DependencyContext(
        filesystem=RealFileSystem(),
        git=SubprocessGitOperations(),
        deps_directory=deps_directory,
    )
