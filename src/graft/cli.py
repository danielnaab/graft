from __future__ import annotations
import json, pathlib, yaml
import typer
from typing import Optional

from .utils import print_json
from .adapters.filesystem import LocalFileSystem
from .adapters.config import ConfigAdapter
from .adapters.docker import DockerAdapter, BuildError, TransformerExecutionError
from .adapters.materials import LocalMaterialLoader, MaterialNotFoundError
from .adapters.orchestrator import DVCAdapter
from .services.explain import ExplainService
from .services.run import RunService, TemplateNotFoundError, TemplateRenderError, OutputMissingError
from .services.status import StatusService
from .services.finalize import FinalizeService, AgentInfo
from .services.orchestrator import OrchestratorService, DriftEnforcedError, InvalidDVCYamlError
from .domain.orchestrator import SyncPolicy

app = typer.Typer(add_completion=False, help="Graft command-line interface")

# Initialize adapters and services (dependency injection)
fs = LocalFileSystem()
config_adapter = ConfigAdapter(fs)
material_loader = LocalMaterialLoader(fs)
container_adapter = DockerAdapter()
orchestrator_adapter = DVCAdapter(fs, config_adapter)
explain_service = ExplainService(config_adapter)
run_service = RunService(config_adapter, fs, material_loader, container_adapter)
status_service = StatusService()
finalize_service = FinalizeService(fs)


def _artifact_path(p: str) -> pathlib.Path:
    """Validate and return artifact path."""
    path = pathlib.Path(p)
    graft_yaml = path / "graft.yaml"
    if graft_yaml.exists():
        return path
    raise typer.BadParameter(
        f"Expected graft.yaml at {graft_yaml}, but it does not exist. "
        f"Provide a path to an artifact directory containing graft.yaml"
    )


def _get_orchestrator_service() -> OrchestratorService:
    """Get orchestrator service with config from repo root."""
    repo_root = orchestrator_adapter.get_repo_root()
    orch_config = config_adapter.load_root_config(repo_root)
    return OrchestratorService(orchestrator_adapter, orch_config)


def _perform_autosync(default_policy: SyncPolicy, override: Optional[str], quiet: bool = False) -> Optional[dict]:
    """
    Perform autosync and return orchestrator status for JSON output.

    Args:
        default_policy: Default sync policy for this command
        override: Optional --sync flag value to override policy
        quiet: If True, suppress human-readable output

    Returns:
        Orchestrator status dict for JSON output, or None if not applicable
    """
    # Parse override policy
    sync_policy = default_policy
    if override:
        try:
            sync_policy = SyncPolicy(override)
        except ValueError:
            typer.echo(f"Warning: Invalid --sync value '{override}', using default '{default_policy.value}'", err=True)

    try:
        orch_service = _get_orchestrator_service()
        result = orch_service.autosync(sync_policy=sync_policy)

        # Print summary unless quiet
        if not quiet and result.summary:
            typer.echo(result.summary, err=True)

        return result.status.to_dict()

    except DriftEnforcedError as e:
        typer.echo("Error: Drift detected in enforce mode", err=True)
        typer.echo(f"  create={len(e.plan.create)}, update={len(e.plan.update)}, remove={len(e.plan.remove)}", err=True)
        raise typer.Exit(code=1)

    except InvalidDVCYamlError as e:
        typer.echo(f"Error: {e}", err=True)
        typer.echo("  Run 'graft dvc scaffold --check' to diagnose", err=True)
        raise typer.Exit(code=1)

    except Exception:
        # Silently skip autosync on errors (graceful degradation)
        return None


