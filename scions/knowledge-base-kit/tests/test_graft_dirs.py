# spec: specs/tool-cli.md

"""Directories graft wrote are skipped by every tool; links into them still resolve."""

from pathlib import Path

from backlink_scanner.scanner import scan
from kb_linter.linter import lint
from link_validator.validator import validate
from tool_cli.graft import is_graft_dir

MARKER_21 = "<!-- managed by graft: https://github.com/danielnaab/graft -->\n# scions/\n"
MARKER_20 = "# vendor/\n\nManaged by [graft](https://github.com/danielnaab/graft).\n"


def _graft_dir(root: Path, rel: str, readme: str = MARKER_21) -> Path:
    d = root / rel
    (d / "dep/docs").mkdir(parents=True)
    (d / "README.md").write_text(readme)
    return d


def test_is_graft_dir_recognizes_both_markers(tmp_path: Path) -> None:
    assert is_graft_dir(_graft_dir(tmp_path, "a", MARKER_21))
    assert is_graft_dir(_graft_dir(tmp_path, "b", MARKER_20))


def test_is_graft_dir_rejects_other_directories(tmp_path: Path) -> None:
    (tmp_path / "plain").mkdir()
    (tmp_path / "plain/README.md").write_text("# plain\n\nNot graft's.\n")
    (tmp_path / "bare").mkdir()
    assert not is_graft_dir(tmp_path / "plain")
    assert not is_graft_dir(tmp_path / "bare")


def test_link_validator_skips_a_graft_dir_inside_docs(tmp_path: Path) -> None:
    (tmp_path / "docs").mkdir()
    g = _graft_dir(tmp_path, "docs/Grafted")
    (g / "dep/docs/upstream.md").write_text("[broken upstream](missing.md)\n")
    (tmp_path / "docs/own.md").write_text("[into graft](Grafted/dep/docs/upstream.md)\n")

    result = validate(str(tmp_path))

    assert result.violations == []


def test_link_validator_still_reports_a_broken_link_into_a_graft_dir(tmp_path: Path) -> None:
    (tmp_path / "docs").mkdir()
    _graft_dir(tmp_path, "scions")
    (tmp_path / "docs/own.md").write_text("[gone](../scions/dep/docs/gone.md)\n")

    result = validate(str(tmp_path))

    assert [v.file for v in result.violations] == ["docs/own.md"]


def test_kb_linter_skips_a_graft_dir_inside_docs(tmp_path: Path) -> None:
    (tmp_path / "knowledge-base.yaml").write_text(
        'apiVersion: kb/v1\nname: t\nrules:\n  lifecycle:\n    statuses: ["draft"]\n'
    )
    (tmp_path / "docs").mkdir()
    g = _graft_dir(tmp_path, "docs/Grafted")
    (g / "dep/docs/upstream.md").write_text("---\nstatus: nonsense\n---\n# Upstream\n")

    result = lint(str(tmp_path))

    assert result.violations == []


def test_backlink_scanner_skips_annotations_in_a_graft_dir(tmp_path: Path) -> None:
    g = _graft_dir(tmp_path, "scions")
    (g / "dep/docs/example.py").write_text("# spec: specs/upstream-only.md\n")

    result = scan(str(tmp_path))

    assert result.dangling == []
    assert result.specs == {}
