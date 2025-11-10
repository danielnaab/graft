import json, subprocess, sys, pathlib, shutil

def run_cli(*args, cwd=None):
    return subprocess.run([sys.executable, "-m", "graft.cli", *args], capture_output=True, text=True, cwd=cwd)

def test_status_and_finalize(tmp_path):
    src = pathlib.Path("examples/agile-ops/artifacts/sprint-brief/")
    dst = tmp_path / "artifact"
    shutil.copytree(src, dst)
    # Direct edit
    p = dst / "brief.md"
    p.write_text(p.read_text() + "\n- Added note\n", encoding="utf-8")
    st = run_cli("status", str(dst), "--json")
    assert st.returncode == 0
    data = json.loads(st.stdout)
    assert "change_origin" in data
    fin = run_cli("finalize", str(dst), "--agent", "Claude Code", "--model", "claude-3.7")
    assert fin.returncode == 0
    prov = dst / ".graft" / "provenance" / "finalize.json"
    assert prov.exists()
    info = json.loads(prov.read_text())
    assert info["agent"]["name"] == "Claude Code"
