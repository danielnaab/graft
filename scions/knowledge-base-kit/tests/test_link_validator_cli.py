# spec: specs/link-validator.md
# spec-section: Behavior/Exit codes

"""Integration tests for the link validator CLI."""

import json
import subprocess
import sys
from pathlib import Path


def _run_validator(tmp_path: Path, *args: str) -> subprocess.CompletedProcess:
    """Run link-validator CLI on a temp directory."""
    cmd = [sys.executable, "-m", "link_validator", str(tmp_path), *args]
    return subprocess.run(cmd, capture_output=True, text=True)


def _setup_kb(tmp_path: Path) -> None:
    """Create a minimal KB structure."""
    (tmp_path / "docs").mkdir()


class TestCLIOutput:
    def test_outputs_valid_json(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/a.md").write_text("[b](b.md)\n")
        (tmp_path / "docs/b.md").write_text("# B\n")

        proc = _run_validator(tmp_path)

        output = json.loads(proc.stdout)
        assert "violations" in output
        assert "summary" in output

    def test_summary_counts(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/a.md").write_text("[b](b.md) and [c](missing.md)\n")
        (tmp_path / "docs/b.md").write_text("# B\n")

        proc = _run_validator(tmp_path, "--report-only")

        output = json.loads(proc.stdout)
        assert output["summary"]["files_checked"] == 2
        assert output["summary"]["links_checked"] == 2
        assert output["summary"]["broken"] == 1


class TestExitCodes:
    def test_exit_0_when_no_broken_links(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/a.md").write_text("[b](b.md)\n")
        (tmp_path / "docs/b.md").write_text("# B\n")

        proc = _run_validator(tmp_path)

        assert proc.returncode == 0

    def test_exit_1_when_broken_links(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/a.md").write_text("[missing](gone.md)\n")

        proc = _run_validator(tmp_path)

        assert proc.returncode == 1

    def test_report_only_exits_0_with_broken_links(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/a.md").write_text("[missing](gone.md)\n")

        proc = _run_validator(tmp_path, "--report-only")

        assert proc.returncode == 0

    def test_exit_0_when_no_files(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)

        proc = _run_validator(tmp_path)

        assert proc.returncode == 0
