"""Configuration domain models.

Value objects for graft configuration.
"""

from dataclasses import dataclass, field
from typing import Any

from graft.domain.change import Change
from graft.domain.command import Command
from graft.domain.dependency import DependencySpec
from graft.domain.exceptions import ValidationError


@dataclass(frozen=True)
class GraftConfig:
    """Graft configuration from graft.yaml.

    Immutable value object representing entire graft.yaml configuration.

    Attributes:
        api_version: Configuration API version (e.g., "graft/v0")
        dependencies: Mapping of dependency name to spec
        metadata: Optional metadata section (name, description, etc.)
        changes: Mapping of git ref to Change objects
        commands: Mapping of command name to Command objects

    Example:
        >>> config = GraftConfig(
        ...     api_version="graft/v0",
        ...     dependencies={"graft-knowledge": spec},
        ...     changes={"v1.0.0": change},
        ...     commands={"migrate": cmd}
        ... )
        >>> config.has_dependency("graft-knowledge")
        True
        >>> config.has_change("v1.0.0")
        True
    """

    api_version: str
    dependencies: dict[str, DependencySpec] = field(default_factory=dict)
    metadata: dict[str, Any] = field(default_factory=dict)
    changes: dict[str, Change] = field(default_factory=dict)
    commands: dict[str, Command] = field(default_factory=dict)

    def __post_init__(self) -> None:
        """Validate configuration."""
        if not self.api_version:
            raise ValidationError("API version cannot be empty")

        # Currently only support v0
        if not self.api_version.startswith("graft/"):
            raise ValidationError(
                f"Invalid API version: {self.api_version}. Must start with 'graft/'"
            )

        # Validate that migration/verify commands exist
        for ref, change in self.changes.items():
            if change.migration and change.migration not in self.commands:
                raise ValidationError(
                    f"Change '{ref}': migration command '{change.migration}' "
                    f"not found in commands section"
                )
            if change.verify and change.verify not in self.commands:
                raise ValidationError(
                    f"Change '{ref}': verify command '{change.verify}' "
                    f"not found in commands section"
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

    def get_change(self, ref: str) -> Change:
        """Get change by ref.

        Args:
            ref: Git ref

        Returns:
            Change for the ref

        Raises:
            KeyError: If change not found
        """
        return self.changes[ref]

    def has_change(self, ref: str) -> bool:
        """Check if change exists.

        Args:
            ref: Git ref

        Returns:
            True if change exists
        """
        return ref in self.changes

    def get_command(self, name: str) -> Command:
        """Get command by name.

        Args:
            name: Command name

        Returns:
            Command for the name

        Raises:
            KeyError: If command not found
        """
        return self.commands[name]

    def has_command(self, name: str) -> bool:
        """Check if command exists.

        Args:
            name: Command name

        Returns:
            True if command exists
        """
        return name in self.commands

    def get_breaking_changes(self) -> list[Change]:
        """Get all breaking changes.

        Returns:
            List of changes with type="breaking"
        """
        return [c for c in self.changes.values() if c.is_breaking()]

    def get_changes_needing_migration(self) -> list[Change]:
        """Get all changes that need migration.

        Returns:
            List of changes with migration command defined
        """
        return [c for c in self.changes.values() if c.needs_migration()]
