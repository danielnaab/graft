# spec: specs/kb-linter.md
# spec-section: Behavior/Provenance validation

"""Both tools against a notebook-style layout declared in knowledge-base.yaml.

Synthetic fixture: content under notebook/ and at the root, dated notes with no status,
a README that is an index, and a graft directory.
"""

import json
import subprocess
import sys
from pathlib import Path

import pytest

from kb_linter.linter import lint, parse_config
from link_validator.validator import validate
from tool_cli.config import ConfigError

KB_YAML = """\
apiVersion: meta-kb/v1
name: notebook-style

sources:
  canonical:
    - path: AGENTS.md
      note: "Behavior"
    - path: flake.nix

rules:
  lifecycle:
    # Unquoted inline list: valid YAML.
    statuses: [draft, working, stable, deprecated]
  checks:
    links:
      include: [README.md, AGENTS.md, docs/**, notebook/**, .pi/skills/**]
      exclude: [notebook/raw/**]
    lifecycle:
      include: [docs/**, notebook/wiki/**]
      extraStatuses: [reference]
    provenance:
      include:
        - docs/**
        - notebook/wiki/**
"""

GOOD = "---\nstatus: working\n---\n\n# T\n\n## Sources\n- x\n"


def _write(root: Path, rel: str, content: str) -> None:
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content)


def _kb(root: Path, yaml: str = KB_YAML) -> None:
    _write(root, "knowledge-base.yaml", yaml)
    _write(root, "README.md", "# Home\n\n[wiki](notebook/wiki/topic.md)\n")
    _write(root, "AGENTS.md", "# Agents\n\n[docs](docs/guide.md)\n")
    _write(root, "docs/guide.md", GOOD)
    _write(root, "notebook/wiki/topic.md", GOOD)
    _write(root, "notebook/wiki/README.md", "---\nstatus: living\n---\n\n# Index\n")
    _write(root, "notebook/daily/2026-11/2026-11-05.md", "# Day\n\n[g](../../../docs/guide.md)\n")
    _write(root, ".pi/skills/s/SKILL.md", "# Skill\n\n[g](../../../docs/guide.md)\n")
    _write(root, ".scions/README.md", "<!-- managed by graft: https://github.com/danielnaab/graft -->\n")
    _write(root, ".scions/dep/x.md", "[gone](gone.md)\n")


class TestLinkScope:
    def test_scans_root_files_notebook_and_hidden_skills(self, tmp_path: Path) -> None:
        _kb(tmp_path)

        result = validate(str(tmp_path))

        assert result.violations == []
        # README, AGENTS, docs/guide, wiki topic + README, daily note, skill
        assert result.files_checked == 7

    def test_reports_broken_link_in_root_file_and_notebook(self, tmp_path: Path) -> None:
        _kb(tmp_path)
        _write(tmp_path, "README.md", "[bad](notebook/wiki/missing.md)\n")
        _write(tmp_path, "notebook/daily/2026-11/2026-11-05.md", "[bad](nope.md)\n")

        result = validate(str(tmp_path))

        assert sorted(v.file for v in result.violations) == [
            "README.md",
            "notebook/daily/2026-11/2026-11-05.md",
        ]

    def test_exclude_and_graft_dirs_are_skipped(self, tmp_path: Path) -> None:
        _kb(tmp_path)
        _write(tmp_path, "notebook/raw/capture.md", "[bad](nope.md)\n")

        result = validate(str(tmp_path))

        assert result.violations == []  # raw excluded; .scions never walked

    def test_links_into_graft_dir_are_still_checked(self, tmp_path: Path) -> None:
        _kb(tmp_path)
        _write(tmp_path, "docs/guide.md", GOOD + "\n[dep](../.scions/dep/missing.md)\n")

        result = validate(str(tmp_path))

        assert [v.file for v in result.violations] == ["docs/guide.md"]

    def test_defaults_apply_without_configuration(self, tmp_path: Path) -> None:
        _write(tmp_path, "docs/a.md", "[b](missing.md)\n")
        _write(tmp_path, "README.md", "[b](missing.md)\n")  # root is not a default scope

        result = validate(str(tmp_path))

        assert [v.file for v in result.violations] == ["docs/a.md"]

    def test_wrong_shape_is_a_config_error(self, tmp_path: Path) -> None:
        _write(tmp_path, "knowledge-base.yaml", "rules:\n  checks:\n    links:\n      include:\n        - path: x\n")

        with pytest.raises(ConfigError, match="rules.checks.links.include"):
            validate(str(tmp_path))


