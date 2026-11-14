# spec: specs/kb-linter.md
# spec-section: Behavior/Exit codes

"""Integration tests for the KB linter CLI."""

import json
import subprocess
import sys
from pathlib import Path

KB_YAML = """\
apiVersion: kb/v1
name: test-kb

rules:
  lifecycle:
    statuses: ["draft", "working", "stable", "deprecated"]

sources:
  canonical:
    - path: docs/**
    - path: policies/**
"""


def _run_linter(tmp_path: Path, *extra_args: str) -> subprocess.CompletedProcess:
    """Run the KB linter CLI on a temp directory."""
    return subprocess.run(
        [sys.executable, "-m", "kb_linter", str(tmp_path), *extra_args],
        capture_output=True,
        text=True,
    )


def _setup_kb(tmp_path: Path) -> None:
    (tmp_path / "knowledge-base.yaml").write_text(KB_YAML)
    (tmp_path / "docs").mkdir()
    (tmp_path / "policies").mkdir()


class TestCLIOutput:
    def test_produces_valid_json(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/test.md").write_text(
            "---\nstatus: working\n---\n\n# Test\n\n## Sources\n- x\n"
        )

        proc = _run_linter(tmp_path)
        output = json.loads(proc.stdout)

        assert "violations" in output
        assert "summary" in output

    def test_summary_counts(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/good.md").write_text(
            "---\nstatus: working\n---\n\n# Good\n\n## Sources\n- x\n"
        )
        (tmp_path / "docs/bad.md").write_text("# No frontmatter\n")

        proc = _run_linter(tmp_path)
        output = json.loads(proc.stdout)

        assert output["summary"]["files_checked"] == 2
        assert output["summary"]["violations"] >= 1


class TestExitCodes:
    def test_exit_0_when_no_violations(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/clean.md").write_text(
            "---\nstatus: working\n---\n\n# Clean\n\n## Sources\n- x\n"
        )

        proc = _run_linter(tmp_path)

        assert proc.returncode == 0

    def test_exit_1_when_violations(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/bad.md").write_text("# No frontmatter\n")

        proc = _run_linter(tmp_path)

        assert proc.returncode == 1

    def test_report_only_exit_0_with_violations(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/bad.md").write_text("# No frontmatter\n")

        proc = _run_linter(tmp_path, "--report-only")

        assert proc.returncode == 0

    def test_exit_2_when_config_missing(self, tmp_path: Path) -> None:
        # No knowledge-base.yaml
        proc = _run_linter(tmp_path)

        assert proc.returncode == 2

    def test_report_only_still_produces_output(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/bad.md").write_text("# No frontmatter\n")

        proc = _run_linter(tmp_path, "--report-only")
        output = json.loads(proc.stdout)

        assert output["summary"]["violations"] >= 1
