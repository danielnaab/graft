"""Command domain model.

Represents an executable command defined in a dependency's graft.yaml.
"""

from dataclasses import dataclass, field

from graft.domain.exceptions import ValidationError


@dataclass(frozen=True)
class Command:
    """Represents an executable command from a dependency.

    Commands can be migration scripts, verification scripts, or utility commands
    that consumers can execute.

    Attributes:
        name: Command name (unique identifier)
        run: Shell command to execute (required)
        description: Optional human-readable description
        working_dir: Optional working directory (relative to consumer root)
        env: Optional environment variables to set during execution

    Example:
        >>> command = Command(
        ...     name="migrate-v2",
        ...     run="npx jscodeshift -t codemods/v2.js src/",
        ...     description="Rename getUserData → fetchUserData",
        ...     env={"NODE_ENV": "production"}
        ... )
        >>> command.name
        'migrate-v2'
        >>> command.has_env_vars()
        True
    """

    name: str
    run: str
    description: str | None = None
    working_dir: str | None = None
    env: dict[str, str] = field(default_factory=dict)

    def __post_init__(self) -> None:
        """Validate command."""
        # Validate name
        if not self.name:
            raise ValidationError("Command name cannot be empty")
        if not self.name.strip():
            raise ValidationError("Command name cannot be only whitespace")
        if len(self.name) > 100:
            raise ValidationError(
                f"Command name too long: {len(self.name)} chars (max 100)"
            )
        if ":" in self.name:
            raise ValidationError(
                f"Command name '{self.name}' cannot contain ':' (reserved separator). "
                f"Use '{self.name.replace(':', '-')}' instead."
            )

        # Validate run command (required)
        if not self.run:
            raise ValidationError(f"Command '{self.name}': 'run' field is required")
        if not self.run.strip():
            raise ValidationError(
                f"Command '{self.name}': 'run' field cannot be only whitespace"
            )

        # Validate description length if provided
        if self.description and len(self.description) > 500:
            raise ValidationError(
                f"Command '{self.name}': description too long: "
                f"{len(self.description)} chars (max 500)"
            )

        # Validate working_dir if provided
        if self.working_dir is not None:
            if not self.working_dir.strip():
                raise ValidationError(
                    f"Command '{self.name}': working_dir cannot be only whitespace"
                )
            # Check for absolute paths (should be relative)
            if self.working_dir.startswith("/") or (
                len(self.working_dir) > 1 and self.working_dir[1] == ":"
            ):
                raise ValidationError(
                    f"Command '{self.name}': working_dir must be relative, not absolute"
                )

    def has_env_vars(self) -> bool:
        """Check if this command has environment variables defined.

        Returns:
            True if env dict is not empty
        """
        return len(self.env) > 0

    def get_full_command(self, args: list[str] | None = None) -> str:
        """Get the full command string with optional arguments appended.

        SECURITY NOTE: Arguments are appended without shell escaping.
        This is safe when args come from trusted sources (CLI user input),
        but would be unsafe if args came from untrusted sources.
        The command is executed with shell=True, so shell metacharacters
        in args will be interpreted.

        Args:
            args: Optional additional arguments from trusted source (CLI)

        Returns:
            Complete command string

        Example:
            >>> cmd = Command(name="test", run="npm test")
            >>> cmd.get_full_command(["--verbose"])
            'npm test --verbose'
        """
        if args:
            return f"{self.run} {' '.join(args)}"
        return self.run