class TestLintScope:
    def test_clean_layout(self, tmp_path: Path) -> None:
        _kb(tmp_path)

        result = lint(str(tmp_path))

        assert result.violations == []
        assert result.files_checked == 3  # guide, topic, wiki README

    def test_unquoted_statuses_are_enforced(self, tmp_path: Path) -> None:
        _kb(tmp_path)
        assert parse_config(tmp_path).valid_statuses == ["draft", "working", "stable", "deprecated"]
        _write(tmp_path, "docs/guide.md", GOOD.replace("working", "experimental"))

        result = lint(str(tmp_path))

        assert [(v.rule, v.file) for v in result.violations] == [("invalid-status", "docs/guide.md")]

    def test_dated_notes_and_tasks_are_not_checked(self, tmp_path: Path) -> None:
        _kb(tmp_path)
        _write(tmp_path, "notebook/daily/2026-11/2026-11-05.md", "# no frontmatter\n")
        _write(tmp_path, "notebook/tasks/2026-11/t.md", "---\nstatus: ready\n---\n")

        assert lint(str(tmp_path)).violations == []

    def test_living_is_always_allowed_and_extra_statuses_are_configurable(
        self, tmp_path: Path
    ) -> None:
        _kb(tmp_path)
        _write(tmp_path, "docs/guide.md", GOOD.replace("working", "reference"))
        _write(tmp_path, "notebook/wiki/topic.md", GOOD.replace("working", "living"))

        assert lint(str(tmp_path)).violations == []

    def test_living_is_allowed_without_extra_configuration(self, tmp_path: Path) -> None:
        _write(tmp_path, "knowledge-base.yaml", "rules:\n  lifecycle:\n    statuses: [draft, stable]\n")
        _write(tmp_path, "docs/a.md", "---\nstatus: living\n---\n")

        assert lint(str(tmp_path)).violations == []

    def test_provenance_scope_replaces_canonical_paths(self, tmp_path: Path) -> None:
        _kb(tmp_path)
        _write(tmp_path, "notebook/wiki/topic.md", "---\nstatus: working\n---\n\n# T\n")
        _write(tmp_path, "AGENTS.md", "# canonical but not a document that cites sources\n")

        result = lint(str(tmp_path))

        assert [(v.rule, v.file) for v in result.violations] == [
            ("missing-provenance", "notebook/wiki/topic.md")
        ]

    def test_provenance_only_files_are_scanned_without_status_check(self, tmp_path: Path) -> None:
        yaml = KB_YAML.replace("- notebook/wiki/**", "- notebook/wiki/**\n        - notebook/sessions/**")
        _kb(tmp_path, yaml)
        _write(tmp_path, "notebook/sessions/s/design.md", "# Design, no status\n")

        result = lint(str(tmp_path))

        assert [(v.rule, v.file) for v in result.violations] == [
            ("missing-provenance", "notebook/sessions/s/design.md")
        ]

    def test_readme_exempt_from_provenance_even_when_covered(self, tmp_path: Path) -> None:
        _kb(tmp_path)  # notebook/wiki/README.md has no Sources and is covered by the glob

        assert lint(str(tmp_path)).violations == []

    def test_unconfigured_provenance_uses_canonical_within_lifecycle_scope(
        self, tmp_path: Path
    ) -> None:
        _write(
            tmp_path,
            "knowledge-base.yaml",
            "rules:\n  lifecycle:\n    statuses: [working]\nsources:\n  canonical:\n"
            "    - path: docs/**\n    - path: AGENTS.md\n",
        )
        _write(tmp_path, "docs/a.md", "---\nstatus: working\n---\n")
        _write(tmp_path, "AGENTS.md", "# not scanned by default\n")

        result = lint(str(tmp_path))

        assert [(v.rule, v.file) for v in result.violations] == [("missing-provenance", "docs/a.md")]
        assert result.files_checked == 1

    @pytest.mark.parametrize(
        "heading,ok",
        [
            ("## Sources", True),
            ("## Sources and verification", True),
            ("## Sources\t", True),
            ("## Sourcesx", False),
            ("### Sources", False),
            ("## Resources", False),
        ],
    )
    def test_sources_heading_forms(self, tmp_path: Path, heading: str, ok: bool) -> None:
        _kb(tmp_path)
        _write(tmp_path, "docs/guide.md", f"---\nstatus: working\n---\n\n# T\n\n{heading}\n- x\n")

        rules = [v.rule for v in lint(str(tmp_path)).violations]

        assert ("missing-provenance" not in rules) is ok

    def test_unreadable_statuses_is_a_config_error_not_an_empty_set(self, tmp_path: Path) -> None:
        _write(tmp_path, "knowledge-base.yaml", "rules:\n  lifecycle:\n    statuses:\n      a: b\n")
        _write(tmp_path, "docs/a.md", "---\nstatus: whatever\n---\n")

        with pytest.raises(ConfigError, match="rules.lifecycle.statuses"):
            lint(str(tmp_path))


class TestCLIExitCodes:
    def _run(self, module: str, root: Path) -> subprocess.CompletedProcess:
        return subprocess.run(
            [sys.executable, "-m", module, str(root)], capture_output=True, text=True
        )

    def test_config_error_exits_2_with_message(self, tmp_path: Path) -> None:
        _write(tmp_path, "knowledge-base.yaml", "rules:\n  lifecycle:\n    statuses:\n      a: b\n")

        for module in ("kb_linter",):
            proc = self._run(module, tmp_path)
            assert proc.returncode == 2
            assert "rules.lifecycle.statuses" in proc.stderr

    def test_link_validator_config_error_exits_2(self, tmp_path: Path) -> None:
        _write(tmp_path, "knowledge-base.yaml", "rules:\n  checks:\n    links:\n      include:\n        - path: x\n")

        proc = self._run("link_validator", tmp_path)

        assert proc.returncode == 2
        assert "rules.checks.links.include" in proc.stderr

    def test_configured_layout_end_to_end(self, tmp_path: Path) -> None:
        _kb(tmp_path)

        for module in ("kb_linter", "link_validator"):
            proc = self._run(module, tmp_path)
            assert proc.returncode == 0, proc.stdout + proc.stderr
            assert json.loads(proc.stdout)["violations"] == []
