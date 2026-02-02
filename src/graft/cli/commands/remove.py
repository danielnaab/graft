"""Remove command - remove a dependency from graft.yaml.

CLI command for removing dependencies.
"""

import shutil
from pathlib import Path

import typer
import yaml


def remove_command(
    name: str = typer.Argument(..., help="Dependency name to remove"),
    keep_files: bool = typer.Option(
        False,
        "--keep-files",
        help="Keep the dependency files in .graft/",
    ),
) -> None:
    """Remove a dependency from graft.yaml.

    By default, also deletes the dependency files from .graft/.
    Use --keep-files to preserve the local copy.

    Example:
        $ graft remove my-kb

        Removed my-kb from graft.yaml
        Deleted .graft/my-kb

        $ graft remove other-kb --keep-files

        Removed other-kb from graft.yaml
        Kept files in .graft/other-kb
    """
    config_path = Path("graft.yaml")
    deps_path = Path(".graft") / name

    # Check config file exists
    if not config_path.exists():
        typer.secho(
            "Error: graft.yaml not found",
            fg=typer.colors.RED,
            err=True,
        )
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

    # Check if dependency exists
    if "deps" not in config or name not in config.get("deps", {}):
        typer.secho(
            f"Error: Dependency '{name}' not found in graft.yaml",
            fg=typer.colors.RED,
            err=True,
        )
        raise typer.Exit(code=1)

    # Remove from config
    del config["deps"][name]

    # Clean up empty deps section
    if not config["deps"]:
        del config["deps"]

    # Write back
    try:
        with open(config_path, "w") as f:
            yaml.dump(
                config,
                f,
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

    typer.secho(f"Removed {name} from graft.yaml", fg=typer.colors.GREEN)

    # Handle files
    if deps_path.exists():
        if keep_files:
            typer.echo(f"Kept files in {deps_path}")
        else:
            try:
                shutil.rmtree(deps_path)
                typer.echo(f"Deleted {deps_path}")
            except Exception as e:
                typer.secho(
                    f"Warning: Failed to delete {deps_path}: {e}",
                    fg=typer.colors.YELLOW,
                )
    else:
        if not keep_files:
            typer.echo(f"No files found at {deps_path}")
