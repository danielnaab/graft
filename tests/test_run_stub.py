import subprocess, sys, pathlib, shutil

def test_run_writes_output(tmp_path):
    src = pathlib.Path("examples/agile-ops/artifacts/sprint-brief/")
    dst = tmp_path / "artifact"
    shutil.copytree(src, dst)
    out = subprocess.run([sys.executable, "-m", "graft.cli", "run", str(dst)],
                         capture_output=True, text=True)
    assert out.returncode == 0
    out_file = dst / "brief.md"
    assert out_file.exists()
    assert "Sprint Brief" in out_file.read_text()
