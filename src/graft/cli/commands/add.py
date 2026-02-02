"""Add command - add a dependency to graft.yaml.

CLI command for adding dependencies.
"""

import re
from pathlib import Path

import typer
import yaml


def add_command(
    name: str = typer.Argument(..., help="Dependency name"),
    url_ref: str = typer.Argument(..., help="Git URL with ref (url#ref)"),
) -> None:
    """Add a dependency to graft.yaml.

    Does NOT resolve the dependency - run 'graft resolve' after adding.

    Example:
        $ graft add my-kb https://github.com/user/repo.git#main

        Added my-kb to graft.yaml
        Run 'graft resolve' to clone the dependency.

        $ graft add other-kb git@github.com:org/repo.git#v1.0.0
    """
    config_path = Path("graft.yaml")

    # Parse URL#ref format
    if "#" not in url_ref:
        typer.secho(
            "Error: URL must include ref (e.g., url#main or url#v1.0.0)",
            fg=typer.colors.RED,
            err=True,
        )
        raise typer.Exit(code=1)

    url, ref = url_ref.rsplit("#", 1)

    if not url:
        typer.secho("Error: URL cannot be empty", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=1)

    if not ref:
        typer.secho("Error: Ref cannot be empty", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=1)

    # Validate name
    if not re.match(r"^[a-zA-Z0-9_-]+$", name):
        typer.secho(
            "Error: Name must contain only alphanumeric characters, hyphens, and underscores",
            fg=typer.colors.RED,
            err=True,
        )
        raise typer.Exit(code=1)

    # Check config file exists
    if not config_path.exists():
        typer.secho(
            "Error: graft.yaml not found",
            fg=typer.colors.RED,
            err=True,
        )
        typer.echo("Create graft.yaml first or run 'graft init'.", err=True)
        raise typer.Exit(code=1)

    # Read existing config
    try:
        content = config_path.read_text()
        config = yaml.safe_load(content) or {}
    except yaml.YAMLError as e:
        typer.secho(
            f"Error: Failed to parse graft.yaml: {e}",
            fg=typer.colors.RED,
            err=True,
        )
        raise typer.Exit(code=1) from e

    # Ensure deps section exists
    if "deps" not in config:
        config["deps"] = {}

    # Check if dependency already exists
    if name in config["deps"]:
        typer.secho(
            f"Error: Dependency '{name}' already exists in graft.yaml",
            fg=typer.colors.RED,
            err=True,
        )
        typer.echo(f"Current value: {config['deps'][name]}", err=True)
        raise typer.Exit(code=1)

    # Add the dependency (quoted for consistency with hand-written format)
    dep_value = f'"{url}#{ref}"'
    config["deps"][name] = dep_value

    # Write back - use custom representer to avoid double-quoting
    class QuotedDumper(yaml.SafeDumper):
        pass

    def str_representer(dumper, data):
        # If string starts and ends with quotes, it's pre-quoted - use literal style
        if data.startswith('"') and data.endswith('"'):
            return dumper.represent_scalar('tag:yaml.org,2002:str', data[1:-1], style='"')
        return dumper.represent_scalar('tag:yaml.org,2002:str', data)

    QuotedDumper.add_representer(str, str_representer)

    try:
        with open(config_path, "w") as f:
            yaml.dump(
                config,
                f,
                Dumper=QuotedDumper,
                default_flow_style=False,
                sort_keys=False,
                allow_unicode=True,
            )
    except Exception as e:
        typer.secho(
            f"Error: Failed to write graft.yaml: {e}",
            fg=typer.colors.RED,
            err=True,
        )
        raise typer.Exit(code=1) from e

    typer.secho(f"Added {name} to graft.yaml", fg=typer.colors.GREEN)
    typer.echo("Run 'graft resolve' to clone the dependency.")
