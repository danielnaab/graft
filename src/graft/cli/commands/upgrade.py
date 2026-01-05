"""Upgrade command - upgrade dependency to new version.

CLI command for atomic dependency upgrades with migration and rollback.
"""

import subprocess
from pathlib import Path

import typer

from graft.adapters.command_executor import SubprocessCommandExecutor
from graft.adapters.lock_file import YamlLockFile
from graft.adapters.snapshot import FilesystemSnapshot
from graft.cli.dependency_context_factory import get_dependency_context
from graft.domain.exceptions import (
    ConfigFileNotFoundError,
    ConfigParseError,
    ConfigValidationError,
    DomainError,
)
from graft.services import config_service, upgrade_service
from graft.services.upgrade_service import UpgradeResult


def upgrade_command(
    dep_name: str,
    to: str | None = typer.Option(
        None, "--to", help="Target ref to upgrade to (e.g., v2.0.0)"
    ),
    skip_migration: bool = typer.Option(
        False, "--skip-migration", help="Skip migration command (not recommended)"
    ),
    skip_verify: bool = typer.Option(
        False, "--skip-verify", help="Skip verification command (not recommended)"
    ),
    dry_run: bool = typer.Option(
        False, "--dry-run", help="Preview upgrade without making changes"
    ),
) -> None:
    """Upgrade dependency to new version.

    Performs atomic upgrade with automatic rollback on failure:
    1. Creates snapshot for rollback
    2. Runs migration command (if defined)
    3. Runs verification command (if defined)
    4. Updates lock file
    5. On failure: rolls back all changes

    Args:
        dep_name: Name of dependency to upgrade
        to: Target ref (e.g., v2.0.0). Required.
        skip_migration: Skip migration command
        skip_verify: Skip verification command

    Example:
        $ graft upgrade meta-kb --to v2.0.0

        Upgrading meta-kb: v1.5.0 → v2.0.0

        Running migration: migrate-v2
          Command: npx jscodeshift -t codemods/v2.js src/
          ✓ Migration completed

        Running verification: verify-v2
          Command: npm test
          ✓ Verification passed

        ✓ Upgrade complete
        Updated graft.lock: meta-kb@v2.0.0
    """
    # Validate --to parameter
    if not to:
        typer.secho(
            "Error: --to parameter is required",
            fg=typer.colors.RED,
            err=True,
        )
        typer.echo("  Example: graft upgrade meta-kb --to v2.0.0", err=True)
        raise typer.Exit(code=1)

    ctx = get_dependency_context()

    try:
        # Step 1: Find and parse consumer's graft.yaml to get dependency source
        consumer_config_path = config_service.find_graft_yaml(ctx)
        consumer_config = config_service.parse_graft_yaml(ctx, consumer_config_path)

        # Check dependency exists in consumer's config
        if dep_name not in consumer_config.dependencies:
            typer.secho(
                f"Error: Dependency '{dep_name}' not found in graft.yaml",
                fg=typer.colors.RED,
                err=True,
            )
            typer.echo(
                f"  Available dependencies: {', '.join(consumer_config.dependencies.keys())}",
                err=True,
            )
            raise typer.Exit(code=1)

        dep_spec = consumer_config.dependencies[dep_name]
        source = dep_spec.git_url.url

        # Step 2: Find and parse dependency's graft.yaml
        dep_path = Path(ctx.deps_directory) / dep_name / "graft.yaml"
        dep_config_path = str(dep_path)
        dep_config = config_service.parse_graft_yaml(ctx, dep_config_path)

        # Step 3: Resolve ref to commit hash
        dep_repo_path = str(Path(ctx.deps_directory) / dep_name)

        # Try to fetch the ref to ensure we have it locally
        # (this may fail for local-only repos, which is OK)
        fetch_cmd = ["git", "-C", dep_repo_path, "fetch", "origin", to]
        fetch_result = subprocess.run(fetch_cmd, capture_output=True, text=True, check=False)

        # Now try to resolve the ref to a commit hash
        try:
            rev_parse_cmd = ["git", "-C", dep_repo_path, "rev-parse", to]
            rev_parse_result = subprocess.run(
                rev_parse_cmd, capture_output=True, text=True, check=True
            )
            commit = rev_parse_result.stdout.strip()
        except subprocess.CalledProcessError as e:
            # If resolution failed and fetch also failed, show helpful error
            if fetch_result.returncode != 0:
                typer.secho(
                    f"Error: Could not resolve ref '{to}'",
                    fg=typer.colors.RED,
                    err=True,
                )
                typer.echo(f"  Fetch failed: {fetch_result.stderr.strip()}", err=True)
                typer.echo(f"  Resolve failed: {e.stderr.strip()}", err=True)
                typer.secho(
                    "  Suggestion: Ensure the ref exists locally or can be fetched from origin",
                    fg=typer.colors.YELLOW,
                    err=True,
                )
            else:
                typer.secho(
                    f"Error: Failed to resolve ref '{to}' to commit hash",
                    fg=typer.colors.RED,
                    err=True,
                )
                typer.echo(f"  Git error: {e.stderr.strip()}", err=True)
            raise typer.Exit(code=1) from e

        # Step 4: Display upgrade info
        typer.secho(f"Upgrading {dep_name} → {to}", fg=typer.colors.BLUE, bold=True)
        typer.echo(f"  Source: {source}")
        typer.echo(f"  Commit: {commit[:7]}...")
        typer.echo()

        # Show warnings if skipping steps
        if skip_migration:
            typer.secho(
                "  Warning: Skipping migration command",
                fg=typer.colors.YELLOW,
            )
        if skip_verify:
            typer.secho(
                "  Warning: Skipping verification command",
                fg=typer.colors.YELLOW,
            )

        # Handle dry-run mode
        if dry_run:
            typer.secho("DRY RUN MODE - No changes will be made", fg=typer.colors.CYAN, bold=True)
            typer.echo()

            # Get change details to show what would happen
            if not dep_config.has_change(to):
                typer.secho(
                    f"Error: Change '{to}' not found in dependency configuration",
                    fg=typer.colors.RED,
                    err=True,
                )
                raise typer.Exit(code=1)

            change = dep_config.get_change(to)

            # Show what would be executed
            typer.secho("Planned operations:", fg=typer.colors.BLUE)
            typer.echo()

            # Step 1: Snapshot
            typer.echo("1. Create snapshot for rollback")
            typer.echo("   Snapshot: graft.lock")
            typer.echo()

            # Step 2: Migration
            if change.migration and not skip_migration:
                typer.echo("2. Run migration command")
                if change.migration in dep_config.commands:
                    cmd = dep_config.commands[change.migration]
                    typer.echo(f"   Name: {change.migration}")
                    typer.echo(f"   Command: {cmd.run}")
                    if cmd.description:
                        typer.echo(f"   Description: {cmd.description}")
                    if cmd.working_dir:
                        typer.echo(f"   Working directory: {cmd.working_dir}")
                else:
                    typer.secho(
                        f"   Warning: Migration command '{change.migration}' not found in config",
                        fg=typer.colors.YELLOW,
                    )
                typer.echo()
            elif change.migration and skip_migration:
                typer.secho("2. Migration command (SKIPPED)", fg=typer.colors.YELLOW)
                typer.echo(f"   Name: {change.migration}")
                typer.echo()
            else:
                typer.echo("2. No migration required")
                typer.echo()

            # Step 3: Verification
            if change.verify and not skip_verify:
                typer.echo("3. Run verification command")
                if change.verify in dep_config.commands:
                    cmd = dep_config.commands[change.verify]
                    typer.echo(f"   Name: {change.verify}")
                    typer.echo(f"   Command: {cmd.run}")
                    if cmd.description:
                        typer.echo(f"   Description: {cmd.description}")
                    if cmd.working_dir:
                        typer.echo(f"   Working directory: {cmd.working_dir}")
                else:
                    typer.secho(
                        f"   Warning: Verification command '{change.verify}' not found in config",
                        fg=typer.colors.YELLOW,
                    )
                typer.echo()
            elif change.verify and skip_verify:
                typer.secho("3. Verification command (SKIPPED)", fg=typer.colors.YELLOW)
                typer.echo(f"   Name: {change.verify}")
                typer.echo()
            else:
                typer.echo("3. No verification required")
                typer.echo()

            # Step 4: Lock file update
            typer.echo("4. Update graft.lock")
            typer.echo(f"   Dependency: {dep_name}")
            typer.echo(f"   New ref: {to}")
            typer.echo(f"   New commit: {commit[:7]}...")
            typer.echo()

            typer.secho("✓ Dry run complete - no changes made", fg=typer.colors.CYAN, bold=True)
            typer.echo()
            typer.echo("To perform the upgrade, run without --dry-run:")
            typer.echo(f"  graft upgrade {dep_name} --to {to}")
            return

        # Step 5: Call upgrade service
        snapshot = FilesystemSnapshot()
        executor = SubprocessCommandExecutor()
        lock_file = YamlLockFile()

        result: UpgradeResult = upgrade_service.upgrade_dependency(
            snapshot=snapshot,
            executor=executor,
            lock_file=lock_file,
            config=dep_config,
            dep_name=dep_name,
            to_ref=to,
            source=source,
            commit=commit,
            base_dir=".",
            lock_path="graft.lock",
            skip_migration=skip_migration,
            skip_verify=skip_verify,
            auto_cleanup=True,
        )

        # Step 6: Display results
        if result.success:
            typer.echo()

            # Show migration result
            if result.migration_result:
                typer.secho("Migration completed:", fg=typer.colors.GREEN)
                if result.migration_result.stdout:
                    typer.echo(f"  {result.migration_result.stdout.strip()}")

            # Show verification result
            if result.verify_result:
                typer.secho("Verification passed:", fg=typer.colors.GREEN)
                if result.verify_result.stdout:
                    typer.echo(f"  {result.verify_result.stdout.strip()}")

            typer.echo()
            typer.secho("✓ Upgrade complete", fg=typer.colors.GREEN, bold=True)
            typer.echo(f"Updated graft.lock: {dep_name}@{to}")

        else:
            typer.echo()
            typer.secho("✗ Upgrade failed", fg=typer.colors.RED, bold=True, err=True)
            typer.echo(f"  Error: {result.error}", err=True)
            typer.echo()
            typer.secho(
                "All changes have been rolled back",
                fg=typer.colors.YELLOW,
                err=True,
            )
            typer.echo("Lock file remains unchanged", err=True)

            # Show command output if available
            if result.migration_result and result.migration_result.stderr:
                typer.echo()
                typer.echo("Migration output:", err=True)
                typer.echo(f"  {result.migration_result.stderr.strip()}", err=True)

            if result.verify_result and result.verify_result.stderr:
                typer.echo()
                typer.echo("Verification output:", err=True)
                typer.echo(f"  {result.verify_result.stderr.strip()}", err=True)

            raise typer.Exit(code=1)

    except ConfigFileNotFoundError as e:
        typer.secho("Error: Configuration file not found", fg=typer.colors.RED, err=True)
        typer.echo(f"  Path: {e.path}", err=True)
        typer.secho(f"  Suggestion: {e.suggestion}", fg=typer.colors.YELLOW, err=True)
        raise typer.Exit(code=1) from e

    except ConfigParseError as e:
        typer.secho(
            "Error: Failed to parse configuration", fg=typer.colors.RED, err=True
        )
        typer.echo(f"  File: {e.path}", err=True)
        typer.echo(f"  Reason: {e.reason}", err=True)
        raise typer.Exit(code=1) from e

    except ConfigValidationError as e:
        typer.secho("Error: Invalid configuration", fg=typer.colors.RED, err=True)
        typer.echo(f"  File: {e.path}", err=True)
        typer.echo(f"  Field: {e.field}", err=True)
        typer.echo(f"  Reason: {e.reason}", err=True)
        raise typer.Exit(code=1) from e

    except DomainError as e:
        typer.secho(f"Error: {e}", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=1) from e

    except Exception as e:
        typer.secho(
            f"Error: Unexpected error during upgrade: {e}",
            fg=typer.colors.RED,
            err=True,
        )
        raise typer.Exit(code=1) from e
