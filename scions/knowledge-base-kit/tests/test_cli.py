# spec: specs/backlink-scanner.md
# spec-section: Behavior/Exit codes

"""Integration tests for the backlink scanner CLI."""

import json
import subprocess
import sys
from pathlib import Path


def _run_scanner(tmp_path: Path, *extra_args: str) -> subprocess.CompletedProcess:
    """Run the backlink scanner CLI on a temp directory."""
    return subprocess.run(
        [sys.executable, "-m", "backlink_scanner", str(tmp_path), *extra_args],
        capture_output=True,
        text=True,
    )


class TestCLIOutput:
    def test_produces_valid_json(self, tmp_path: Path) -> None:
        (tmp_path / "specs").mkdir()
        (tmp_path / "specs/auth.md").write_text("# Auth")
        (tmp_path / "src.py").write_text("# spec: specs/auth.md")

        proc = _run_scanner(tmp_path)
        output = json.loads(proc.stdout)

        assert "specs" in output
        assert "dangling" in output
        assert "orphans" in output

    def test_output_has_nested_structure(self, tmp_path: Path) -> None:
        (tmp_path / "specs").mkdir()
        (tmp_path / "specs/auth.md").write_text("# Auth")
        (tmp_path / "src.py").write_text("# spec: specs/auth.md")

        proc = _run_scanner(tmp_path)
        output = json.loads(proc.stdout)

        spec_entry = output["specs"]["specs/auth.md"]
        assert "implementors" in spec_entry
        assert "sections" in spec_entry
        assert spec_entry["implementors"] == ["src.py"]


class TestExitCodes:
    def test_exit_0_when_no_issues(self, tmp_path: Path) -> None:
        (tmp_path / "specs").mkdir()
        (tmp_path / "specs/auth.md").write_text("# Auth")
        (tmp_path / "src.py").write_text("# spec: specs/auth.md")

        proc = _run_scanner(tmp_path)

        assert proc.returncode == 0

    def test_exit_1_when_dangling_refs(self, tmp_path: Path) -> None:
        (tmp_path / "src.py").write_text("# spec: specs/nonexistent.md")

        proc = _run_scanner(tmp_path)

        assert proc.returncode == 1

    def test_exit_1_when_orphan_specs(self, tmp_path: Path) -> None:
        (tmp_path / "specs").mkdir()
        (tmp_path / "specs/orphan.md").write_text("# Orphan")
        (tmp_path / "src.py").write_text("x = 1")

        proc = _run_scanner(tmp_path)

        assert proc.returncode == 1

    def test_report_only_exit_0_with_dangling(self, tmp_path: Path) -> None:
        (tmp_path / "src.py").write_text("# spec: specs/nonexistent.md")

        proc = _run_scanner(tmp_path, "--report-only")

        assert proc.returncode == 0

    def test_report_only_exit_0_with_orphans(self, tmp_path: Path) -> None:
        (tmp_path / "specs").mkdir()
        (tmp_path / "specs/orphan.md").write_text("# Orphan")
        (tmp_path / "src.py").write_text("x = 1")

        proc = _run_scanner(tmp_path, "--report-only")

        assert proc.returncode == 0

    def test_report_only_still_produces_output(self, tmp_path: Path) -> None:
        (tmp_path / "src.py").write_text("# spec: specs/nonexistent.md")

        proc = _run_scanner(tmp_path, "--report-only")
        output = json.loads(proc.stdout)

        assert "specs/nonexistent.md" in output["dangling"]
