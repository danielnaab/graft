import json, subprocess, sys, pathlib, shutil, os

def run_cli(*args, cwd=None):
    # Set PYTHONPATH for the subprocess
    env = os.environ.copy()
    src_path = str(pathlib.Path(__file__).parent.parent / "src")
    env["PYTHONPATH"] = src_path
    return subprocess.run([sys.executable, "-m", "graft.cli", *args], capture_output=True, text=True, cwd=cwd, env=env)

def test_impact_and_simulate(tmp_path):
    src = pathlib.Path("examples/agile-ops/artifacts/sprint-brief/")
    dst = tmp_path / "artifact"
    shutil.copytree(src, dst)
    imp = run_cli("impact", str(dst), "--json")
    assert imp.returncode == 0
    data = json.loads(imp.stdout)
    assert "downstream" in data
    sim = run_cli("simulate", str(dst), "--cascade")
    assert sim.returncode == 0
    assert "Simulation complete" in sim.stdout
