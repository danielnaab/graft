    from __future__ import annotations
    import json, pathlib, sys, time
    import typer
    from typing import Optional, List
    from .utils import load_yaml, print_json

    app = typer.Typer(add_completion=False, help="Graft starter CLI (stub — outside-in contract)")

    def _artifact_path(p: str) -> pathlib.Path:
        path = pathlib.Path(p)
        if (path / "graft.yaml").exists():
            return path
        raise typer.BadParameter("Expected an artifact directory containing graft.yaml")

    @app.command()
    def explain(artifact: str, json_out: bool = typer.Option(False, "--json", help="JSON output")):
        """Show merged configuration for a graft (stub)."""
        a = _artifact_path(artifact)
        conf = load_yaml(a / "graft.yaml")
        result = {
            "artifact": str(a),
            "graft": conf.get("graft"),
            "policy": conf.get("policy", {}),
            "inputs": conf.get("inputs", {}),
            "derivations": [d.get("id") for d in conf.get("derivations", [])]
        }
        if json_out:
            print_json(result)
        else:
            typer.echo(json.dumps(result, indent=2))

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
