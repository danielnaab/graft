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
    mode: str = typer.Argument(
        "all",
        help="Validation mode: config, lock, integrity, or all",
    ),
    # Legacy flags (deprecated)
    schema_only: bool = typer.Option(
        False, "--schema", help="[DEPRECATED] Use 'graft validate config' instead"
    ),
    refs_only: bool = typer.Option(
        False, "--refs", help="[DEPRECATED] Use 'graft validate integrity' instead"
    ),
    lock_only: bool = typer.Option(
        False, "--lock", help="[DEPRECATED] Use 'graft validate lock' instead"
    ),
) -> None:
    """Validate graft.yaml and graft.lock for correctness.

    Modes:
    - config: Validate graft.yaml structure and schema only
    - lock: Validate graft.lock file consistency only
    - integrity: Verify .graft/ matches lock file (checks commits)
    - all: Run all validations (default)

    Examples:
        $ graft validate              # Validate everything (default: all)
        $ graft validate config       # Validate only graft.yaml
        $ graft validate lock         # Validate only graft.lock
        $ graft validate integrity    # Verify dependencies match lock file

    Exit codes:
    - 0: Success
    - 1: Validation error
    - 2: Integrity mismatch

    Note: Command reference validation (migration/verify) happens
    automatically during graft.yaml parsing.
    """
    ctx = get_dependency_context()

    # Check for deprecated flag usage
    flags_set = sum([schema_only, refs_only, lock_only])
    if flags_set > 0:
        typer.secho(
            "Warning: --schema, --refs, and --lock flags are deprecated.",
            fg=typer.colors.YELLOW,
        )
        typer.secho(
            "         Use modes instead: 'graft validate [config|lock|integrity|all]'",
            fg=typer.colors.YELLOW,
        )
        typer.echo()

        # Validate flag combinations
        if flags_set > 1:
            typer.secho(
                "Error: --schema, --refs, and --lock are mutually exclusive",
                fg=typer.colors.RED,
                err=True,
            )
            raise typer.Exit(code=1)

    # Validate mode argument
    valid_modes = ["config", "lock", "integrity", "all"]
    if mode not in valid_modes:
        typer.secho(
            f"Error: Invalid mode '{mode}'. Must be one of: {', '.join(valid_modes)}",
            fg=typer.colors.RED,
            err=True,
        )
        raise typer.Exit(code=1)

    all_errors = []
    all_warnings = []
    integrity_mismatch = False

    # Determine what to validate based on mode (or legacy flags)
    if flags_set > 0:
        # Legacy flag-based validation (maintain exact original behavior)
        validate_schema = schema_only or not (refs_only or lock_only)
        validate_refs = refs_only or not (schema_only or lock_only)
        validate_lock = lock_only or not (schema_only or refs_only)
        validate_integrity = False  # Legacy flags don't have integrity mode
    else:
        # Mode-based validation
        validate_schema = mode in ["config", "all"]
        validate_refs = mode in ["config", "all"]
        validate_lock = mode in ["lock", "all"]
        validate_integrity = mode in ["integrity", "all"]

    # Validate graft.yaml
    if validate_schema or validate_refs:
        typer.echo("Validating graft.yaml...")

        try:
            # Find and parse configuration
            config_path = config_service.find_graft_yaml(ctx)
            config = config_service.parse_graft_yaml(ctx, config_path)

            # Validate schema
            if validate_schema:
                schema_errors = validation_service.validate_config_schema(config)
                errors, warnings = validation_service.get_validation_summary(schema_errors)
                all_errors.extend(errors)
                all_warnings.extend(warnings)

                if not errors:
                    typer.secho("  ✓ Schema is valid", fg=typer.colors.GREEN)
                else:
                    for error in errors:
                        typer.secho(f"  ✗ {error}", fg=typer.colors.RED, err=True)

            # Validate refs exist in git
            if validate_refs:
                # Check that dependencies are cloned first
                deps_not_cloned = []
                for dep in config.dependencies.values():
                    dep_path = Path(ctx.deps_directory) / dep.name
                    if not dep_path.exists():
                        deps_not_cloned.append(dep.name)

                if deps_not_cloned:
                    typer.secho(
                        f"  ⚠ Dependencies not cloned (run 'graft resolve'): {', '.join(deps_not_cloned)}",
                        fg=typer.colors.YELLOW,
                    )
                    all_warnings.append(f"Dependencies not cloned: {', '.join(deps_not_cloned)}")
                else:
                    # Validate refs for each dependency
                    ref_errors_found = False
                    for dep in config.dependencies.values():
                        dep_path = Path(ctx.deps_directory) / dep.name
                        dep_config_path = dep_path / "graft.yaml"

                        if not dep_config_path.exists():
                            typer.secho(
                                f"  ⚠ {dep.name}: graft.yaml not found",
                                fg=typer.colors.YELLOW,
                            )
                            all_warnings.append(f"{dep.name}: graft.yaml not found")
                            continue

                        try:
                            dep_config = config_service.parse_graft_yaml(
                                ctx, str(dep_config_path)
                            )
                            ref_errors = validation_service.validate_refs_exist(
                                dep_config, ctx.git, str(dep_path)
                            )
                            errors, warnings = validation_service.get_validation_summary(
                                ref_errors
                            )

                            # Print ref errors immediately
                            for error in errors:
                                typer.secho(f"  ✗ {dep.name}: {error}", fg=typer.colors.RED, err=True)
                                ref_errors_found = True

                            all_errors.extend([f"{dep.name}: {e}" for e in errors])
                            all_warnings.extend([f"{dep.name}: {w}" for w in warnings])
                        except Exception as e:
                            error_msg = f"{dep.name}: Failed to validate refs: {e}"
                            typer.secho(f"  ✗ {error_msg}", fg=typer.colors.RED, err=True)
                            all_errors.append(error_msg)
                            ref_errors_found = True

                    if not ref_errors_found and not deps_not_cloned:
                        typer.secho(
                            "  ✓ All refs exist in git repositories",
                            fg=typer.colors.GREEN,
                        )

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
    if validate_lock or validate_integrity:
        typer.echo("Validating graft.lock...")

        lock_path = "graft.lock"
        if not Path(lock_path).exists():
            typer.secho(
                "  ⚠ graft.lock not found (run 'graft apply' to create)",
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

                    # Validate each lock entry
                    deps_not_cloned = []
                    lock_errors_found = False
                    for dep_name, entry in lock_entries.items():
                        dep_path = Path(ctx.deps_directory) / dep_name

                        if not dep_path.exists():
                            deps_not_cloned.append(dep_name)
                            continue

                        if not ctx.git.is_repository(str(dep_path)):
                            typer.secho(
                                f"  ⚠ {dep_name}: not a git repository",
                                fg=typer.colors.YELLOW,
                            )
                            all_warnings.append(f"{dep_name}: not a git repository")
                            continue

                        lock_errors = validation_service.validate_lock_entry(
                            entry, ctx.git, str(dep_path)
                        )
                        errors, warnings = validation_service.get_validation_summary(
                            lock_errors
                        )

                        # Print errors/warnings with dep name prefix
                        for error in errors:
                            typer.secho(f"  ✗ {dep_name}: {error}", fg=typer.colors.RED, err=True)
                            lock_errors_found = True

                        # In integrity mode, commit mismatches are integrity errors (not just warnings)
                        for warning in warnings:
                            if "has moved" in warning and validate_integrity:
                                typer.secho(f"  ✗ {dep_name}: {warning}", fg=typer.colors.RED, err=True)
                                integrity_mismatch = True
                            else:
                                typer.secho(f"  ⚠ {dep_name}: {warning}", fg=typer.colors.YELLOW)

                        all_errors.extend([f"{dep_name}: {e}" for e in errors])

                        # Track integrity mismatches separately in integrity mode
                        if validate_integrity:
                            integrity_warnings = [w for w in warnings if "has moved" in w]
                            if integrity_warnings:
                                all_errors.extend([f"{dep_name}: {w}" for w in integrity_warnings])
                            normal_warnings = [w for w in warnings if "has moved" not in w]
                            all_warnings.extend([f"{dep_name}: {w}" for w in normal_warnings])
                        else:
                            all_warnings.extend([f"{dep_name}: {w}" for w in warnings])

                    # Warn about dependencies not cloned
                    if deps_not_cloned:
                        typer.secho(
                            f"  ⚠ Dependencies not cloned (run 'graft resolve'): {', '.join(deps_not_cloned)}",
                            fg=typer.colors.YELLOW,
                        )
                        all_warnings.append(f"Dependencies not cloned: {', '.join(deps_not_cloned)}")

                    # Show summary if no errors/warnings
                    if not lock_errors_found and not deps_not_cloned:
                        lock_warnings_only = [w for w in all_warnings if "has moved" in w]
                        if not lock_warnings_only:
                            typer.secho("  ✓ All commits match", fg=typer.colors.GREEN)

            except Exception as e:
                typer.secho(
                    f"  ✗ Failed to read graft.lock: {e}",
                    fg=typer.colors.RED,
                    err=True,
                )
                all_errors.append(f"Failed to read graft.lock: {e}")

        typer.echo()

    # Summary and exit codes
    if all_errors:
        error_count = len(all_errors)
        warning_count = len(all_warnings)

        # Determine exit code: 2 for integrity mismatch, 1 for validation error
        exit_code = 2 if integrity_mismatch else 1

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

        # Special message for integrity mismatches
        if integrity_mismatch:
            typer.secho(
                "Integrity check failed: Dependencies do not match lock file",
                fg=typer.colors.RED,
                err=True,
            )
            typer.secho(
                "Run 'graft resolve' to update dependencies to match lock file",
                fg=typer.colors.YELLOW,
            )

        raise typer.Exit(code=exit_code)

    elif all_warnings:
        warning_count = len(all_warnings)
        typer.secho(
            f"Validation passed with {warning_count} warning(s)",
            fg=typer.colors.YELLOW,
        )
        # Exit 0 for warnings (not failures)

    else:
        typer.secho("✓ Validation successful", fg=typer.colors.GREEN, bold=True)