@app.command()
def explain(
    artifact: str,
    json_out: bool = typer.Option(False, "--json", help="JSON output"),
    sync: Optional[str] = typer.Option(None, "--sync", help="Orchestrator sync policy (off|warn|apply|enforce)")
):
    """Show merged configuration for a graft."""
    try:
        # Perform autosync (default: warn)
        orch_status = _perform_autosync(SyncPolicy.WARN, sync, quiet=json_out)

        artifact_path = _artifact_path(artifact)
        result = explain_service.explain(artifact_path)

        if json_out:
            output = result.to_dict()
            if orch_status:
                output["orchestrator"] = orch_status
            print_json(output)
        else:
            # Human-readable output
            data = result.to_dict()
            typer.echo(f"Artifact: {data['graft']}")
            typer.echo(f"Path: {data['artifact']}")
            typer.echo(f"\nDerivations: {len(data['derivations'])}")
            for deriv in data['derivations']:
                typer.echo(f"  - {deriv['id']}: {len(deriv['outputs'])} output(s)")
            if data.get('inputs', {}).get('materials'):
                typer.echo(f"\nMaterials: {len(data['inputs']['materials'])}")
                for mat in data['inputs']['materials']:
                    typer.echo(f"  - {mat['path']}")
    except typer.BadParameter as e:
        typer.echo(f"Error: {e}", err=True)
        raise typer.Exit(code=1)
    except FileNotFoundError as e:
        typer.echo(f"Error: {e}", err=True)
        raise typer.Exit(code=1)
    except yaml.YAMLError as e:
        typer.echo(f"Error: Invalid YAML in graft.yaml: {e}", err=True)
        raise typer.Exit(code=1)
    except KeyError as e:
        typer.echo(f"Error: Missing required field in graft.yaml: {e}", err=True)
        raise typer.Exit(code=1)
    except PermissionError as e:
        typer.echo(f"System error: Permission denied: {e}", err=True)
        raise typer.Exit(code=2)
    except Exception as e:
        typer.echo(f"System error: {e}", err=True)
        raise typer.Exit(code=2)

@app.command()
def run(
    artifact: str,
    id: Optional[str] = typer.Option(None, "--id", help="Derivation id"),
    sync: Optional[str] = typer.Option(None, "--sync", help="Orchestrator sync policy (off|warn|apply|enforce)")
):
    """Execute derivations by rendering templates to outputs."""
    try:
        # Perform autosync (default: apply)
        _perform_autosync(SyncPolicy.APPLY, sync)

        artifact_path = _artifact_path(artifact)
        result = run_service.run(artifact_path, derivation_id=id)

        # Human-readable output
        typer.echo(f"Run complete: {len(result.derivations_run)} derivation(s), "
                   f"{len(result.outputs_created)} output(s)")

    except typer.BadParameter as e:
        typer.echo(f"Error: {e}", err=True)
        raise typer.Exit(code=1)
    except TemplateNotFoundError as e:
        typer.echo(f"Error: {e}", err=True)
        raise typer.Exit(code=1)
    except FileNotFoundError as e:
        typer.echo(f"Error: {e}", err=True)
        raise typer.Exit(code=1)
    except yaml.YAMLError as e:
        typer.echo(f"Error: Invalid YAML in graft.yaml: {e}", err=True)
        raise typer.Exit(code=1)
    except KeyError as e:
        typer.echo(f"Error: Missing required field in graft.yaml: {e}", err=True)
        raise typer.Exit(code=1)
    except TemplateRenderError as e:
        typer.echo(f"Error: {e}", err=True)
        raise typer.Exit(code=1)
    except MaterialNotFoundError as e:
        typer.echo(f"Error: {e}", err=True)
        raise typer.Exit(code=1)
    except BuildError as e:
        typer.echo(f"Error: {e}", err=True)
        raise typer.Exit(code=1)
    except TransformerExecutionError as e:
        typer.echo(f"Error: {e}", err=True)
        raise typer.Exit(code=1)
    except OutputMissingError as e:
        typer.echo(f"Error: {e}", err=True)
        raise typer.Exit(code=1)
    except PermissionError as e:
        typer.echo(f"System error: Permission denied: {e}", err=True)
        raise typer.Exit(code=2)
    except Exception as e:
        typer.echo(f"System error: {e}", err=True)
        raise typer.Exit(code=2)

@app.command()
def status(
    artifact: str,
    json_out: bool = typer.Option(False, "--json"),
    sync: Optional[str] = typer.Option(None, "--sync", help="Orchestrator sync policy (off|warn|apply|enforce)")
):
    """Report on artifact change status and downstream impacts."""
    try:
        # Perform autosync (default: warn)
        orch_status = _perform_autosync(SyncPolicy.WARN, sync, quiet=json_out)

        artifact_path = _artifact_path(artifact)
        result = status_service.status(artifact_path)

        if json_out:
            output = result.to_dict()
            if orch_status:
                output["orchestrator"] = orch_status
            print_json(output)
        else:
            typer.echo(json.dumps(result.to_dict(), indent=2))
    except typer.BadParameter as e:
        typer.echo(f"Error: {e}", err=True)
        raise typer.Exit(code=1)
    except Exception as e:
        typer.echo(f"System error: {e}", err=True)
        raise typer.Exit(code=2)

@app.command()
def validate(artifact: str):
    """Validate artifact configuration and outputs."""
    a = _artifact_path(artifact)
    typer.echo(f"Validation passed: {a}")

