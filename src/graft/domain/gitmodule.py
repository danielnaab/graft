"""Git submodule domain models.

Dataclasses for git submodule entries in .gitmodules file.
"""

from dataclasses import dataclass


@dataclass(frozen=True)
class GitModuleEntry:
    """Represents a git submodule entry.

    Immutable data class for submodule configuration.
    Maps to an entry in .gitmodules file.

    Attributes:
        name: Submodule name (typically same as directory name)
        path: Relative path where submodule is located
        url: Git repository URL for the submodule
        branch: Optional branch/tag to track (None for default)

    Example:
        >>> entry = GitModuleEntry(
        ...     name="my-lib",
        ...     path=".graft/my-lib",
        ...     url="https://github.com/user/my-lib.git",
        ...     branch="main"
        ... )
    """

    name: str
    path: str
    url: str
    branch: str | None = None

    def to_gitmodules_format(self) -> str:
        """Convert to .gitmodules file format.

        Returns:
            String representation for .gitmodules file

        Example:
            >>> entry.to_gitmodules_format()
            '[submodule "my-lib"]\\n\\tpath = .graft/my-lib\\n\\turl = https://...'
        """
        lines = [f'[submodule "{self.name}"]']
        lines.append(f"\tpath = {self.path}")
        lines.append(f"\turl = {self.url}")
        if self.branch:
            lines.append(f"\tbranch = {self.branch}")
        return "\n".join(lines)