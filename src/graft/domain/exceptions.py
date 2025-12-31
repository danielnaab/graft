"""Domain exceptions - business rule violations."""


class DomainError(Exception):
    """Base exception for all domain errors.

    Use this for business rule violations and domain-specific errors.
    """


class ValidationError(DomainError):
    """Validation error - data doesn't meet business rules.

    Raised when:
    - Value objects receive invalid data
    - Entities fail validation
    - Business constraints are violated
    """


class EntityNotFoundError(DomainError):
    """Entity not found error.

    Raised when attempting to retrieve entity that doesn't exist.
    """

    def __init__(self, entity_id: str, entity_type: str = "Entity") -> None:
        """Initialize error with entity details.

        Args:
            entity_id: ID of entity that wasn't found
            entity_type: Type of entity (default: "Entity")
        """
        super().__init__(f"{entity_type} not found: {entity_id}")
        self.entity_id = entity_id
        self.entity_type = entity_type


class ConfigurationError(DomainError):
    """Base configuration error.

    Raised when:
    - graft.yaml is malformed
    - Required configuration is missing
    - Configuration violates constraints
    """


class ConfigFileNotFoundError(ConfigurationError):
    """Configuration file not found.

    Raised when graft.yaml doesn't exist in expected location.
    """

    def __init__(self, path: str, suggestion: str = "Create graft.yaml in project root") -> None:
        """Initialize error with file details.

        Args:
            path: Path where config was expected
            suggestion: How to fix the issue
        """
        super().__init__(f"Configuration file not found: {path}")
        self.path = path
        self.suggestion = suggestion


class ConfigParseError(ConfigurationError):
    """Configuration file parsing failed.

    Raised when YAML is malformed or invalid.
    """

    def __init__(self, path: str, reason: str) -> None:
        """Initialize error with parse details.

        Args:
            path: Path to config file
            reason: Why parsing failed
        """
        super().__init__(f"Failed to parse {path}: {reason}")
        self.path = path
        self.reason = reason


class ConfigValidationError(ConfigurationError):
    """Configuration validation failed.

    Raised when config structure violates requirements.
    """

    def __init__(self, path: str, field: str, reason: str) -> None:
        """Initialize error with validation details.

        Args:
            path: Path to config file
            field: Field that failed validation
            reason: Why validation failed
        """
        super().__init__(f"Invalid configuration in {path}: {field} - {reason}")
        self.path = path
        self.field = field
        self.reason = reason


class DependencyResolutionError(DomainError):
    """Base dependency resolution error.

    Raised when unable to resolve a dependency.
    """

    def __init__(self, dependency_name: str, reason: str) -> None:
        """Initialize error with dependency details.

        Args:
            dependency_name: Name of dependency that failed
            reason: Reason for failure
        """
        super().__init__(f"Failed to resolve dependency '{dependency_name}': {reason}")
        self.dependency_name = dependency_name
        self.reason = reason


class GitCloneError(DependencyResolutionError):
    """Git clone operation failed.

    Raised when git clone command fails.
    """

    def __init__(
        self,
        dependency_name: str,
        url: str,
        ref: str,
        stderr: str,
        returncode: int = 1,
    ) -> None:
        """Initialize error with git details.

        Args:
            dependency_name: Name of dependency
            url: Git repository URL
            ref: Git reference (branch/tag/commit)
            stderr: Git command error output
            returncode: Git command exit code
        """
        super().__init__(
            dependency_name,
            f"Git clone failed for {url}#{ref}: {stderr}",
        )
        self.url = url
        self.ref = ref
        self.stderr = stderr
        self.returncode = returncode


class GitFetchError(DependencyResolutionError):
    """Git fetch operation failed.

    Raised when git fetch/checkout command fails.
    """

    def __init__(
        self,
        dependency_name: str,
        repo_path: str,
        ref: str,
        stderr: str,
        returncode: int = 1,
    ) -> None:
        """Initialize error with git details.

        Args:
            dependency_name: Name of dependency
            repo_path: Path to existing repository
            ref: Git reference to fetch
            stderr: Git command error output
            returncode: Git command exit code
        """
        super().__init__(
            dependency_name,
            f"Git fetch failed for {ref} in {repo_path}: {stderr}",
        )
        self.repo_path = repo_path
        self.ref = ref
        self.stderr = stderr
        self.returncode = returncode


class GitAuthenticationError(DependencyResolutionError):
    """Git authentication failed.

    Raised when git operations fail due to authentication issues.
    """

    def __init__(
        self,
        dependency_name: str,
        url: str,
        suggestion: str = "Check SSH keys or credentials",
    ) -> None:
        """Initialize error with auth details.

        Args:
            dependency_name: Name of dependency
            url: Git repository URL
            suggestion: How to fix authentication
        """
        super().__init__(
            dependency_name,
            f"Authentication failed for {url}",
        )
        self.url = url
        self.suggestion = suggestion


class GitNotFoundError(DependencyResolutionError):
    """Git repository or ref not found.

    Raised when repository doesn't exist or ref is invalid.
    """

    def __init__(
        self,
        dependency_name: str,
        url: str,
        ref: str,
    ) -> None:
        """Initialize error with git details.

        Args:
            dependency_name: Name of dependency
            url: Git repository URL
            ref: Git reference that wasn't found
        """
        super().__init__(
            dependency_name,
            f"Repository or ref not found: {url}#{ref}",
        )
        self.url = url
        self.ref = ref
