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
    schema_only: bool = typer.Option(
        False, "--schema", help="Validate YAML schema only"
    ),
    refs_only: bool = typer.Option(
        False, "--refs", help="Validate git refs exist"
    ),
    lock_only: bool = typer.Option(
        False, "--lock", help="Validate lock file consistency"
    ),
) -> None:
    """Validate graft.yaml and graft.lock for correctness.

    Checks:
    - graft.yaml structure and schema
    - Git refs exist in repositories
    - Lock file consistency
    - Commit hashes match refs

    Example:
        $ graft validate

        Validating graft.yaml...
          ✓ Schema is valid
          ✓ All refs exist in git repositories

        Validating graft.lock...
          ✓ Schema is valid
          ✓ All commits match

        Validation successful

    Note: Command reference validation (migration/verify) happens
    automatically during graft.yaml parsing.
    """
    ctx = get_dependency_context()

    # Validate flag combinations
    flags_set = sum([schema_only, refs_only, lock_only])
    if flags_set > 1:
        typer.secho(
            "Error: --schema, --refs, and --lock are mutually exclusive",
            fg=typer.colors.RED,
            err=True,
        )
        raise typer.Exit(code=1)

    all_errors = []
    all_warnings = []

    # Determine what to validate based on flags
    validate_schema = schema_only or not (refs_only or lock_only)
    validate_refs = refs_only or not (schema_only or lock_only)
    validate_lock = lock_only or not (schema_only or refs_only)

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
    if validate_lock:
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
                        for warning in warnings:
                            typer.secho(f"  ⚠ {dep_name}: {warning}", fg=typer.colors.YELLOW)

                        all_errors.extend([f"{dep_name}: {e}" for e in errors])
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
        raise typer.Exit(code=1)

    elif all_warnings:
        warning_count = len(all_warnings)
        typer.secho(
            f"Validation passed with {warning_count} warning(s)",
            fg=typer.colors.YELLOW,
        )
        # Exit 0 for warnings (not failures)

    else:
        typer.secho("✓ Validation successful", fg=typer.colors.GREEN, bold=True)
