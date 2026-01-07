"""Tree command - dependency tree visualization.

CLI command for visualizing dependency tree.
"""

import typer

from graft.adapters.lock_file import YamlLockFile
from graft.cli.dependency_context_factory import get_dependency_context
from graft.services import lock_service


def tree_command(show_all: bool = False) -> None:
    """Display dependency tree from lock file.

    Reads graft.lock and visualizes the dependency graph showing
    direct and transitive dependencies with their relationships.

    Args:
        show_all: Show detailed information for each dependency

    Example:
        $ graft tree

        Dependencies:
          meta-kb (v2.0.0) [direct]
            └── standards-kb (v1.5.0)
                └── templates-kb (v1.0.0)

        $ graft tree --show-all

        Dependencies:
          meta-kb (v2.0.0) [direct]
            source: git@github.com:org/meta-kb.git
            requires: standards-kb

          standards-kb (v1.5.0) [transitive via meta-kb]
            source: https://github.com:org/standards.git
            requires: templates-kb

          templates-kb (v1.0.0) [transitive via standards-kb]
            source: https://github.com/org/templates.git
            requires: (none)
    """
    ctx = get_dependency_context()

    # Find and read lock file
    try:
        lock_file = YamlLockFile()
        lock_file_path = lock_service.find_lock_file(lock_file, ".")
        if not lock_file_path:
            raise FileNotFoundError("graft.lock not found")
        entries = lock_file.read_lock_file(lock_file_path)

        if not entries:
            typer.echo("No dependencies found in lock file.")
            typer.echo()
            typer.secho(
                "Run 'graft resolve' to resolve dependencies.",
                fg=typer.colors.YELLOW,
            )
            return

    except FileNotFoundError:
        typer.secho("Error: Lock file not found", fg=typer.colors.RED, err=True)
        typer.secho(
            "  Suggestion: Run 'graft resolve' first", fg=typer.colors.YELLOW, err=True
        )
        raise typer.Exit(code=1)

    except Exception as e:
        typer.secho(
            f"Error: Failed to read lock file: {e}", fg=typer.colors.RED, err=True
        )
        raise typer.Exit(code=1) from e

    # Separate direct and transitive
    direct_deps = {name: entry for name, entry in entries.items() if entry.direct}
    transitive_deps = {name: entry for name, entry in entries.items() if not entry.direct}

    if show_all:
        # Detailed view
        typer.echo("Dependencies:")
        typer.echo()

        # Show direct dependencies first
        for name in sorted(direct_deps.keys()):
            entry = direct_deps[name]
            typer.secho(f"  {name} ({entry.ref}) [direct]", fg=typer.colors.GREEN, bold=True)
            typer.echo(f"    source: {entry.source}")
            if entry.requires:
                typer.echo(f"    requires: {', '.join(entry.requires)}")
            else:
                typer.echo("    requires: (none)")
            typer.echo()

        # Show transitive dependencies
        for name in sorted(transitive_deps.keys()):
            entry = transitive_deps[name]
            parents = ", ".join(entry.required_by)
            typer.secho(
                f"  {name} ({entry.ref}) [transitive via {parents}]",
                fg=typer.colors.BRIGHT_BLACK,
            )
            typer.echo(f"    source: {entry.source}")
            if entry.requires:
                typer.echo(f"    requires: {', '.join(entry.requires)}")
            else:
                typer.echo("    requires: (none)")
            typer.echo()

    else:
        # Tree view
        typer.echo("Dependencies:")
        typer.echo()

        # Build tree for each direct dependency
        for name in sorted(direct_deps.keys()):
            entry = direct_deps[name]
            typer.secho(f"  {name} ({entry.ref}) [direct]", fg=typer.colors.GREEN)

            # Show its transitive dependencies
            _print_tree_recursive(name, entries, level=1, visited=set())

        # Show orphaned transitive deps (shouldn't happen, but be defensive)
        shown_transitive = set()
        for name in direct_deps.keys():
            _collect_transitive(name, entries, shown_transitive)

        orphaned = set(transitive_deps.keys()) - shown_transitive
        if orphaned:
            typer.echo()
            typer.echo("  Orphaned transitive dependencies:")
            for name in sorted(orphaned):
                entry = transitive_deps[name]
                typer.secho(
                    f"    {name} ({entry.ref})",
                    fg=typer.colors.YELLOW,
                )

    # Summary
    typer.echo()
    total = len(entries)
    direct_count = len(direct_deps)
    transitive_count = len(transitive_deps)

    typer.echo(f"Total: {total} dependencies")
    typer.echo(f"  Direct: {direct_count}")
    if transitive_count > 0:
        typer.echo(f"  Transitive: {transitive_count}")


def _print_tree_recursive(
    dep_name: str,
    all_entries: dict,
    level: int,
    visited: set,
) -> None:
    """Recursively print dependency tree.

    Args:
        dep_name: Current dependency name
        all_entries: All lock entries
        level: Current indentation level
        visited: Set of already visited deps (for cycle detection)
    """
    if dep_name in visited:
        # Cycle detected
        return

    visited.add(dep_name)

    entry = all_entries.get(dep_name)
    if not entry or not entry.requires:
        return

    # Print each required dependency
    for i, req_name in enumerate(sorted(entry.requires)):
        is_last = i == len(entry.requires) - 1
        prefix = "    " * level
        connector = "└──" if is_last else "├──"

        req_entry = all_entries.get(req_name)
        if req_entry:
            typer.secho(
                f"{prefix}{connector} {req_name} ({req_entry.ref})",
                fg=typer.colors.BRIGHT_BLACK,
            )

            # Recurse for next level
            _print_tree_recursive(req_name, all_entries, level + 1, visited.copy())


def _collect_transitive(
    dep_name: str,
    all_entries: dict,
    collected: set,
) -> None:
    """Collect all transitive dependencies reachable from a dep.

    Args:
        dep_name: Dependency to start from
        all_entries: All lock entries
        collected: Set to collect transitive dep names into
    """
    entry = all_entries.get(dep_name)
    if not entry:
        return

    for req_name in entry.requires:
        if req_name not in collected:
            collected.add(req_name)
            _collect_transitive(req_name, all_entries, collected)
