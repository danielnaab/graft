"""Validate command - validate graft.yaml and graft.lock.

CLI command for validating configuration files and lock state.
"""

from pathlib import Path

import typer

from graft.adapters.lock_file import YamlLockFile
from graft.cli.dependency_context_factory import get_dependency_context
from graft.domain.exceptions import (
    ConfigFileNotFoundError,
    ConfigParseError,
    ConfigValidationError,
)
from graft.services import config_service, lock_service, validation_service


def validate_command(
    config_only: bool = typer.Option(
        False, "--config", help="Validate graft.yaml only"
    ),
    lock_only: bool = typer.Option(
        False, "--lock", help="Validate graft.lock schema only"
    ),
    integrity_only: bool = typer.Option(
        False, "--integrity", help="Validate .graft/ matches lock file"
    ),
) -> None:
    """Validate graft.yaml, graft.lock, and .graft/ integrity.

    Modes:
    - --config: Validate graft.yaml schema
    - --lock: Validate graft.lock schema
    - --integrity: Validate .graft/ commits match lock file
    - (no flags): Run all validations

    Exit codes:
    - 0: All validations passed
    - 1: Validation errors found
    - 2: Integrity mismatch (use with --integrity)

    Example:
        $ graft validate

        Validating graft.yaml...
          ✓ Schema is valid

        Validating graft.lock...
          ✓ Schema is valid

        Validating integrity...
          ✓ All commits match lock file

        Validation successful

        $ graft validate --integrity

        Validating integrity...
          ✓ my-dep: Commit matches
          ✗ other-dep: Commit mismatch: expected abc123, got def456

        Integrity check failed (exit code 2)
    """
    ctx = get_dependency_context()

    # Validate flag combinations
    flags_set = sum([config_only, lock_only, integrity_only])
    if flags_set > 1:
        typer.secho(
            "Error: --config, --lock, and --integrity are mutually exclusive",
            fg=typer.colors.RED,
            err=True,
        )
        raise typer.Exit(code=1)

    all_errors = []
    all_warnings = []
    integrity_failed = False

    # Determine what to validate based on flags
    validate_config = config_only or not (lock_only or integrity_only)
    validate_lock = lock_only or not (config_only or integrity_only)
    validate_integrity = integrity_only or not (config_only or lock_only)

    # Validate graft.yaml
    if validate_config:
        typer.echo("Validating graft.yaml...")

        try:
            # Find and parse configuration
            config_path = config_service.find_graft_yaml(ctx)
            config = config_service.parse_graft_yaml(ctx, config_path)

            # Validate schema
            schema_errors = validation_service.validate_config_schema(config)
            errors, warnings = validation_service.get_validation_summary(schema_errors)
            all_errors.extend(errors)
            all_warnings.extend(warnings)

            if not errors:
                typer.secho("  ✓ Schema is valid", fg=typer.colors.GREEN)
            else:
                for error in errors:
                    typer.secho(f"  ✗ {error}", fg=typer.colors.RED, err=True)

        except ConfigFileNotFoundError:
            typer.secho(
                "  ✗ graft.yaml not found",
                fg=typer.colors.RED,
                err=True,
            )
            all_errors.append("graft.yaml not found")

        except (ConfigParseError, ConfigValidationError) as e:
            typer.secho(
                f"  ✗ Failed to parse graft.yaml: {e}",
                fg=typer.colors.RED,
                err=True,
            )
            all_errors.append(str(e))

        typer.echo()

    # Validate graft.lock
    if validate_lock:
        typer.echo("Validating graft.lock...")

        lock_path = "graft.lock"
        if not Path(lock_path).exists():
            typer.secho(
                "  ⚠ graft.lock not found (run 'graft resolve' to create)",
                fg=typer.colors.YELLOW,
            )
            all_warnings.append("graft.lock not found")
        else:
            try:
                # Read lock file
                lock_file = YamlLockFile()
                lock_entries = lock_service.get_all_lock_entries(lock_file, lock_path)

                if not lock_entries:
                    typer.secho(
                        "  ⚠ graft.lock is empty",
                        fg=typer.colors.YELLOW,
                    )
                    all_warnings.append("graft.lock is empty")
                else:
                    typer.secho("  ✓ Schema is valid", fg=typer.colors.GREEN)

            except Exception as e:
                typer.secho(
                    f"  ✗ Failed to read graft.lock: {e}",
                    fg=typer.colors.RED,
                    err=True,
                )
                all_errors.append(f"Failed to read graft.lock: {e}")

        typer.echo()

    # Validate integrity (.graft/ matches lock file)
    if validate_integrity:
        typer.echo("Validating integrity...")

        lock_path = "graft.lock"
        if not Path(lock_path).exists():
            typer.secho(
                "  ✗ graft.lock not found",
                fg=typer.colors.RED,
                err=True,
            )
            all_errors.append("graft.lock not found (cannot validate integrity)")
        else:
            try:
                lock_file = YamlLockFile()
                lock_entries = lock_service.get_all_lock_entries(lock_file, lock_path)

                if not lock_entries:
                    typer.secho(
                        "  ⚠ graft.lock is empty",
                        fg=typer.colors.YELLOW,
                    )
                    all_warnings.append("graft.lock is empty")
                else:
                    # Run integrity validation
                    results = validation_service.validate_integrity(
                        filesystem=ctx.filesystem,
                        git=ctx.git,
                        deps_directory=ctx.deps_directory,
                        lock_entries=lock_entries,
                    )

                    # Display results
                    for result in results:
                        if result.valid:
                            typer.secho(
                                f"  ✓ {result.name}: {result.message}",
                                fg=typer.colors.GREEN,
                            )
                        else:
                            typer.secho(
                                f"  ✗ {result.name}: {result.message}",
                                fg=typer.colors.RED,
                            )
                            integrity_failed = True

                    if integrity_failed:
                        all_errors.append("Integrity check failed")

            except Exception as e:
                typer.secho(
                    f"  ✗ Failed to validate integrity: {e}",
                    fg=typer.colors.RED,
                    err=True,
                )
                all_errors.append(f"Failed to validate integrity: {e}")

        typer.echo()

    # Summary
    if all_errors:
        error_count = len(all_errors)
        warning_count = len(all_warnings)
        if warning_count > 0:
            typer.secho(
                f"Validation failed with {error_count} error(s) and {warning_count} warning(s)",
                fg=typer.colors.RED,
                err=True,
            )
        else:
            typer.secho(
                f"Validation failed with {error_count} error(s)",
                fg=typer.colors.RED,
                err=True,
            )
        # Exit code 2 for integrity failures, 1 for other errors
        if integrity_failed:
            raise typer.Exit(code=2)
        raise typer.Exit(code=1)

    elif all_warnings:
        warning_count = len(all_warnings)
        typer.secho(
            f"Validation passed with {warning_count} warning(s)",
            fg=typer.colors.YELLOW,
        )
        # Exit 0 for warnings (not failures)

    else:
        typer.secho("Validation successful", fg=typer.colors.GREEN, bold=True)