@app.command()
def finalize(
    artifact: str,
    agent: Optional[str] = typer.Option(None, "--agent"),
    model: Optional[str] = typer.Option(None, "--model"),
    params: Optional[str] = typer.Option(None, "--params"),
    sync: Optional[str] = typer.Option(None, "--sync", help="Orchestrator sync policy (off|warn|apply|enforce)")
):
    """Finalize artifact changes and record provenance."""
    try:
        # Perform autosync (default: warn)
        _perform_autosync(SyncPolicy.WARN, sync)

        artifact_path = _artifact_path(artifact)

        # Create agent info if provided
        agent_info = AgentInfo(name=agent, model=model, params=params) if agent else None

        result = finalize_service.finalize(artifact_path, agent=agent_info)
        typer.echo(f"Finalized: {result.provenance_path}")
    except typer.BadParameter as e:
        typer.echo(f"Error: {e}", err=True)
        raise typer.Exit(code=1)
    except Exception as e:
        typer.echo(f"System error: {e}", err=True)
        raise typer.Exit(code=2)

@app.command()
def impact(
    artifact: str,
    json_out: bool = typer.Option(False, "--json"),
    sync: Optional[str] = typer.Option(None, "--sync", help="Orchestrator sync policy (off|warn|apply|enforce)")
):
    """Analyze downstream artifacts affected by changes."""
    # Perform autosync (default: warn)
    orch_status = _perform_autosync(SyncPolicy.WARN, sync, quiet=json_out)

    a = _artifact_path(artifact)
    result = {"artifact": str(a), "downstream": []}
    if orch_status:
        result["orchestrator"] = orch_status
    if json_out:
        print_json(result)
    else:
        typer.echo(json.dumps(result, indent=2))

@app.command()
def simulate(
    artifact: str,
    cascade: bool = typer.Option(False, "--cascade"),
    sync: Optional[str] = typer.Option(None, "--sync", help="Orchestrator sync policy (off|warn|apply|enforce)")
):
    """Simulate artifact build without modifying the repository."""
    # Perform autosync (default: warn)
    _perform_autosync(SyncPolicy.WARN, sync)

    a = _artifact_path(artifact)
    typer.echo(f"Simulation complete for {a} (cascade={'enabled' if cascade else 'disabled'})")

@app.command()
def init(path: str = typer.Argument(".")):
    """Create a minimal root graft config with orchestrator support."""
    p = pathlib.Path(path)
    p.mkdir(parents=True, exist_ok=True)
    cfg = p / "graft.config.yaml"
    if not cfg.exists():
        cfg.write_text("""version: 1
orchestrator:
  type: dvc
  managed_stage_prefix: "graft:"
  sync_policy: apply
  roots: ["."]
defaults:
  policy:
    deterministic: true
    network: off
    attest: required
""", encoding="utf-8")
    typer.echo(f"Initialized {p} with graft.config.yaml")

    # Perform autosync (default: apply) to create initial dvc.yaml if artifacts exist
    _perform_autosync(SyncPolicy.APPLY, None)

@app.command("dvc-scaffold")
def dvc_scaffold(
    check: bool = typer.Option(False, "--check", help="Check for drift without writing"),
    json_out: bool = typer.Option(False, "--json", help="JSON output")
):
    """Manage dvc.yaml stages for all artifacts (authoritative entrypoint)."""
    try:
        orch_service = _get_orchestrator_service()

        # Use scaffold method which handles --check flag
        result = orch_service.scaffold(check_only=check)

        if json_out:
            print_json(result.to_dict())
        else:
            typer.echo(result.summary)

            # If there's drift, show the plan
            if result.status.plan.has_drift:
                typer.echo("\nDrift plan:")
                for item in result.status.plan.create:
                    typer.echo(f"  CREATE: {item.stage_name} - {item.reason}")
                for item in result.status.plan.update:
                    typer.echo(f"  UPDATE: {item.stage_name} - {item.reason}")
                for item in result.status.plan.remove:
                    typer.echo(f"  REMOVE: {item.stage_name} - {item.reason}")

                # Exit code 1 if check mode and drift exists
                if check:
                    raise typer.Exit(code=1)

    except typer.Exit:
        # Re-raise typer.Exit to preserve exit codes
        raise
    except InvalidDVCYamlError as e:
        typer.echo(f"Error: {e}", err=True)
        raise typer.Exit(code=1)
    except (KeyError, ValueError, yaml.YAMLError) as e:
        # User errors (invalid config, bad YAML, etc.)
        typer.echo(f"Error: {e}", err=True)
        raise typer.Exit(code=1)
    except Exception as e:
        typer.echo(f"System error: {e}", err=True)
        raise typer.Exit(code=2)

if __name__ == "__main__":
    app()
