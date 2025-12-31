"""Service context for dependency resolution.

Context object containing all dependencies for dependency resolution services.
"""

from dataclasses import dataclass

from graft.protocols.filesystem import FileSystem
from graft.protocols.git import GitOperations


@dataclass(frozen=True)
class DependencyContext:
    """Service context for dependency resolution.

    Contains all dependencies needed by dependency resolution services.
    Immutable context object passed to service functions.

    Attributes:
        filesystem: Filesystem operations
        git: Git operations
        deps_directory: Base directory for dependencies (e.g., "../")

    Example:
        >>> from graft.adapters.filesystem import RealFileSystem
        >>> from graft.adapters.git import SubprocessGitOperations
        >>> ctx = DependencyContext(
        ...     filesystem=RealFileSystem(),
        ...     git=SubprocessGitOperations(),
        ...     deps_directory=".."
        ... )
    """

    filesystem: FileSystem
    git: GitOperations
    deps_directory: str
