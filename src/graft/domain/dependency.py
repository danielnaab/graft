"""Domain models for dependency management.

Entities and value objects for managing knowledge base dependencies.
"""

from dataclasses import dataclass
from enum import Enum
from urllib.parse import urlparse

from graft.domain.exceptions import ValidationError


class DependencyStatus(Enum):
    """Status of a dependency resolution.

    Tracks the current state of dependency resolution.
    """

    PENDING = "pending"      # Not yet resolved
    CLONING = "cloning"      # Currently being cloned
    RESOLVED = "resolved"    # Successfully resolved
    FAILED = "failed"        # Resolution failed


@dataclass(frozen=True)
class GitRef:
    """Git reference (branch, tag, or commit SHA).

    Value object representing a git reference.
    Immutable with validation.

    Attributes:
        ref: Reference string (e.g., "main", "v1.0.0", "abc123")
        ref_type: Optional hint about reference type

    Example:
        >>> ref = GitRef("main")
        >>> str(ref)
        'main'
    """

    ref: str
    ref_type: str | None = None  # "branch", "tag", "commit", or None

    def __post_init__(self) -> None:
        """Validate git reference."""
        if not self.ref:
            raise ValidationError("Git ref cannot be empty")
        if not self.ref.strip():
            raise ValidationError("Git ref cannot be only whitespace")

    def __str__(self) -> str:
        """String representation."""
        return self.ref


@dataclass(frozen=True)
class GitUrl:
    """Git repository URL.

    Value object representing a git URL with validation.
    Supports ssh://, https://, http://, and file:// schemes.

    Attributes:
        url: Full git URL

    Example:
        >>> url = GitUrl("ssh://git@github.com/user/repo.git")
        >>> url.scheme
        'ssh'
    """

    url: str

    def __post_init__(self) -> None:
        """Validate and parse git URL."""
        if not self.url:
            raise ValidationError("Git URL cannot be empty")

        # Parse URL - be lenient to support various git URL formats
        # Git supports: ssh://, https://, http://, file://, git@host:path, relative/absolute paths
        parsed = urlparse(self.url)

        # Only validate scheme if one is provided
        # Allow empty scheme for: git@host:path format, relative paths, absolute paths
        if parsed.scheme and parsed.scheme not in ("ssh", "https", "http", "file", "git"):
            raise ValidationError(
                f"Invalid URL scheme: {parsed.scheme}. "
                f"Supported: ssh, https, http, file, git, or no scheme for git@ format/paths"
            )

        # Store parsed components (use object.__setattr__ for frozen dataclass)
        object.__setattr__(self, "_parsed", parsed)

    @property
    def scheme(self) -> str:
        """Get URL scheme."""
        return self._parsed.scheme  # type: ignore

    @property
    def host(self) -> str:
        """Get hostname."""
        return self._parsed.netloc  # type: ignore

    @property
    def path(self) -> str:
        """Get repository path."""
        return self._parsed.path  # type: ignore

    def __str__(self) -> str:
        """String representation."""
        return self.url


@dataclass(frozen=True)
class DependencySpec:
    """Dependency specification from graft.yaml.

    Immutable value object representing a single dependency.

    Attributes:
        name: Dependency name (unique identifier)
        git_url: Git repository URL
        git_ref: Git reference (branch/tag/commit)

    Example:
        >>> spec = DependencySpec(
        ...     name="graft-knowledge",
        ...     git_url=GitUrl("ssh://git@example.com/repo.git"),
        ...     git_ref=GitRef("main")
        ... )
        >>> spec.name
        'graft-knowledge'
    """

    name: str
    git_url: GitUrl
    git_ref: GitRef

    def __post_init__(self) -> None:
        """Validate dependency specification."""
        if not self.name:
            raise ValidationError("Dependency name cannot be empty")
        if len(self.name) > 100:
            raise ValidationError(f"Dependency name too long: {len(self.name)} chars")
        # Name should be valid as directory name
        if "/" in self.name or "\\" in self.name:
            raise ValidationError(
                f"Invalid dependency name: {self.name}. Cannot contain path separators."
            )


@dataclass
class DependencyResolution:
    """Result of resolving a dependency.

    Mutable entity tracking resolution state.
    Has identity based on dependency name.

    Attributes:
        spec: Dependency specification
        status: Current resolution status
        local_path: Path where dependency is cloned (if resolved)
        error_message: Error message if resolution failed

    Example:
        >>> spec = DependencySpec(...)
        >>> resolution = DependencyResolution(spec=spec, status=DependencyStatus.PENDING)
        >>> resolution.mark_resolved("/path/to/repo")
        >>> resolution.status
        <DependencyStatus.RESOLVED: 'resolved'>
    """

    spec: DependencySpec
    status: DependencyStatus
    local_path: str | None = None
    error_message: str | None = None

    @property
    def name(self) -> str:
        """Get dependency name (acts as ID)."""
        return self.spec.name

    def mark_cloning(self) -> None:
        """Mark dependency as currently being cloned."""
        self.status = DependencyStatus.CLONING

    def mark_resolved(self, local_path: str) -> None:
        """Mark dependency as successfully resolved.

        Args:
            local_path: Absolute path to cloned repository
        """
        self.status = DependencyStatus.RESOLVED
        self.local_path = local_path
        self.error_message = None

    def mark_failed(self, error: str) -> None:
        """Mark dependency as failed.

        Args:
            error: Error message describing failure
        """
        self.status = DependencyStatus.FAILED
        self.error_message = error

    def __eq__(self, other: object) -> bool:
        """Resolutions are equal if they have the same dependency name.

        Args:
            other: Object to compare

        Returns:
            True if same dependency name
        """
        if not isinstance(other, DependencyResolution):
            return NotImplemented
        return self.name == other.name

    def __hash__(self) -> int:
        """Hash based on dependency name.

        Returns:
            Hash of dependency name
        """
        return hash(self.name)
