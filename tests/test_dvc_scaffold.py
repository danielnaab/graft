import subprocess, sys, pathlib, shutil, json

def test_dvc_scaffold(tmp_path):
    # copy examples project into tmp_path
    src_root = pathlib.Path("examples/agile-ops/")
    dst_root = tmp_path / "project"
    shutil.copytree(src_root, dst_root)
    out = subprocess.run([sys.executable, "-m", "graft.cli", "dvc-scaffold", str(dst_root)],
                         capture_output=True, text=True)
    assert out.returncode == 0
    dvc_yaml = (dst_root / "dvc.yaml").read_text()
    # stored as JSON for simplicity
    data = json.loads(dvc_yaml)
    assert "stages" in data and data["stages"]
    assert "sprint-brief" in data["stages"]
    assert data["stages"]["sprint-brief"]["cmd"].startswith("graft run ")
