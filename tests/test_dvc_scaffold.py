import subprocess, sys, pathlib, shutil, json, os, yaml

def test_dvc_scaffold(tmp_path):
    # copy examples project into tmp_path
    src_root = pathlib.Path("examples/agile-ops/")
    dst_root = tmp_path / "project"
    shutil.copytree(src_root, dst_root)

    # Set PYTHONPATH for the subprocess
    env = os.environ.copy()
    src_path = str(pathlib.Path(__file__).parent.parent / "src")
    env["PYTHONPATH"] = src_path

    out = subprocess.run([sys.executable, "-m", "graft.cli", "dvc-scaffold"],
                         capture_output=True, text=True, cwd=str(dst_root), env=env)
    assert out.returncode == 0, f"Command failed: stdout={out.stdout}, stderr={out.stderr}"

    assert (dst_root / "dvc.yaml").exists(), f"dvc.yaml not created. stdout={out.stdout}, stderr={out.stderr}"
    dvc_yaml = (dst_root / "dvc.yaml").read_text()
    # Now stored as YAML (changed from JSON in Slice 5)
    data = yaml.safe_load(dvc_yaml)
    assert "stages" in data and data["stages"]
    # Stage names now follow pattern: graft:<artifact>:<derivation-id>
    assert any(k.startswith("graft:sprint-brief:") for k in data["stages"].keys())
    # Find the sprint-brief stage
    sprint_stage = next((v for k, v in data["stages"].items() if k.startswith("graft:sprint-brief:")), None)
    assert sprint_stage is not None
    assert sprint_stage["cmd"].startswith("graft run ")
