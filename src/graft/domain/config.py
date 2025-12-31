"""Configuration domain models.

Value objects for graft configuration.
"""

from dataclasses import dataclass

from graft.domain.dependency import DependencySpec
from graft.domain.exceptions import ValidationError


@dataclass(frozen=True)
class GraftConfig:
    """Graft configuration from graft.yaml.

    Immutable value object representing entire graft.yaml configuration.

    Attributes:
        api_version: Configuration API version (e.g., "graft/v0")
        dependencies: Mapping of dependency name to spec

    Example:
        >>> config = GraftConfig(
        ...     api_version="graft/v0",
        ...     dependencies={"graft-knowledge": spec}
        ... )
        >>> config.has_dependency("graft-knowledge")
        True
    """

    api_version: str
    dependencies: dict[str, DependencySpec]

    def __post_init__(self) -> None:
        """Validate configuration."""
        if not self.api_version:
            raise ValidationError("API version cannot be empty")

        # Currently only support v0
        if not self.api_version.startswith("graft/"):
            raise ValidationError(
                f"Invalid API version: {self.api_version}. Must start with 'graft/'"
            )

    def get_dependency(self, name: str) -> DependencySpec:
        """Get dependency by name.

        Args:
            name: Dependency name

        Returns:
            DependencySpec for the dependency

        Raises:
            KeyError: If dependency not found
        """
        return self.dependencies[name]

    def has_dependency(self, name: str) -> bool:
        """Check if dependency exists.

        Args:
            name: Dependency name

        Returns:
            True if dependency exists
        """
        return name in self.dependencies
