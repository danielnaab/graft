import json, subprocess, sys, pathlib

def test_explain_json():
    repo = pathlib.Path("examples/agile-ops/artifacts/sprint-brief/")
    out = subprocess.run([sys.executable, "-m", "graft.cli", "explain", str(repo), "--json"],
                         capture_output=True, text=True)
    assert out.returncode == 0
    data = json.loads(out.stdout)
    assert data["graft"] == "sprint-brief"
    assert "derivations" in data and data["derivations"]
