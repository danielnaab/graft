from __future__ import annotations
import json, pathlib, time, yaml
import typer
from typing import Optional

from .utils import print_json, load_yaml
from .adapters.filesystem import LocalFileSystem
from .adapters.config import ConfigAdapter
from .services.explain import ExplainService

app = typer.Typer(add_completion=False, help="Graft starter CLI (stub — outside-in contract)")

# Initialize adapters and services (dependency injection)
fs = LocalFileSystem()
config_adapter = ConfigAdapter(fs)
explain_service = ExplainService(config_adapter)


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


@app.command()
def explain(artifact: str, json_out: bool = typer.Option(False, "--json", help="JSON output")):
    """Show merged configuration for a graft."""
    try:
        artifact_path = _artifact_path(artifact)
        result = explain_service.explain(artifact_path)

        if json_out:
            print_json(result.to_dict())
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
def run(artifact: str, id: Optional[str] = typer.Option(None, "--id", help="Derivation id")):
    """Execute the graft (stub: copy template file(s) to outputs)."""
    a = _artifact_path(artifact)
    conf = load_yaml(a / "graft.yaml")
    for d in conf.get("derivations", []):
        if id and d.get("id") != id:
            continue
        outs = d.get("outputs", [])
        template = d.get("template", {})
        if template.get("file"):
            src = (a / template["file"])
            if not src.exists():
                raise typer.Exit(code=1)
            data = src.read_bytes()
            for o in outs:
                out_path = a / o["path"]
                out_path.parent.mkdir(parents=True, exist_ok=True)
                out_path.write_bytes(data)
    typer.echo("Run complete (stub).")

@app.command()
def status(artifact: str, json_out: bool = typer.Option(False, "--json")):
    """Report authored vs generated changes (stub)."""
    a = _artifact_path(artifact)
    result = {"artifact": str(a), "change_origin": "unknown", "downstream": []}
    if json_out:
        print_json(result)
    else:
        typer.echo(json.dumps(result, indent=2))

@app.command()
def validate(artifact: str):
    a = _artifact_path(artifact)
    typer.echo(f"Validate OK (stub) for {a}")

@app.command()
def finalize(artifact: str, agent: Optional[str] = typer.Option(None, "--agent"),
             model: Optional[str] = typer.Option(None, "--model"),
             params: Optional[str] = typer.Option(None, "--params")):
    a = _artifact_path(artifact)
    prov = {
        "artifact": str(a),
        "finalized_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "agent": {"name": agent, "model": model, "params": params} if agent else None,
        "change_origin": "authored"
    }
    prov_path = a / ".graft" / "provenance" / "finalize.json"
    prov_path.parent.mkdir(parents=True, exist_ok=True)
    prov_path.write_text(json.dumps(prov, indent=2))
    typer.echo(f"Finalized (stub). Wrote {prov_path}")

@app.command()
def impact(artifact: str, json_out: bool = typer.Option(False, "--json")):
    """Analyze downstream impact (stub)."""
    a = _artifact_path(artifact)
    result = {"artifact": str(a), "downstream": []}
    if json_out:
        print_json(result)
    else:
        typer.echo(json.dumps(result, indent=2))

@app.command()
def simulate(artifact: str, cascade: bool = typer.Option(False, "--cascade")):
    """Build in a sandbox without writing to the repo (stub)."""
    a = _artifact_path(artifact)
    typer.echo(f"Simulated build for {a} (cascade={cascade})")

@app.command()
def init(path: str = typer.Argument(".")):
    """Create a minimal root graft config (no DVC changes)."""
    p = pathlib.Path(path)
    p.mkdir(parents=True, exist_ok=True)
    cfg = p / "graft.config.yaml"
    if not cfg.exists():
        cfg.write_text("""version: 1
defaults:
  policy:
    deterministic: true
    network: off
    attest: required
""", encoding="utf-8")
    typer.echo(f"Initialized {p} with graft.config.yaml")

@app.command("dvc-scaffold")
def dvc_scaffold(project_root: str = typer.Argument(".")):
    """Generate a dvc.yaml with stages that wrap graft runs. Does not run 'dvc init'."""
    root = pathlib.Path(project_root).resolve()
    stages = {}
    for art in root.rglob("graft.yaml"):
        art_dir = art.parent
        conf = load_yaml(art)
        name = conf.get("graft", art_dir.name)
        deps = []
        for m in conf.get("inputs", {}).get("materials", []):
            deps.append(str((art_dir / m["path"]).resolve().relative_to(root)))
        deps.append(str((art_dir / "graft.yaml").resolve().relative_to(root)))
        outs = []
        for der in conf.get("derivations", []):
            for o in der.get("outputs", []):
                outs.append(str((art_dir / o["path"]).resolve().relative_to(root)))
        stages[name] = {
            "cmd": f"graft run {art_dir.as_posix()}",
            "deps": sorted(set(deps)),
            "outs": sorted(set(outs)),
        }
    dvc_yaml = {"stages": stages}
    (root / "dvc.yaml").write_text(json.dumps(dvc_yaml, indent=2), encoding="utf-8")
    typer.echo(f"Wrote {root / 'dvc.yaml'} (scaffold). Now run 'dvc init' and configure remotes as needed.")

if __name__ == "__main__":
    app()
