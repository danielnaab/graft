"""Configuration parsing service.

Service functions for parsing and loading graft.yaml configuration files.
"""

from pathlib import Path

import yaml

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

    if "deps" not in data:
        raise ConfigValidationError(
            path=config_path,
            field="deps",
            reason="Missing required field",
        )

    if not isinstance(data["deps"], dict):
        raise ConfigValidationError(
            path=config_path,
            field="deps",
            reason="Must be a mapping/dict of dependency_name: url#ref",
        )

    # Parse dependencies
    dependencies: dict[str, DependencySpec] = {}

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

    return GraftConfig(
        api_version=data["apiVersion"],
        dependencies=dependencies,
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
