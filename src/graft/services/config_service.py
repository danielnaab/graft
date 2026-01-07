"""Configuration parsing service.

Service functions for parsing and loading graft.yaml configuration files.
"""

from pathlib import Path
from typing import Any

import yaml

from graft.domain.change import Change
from graft.domain.command import Command
from graft.domain.config import GraftConfig
from graft.domain.dependency import DependencySpec, GitRef, GitUrl
from graft.domain.exceptions import (
    ConfigFileNotFoundError,
    ConfigParseError,
    ConfigValidationError,
)
from graft.services.dependency_context import DependencyContext


def parse_graft_yaml(
    ctx: DependencyContext,
    config_path: str,
) -> GraftConfig:
    """Parse graft.yaml configuration file.

    Args:
        ctx: Dependency context
        config_path: Path to graft.yaml

    Returns:
        Parsed GraftConfig

    Raises:
        ConfigFileNotFoundError: If file doesn't exist
        ConfigParseError: If YAML is malformed
        ConfigValidationError: If configuration structure is invalid
        ValidationError: If domain rules violated (URL, ref, name validation)

    Example:
        >>> from graft.adapters.filesystem import RealFileSystem
        >>> ctx = DependencyContext(filesystem=RealFileSystem(), deps_directory="..")
        >>> config = parse_graft_yaml(ctx, "graft.yaml")
        >>> config.api_version
        'graft/v0'
    """
    # Check file exists
    if not ctx.filesystem.exists(config_path):
        raise ConfigFileNotFoundError(
            path=config_path,
            suggestion="Create graft.yaml with 'apiVersion: graft/v0' and 'deps:'",
        )

    # Read file
    try:
        content = ctx.filesystem.read_text(config_path)
    except PermissionError as e:
        raise ConfigParseError(
            path=config_path,
            reason=f"Permission denied: {e}",
        ) from e

    # Parse YAML
    try:
        data = yaml.safe_load(content)
    except yaml.YAMLError as e:
        raise ConfigParseError(
            path=config_path,
            reason=f"Invalid YAML syntax: {e}",
        ) from e

    # Validate structure
    if not isinstance(data, dict):
        raise ConfigValidationError(
            path=config_path,
            field="root",
            reason="Configuration must be a YAML mapping/dict",
        )

    if "apiVersion" not in data:
        raise ConfigValidationError(
            path=config_path,
            field="apiVersion",
            reason="Missing required field",
        )

    # Parse metadata (optional)
    metadata: dict[str, Any] = {}
    if "metadata" in data:
        if not isinstance(data["metadata"], dict):
            raise ConfigValidationError(
                path=config_path,
                field="metadata",
                reason="Must be a mapping/dict",
            )
        metadata = data["metadata"]

    # Parse commands (optional)
    commands: dict[str, Command] = {}
    if "commands" in data:
        if not isinstance(data["commands"], dict):
            raise ConfigValidationError(
                path=config_path,
                field="commands",
                reason="Must be a mapping/dict of command_name: {...}",
            )

        for cmd_name, cmd_data in data["commands"].items():
            if not isinstance(cmd_data, dict):
                raise ConfigValidationError(
                    path=config_path,
                    field=f"commands.{cmd_name}",
                    reason="Command must be a mapping/dict with 'run' field",
                )

            if "run" not in cmd_data:
                raise ConfigValidationError(
                    path=config_path,
                    field=f"commands.{cmd_name}",
                    reason="Command must have 'run' field",
                )

            command = Command(
                name=cmd_name,
                run=cmd_data["run"],
                description=cmd_data.get("description"),
                working_dir=cmd_data.get("working_dir"),
                env=cmd_data.get("env", {}),
            )
            commands[cmd_name] = command

    # Parse changes (optional)
    changes: dict[str, Change] = {}
    if "changes" in data:
        if not isinstance(data["changes"], dict):
            raise ConfigValidationError(
                path=config_path,
                field="changes",
                reason="Must be a mapping/dict of ref: {...}",
            )

        for ref, change_data in data["changes"].items():
            if not isinstance(change_data, dict):
                # Allow simple format: changes: { v1.0.0: null }
                change_data = {}

            # Extract known fields and rest goes to metadata
            change_metadata = {
                k: v
                for k, v in change_data.items()
                if k not in ("type", "description", "migration", "verify")
            }

            change = Change(
                ref=ref,
                type=change_data.get("type"),
                description=change_data.get("description"),
                migration=change_data.get("migration"),
                verify=change_data.get("verify"),
                metadata=change_metadata,
            )
            changes[ref] = change

    # Parse dependencies (optional now, required for backward compatibility with old format)
    dependencies: dict[str, DependencySpec] = {}
    if "deps" in data:
        if not isinstance(data["deps"], dict):
            raise ConfigValidationError(
                path=config_path,
                field="deps",
                reason="Must be a mapping/dict of dependency_name: url#ref",
            )

        for name, url_with_ref in data["deps"].items():
            # Parse URL#ref format
            if "#" not in url_with_ref:
                raise ConfigValidationError(
                    path=config_path,
                    field=f"deps.{name}",
                    reason=f"Must use format 'url#ref', got: {url_with_ref}",
                )

            url_part, ref_part = url_with_ref.rsplit("#", 1)

            # Create dependency spec
            spec = DependencySpec(
                name=name,
                git_url=GitUrl(url_part),
                git_ref=GitRef(ref_part),
            )

            dependencies[name] = spec

    # Also support new 'dependencies' format from spec
    if "dependencies" in data:
        if not isinstance(data["dependencies"], dict):
            raise ConfigValidationError(
                path=config_path,
                field="dependencies",
                reason="Must be a mapping/dict",
            )

        for name, dep_data in data["dependencies"].items():
            if isinstance(dep_data, str):
                # Simple format: "url#ref"
                if "#" not in dep_data:
                    raise ConfigValidationError(
                        path=config_path,
                        field=f"dependencies.{name}",
                        reason=f"Must use format 'url#ref', got: {dep_data}",
                    )
                url_part, ref_part = dep_data.rsplit("#", 1)
            elif isinstance(dep_data, dict):
                # Object format with source and ref
                if "source" not in dep_data:
                    raise ConfigValidationError(
                        path=config_path,
                        field=f"dependencies.{name}",
                        reason="Dependency must have 'source' field",
                    )
                url_part = dep_data["source"]
                ref_part = dep_data.get("ref", "main")  # Default to main
            else:
                raise ConfigValidationError(
                    path=config_path,
                    field=f"dependencies.{name}",
                    reason="Dependency must be string or object",
                )

            spec = DependencySpec(
                name=name,
                git_url=GitUrl(url_part),
                git_ref=GitRef(ref_part),
            )
            dependencies[name] = spec

    return GraftConfig(
        api_version=data["apiVersion"],
        dependencies=dependencies,
        metadata=metadata,
        changes=changes,
        commands=commands,
    )


def find_graft_yaml(ctx: DependencyContext) -> str:
    """Find graft.yaml in current directory.

    Args:
        ctx: Dependency context

    Returns:
        Path to graft.yaml

    Raises:
        ConfigFileNotFoundError: If graft.yaml not found

    Example:
        >>> ctx = DependencyContext(filesystem=RealFileSystem(), deps_directory="..")
        >>> config_path = find_graft_yaml(ctx)
        >>> config_path.endswith("graft.yaml")
        True
    """
    cwd = ctx.filesystem.get_cwd()
    config_path = str(Path(cwd) / "graft.yaml")

    if not ctx.filesystem.exists(config_path):
        raise ConfigFileNotFoundError(
            path=config_path,
            suggestion=f"Create graft.yaml in {cwd}",
        )

    return config_path
