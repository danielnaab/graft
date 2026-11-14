# spec: specs/kb-linter.md

"""Tests for the KB content linter."""

from pathlib import Path

from kb_linter.linter import lint, parse_config

KB_YAML = """\
apiVersion: kb/v1
name: test-kb

rules:
  lifecycle:
    statuses: ["draft", "working", "stable", "deprecated"]

sources:
  canonical:
    - path: docs/**
      note: "Docs are canonical"
    - path: policies/**
      note: "Policies are normative"
"""


def _setup_kb(tmp_path: Path, kb_yaml: str = KB_YAML) -> None:
    """Create a minimal KB structure for testing."""
    (tmp_path / "knowledge-base.yaml").write_text(kb_yaml)
    (tmp_path / "docs").mkdir()
    (tmp_path / "policies").mkdir()
    (tmp_path / "playbooks").mkdir()


class TestConfigParsing:
    def test_reads_valid_statuses(self, tmp_path: Path) -> None:
        (tmp_path / "knowledge-base.yaml").write_text(KB_YAML)

        config = parse_config(tmp_path)

        assert config.valid_statuses == ["draft", "working", "stable", "deprecated"]

    def test_reads_canonical_paths(self, tmp_path: Path) -> None:
        (tmp_path / "knowledge-base.yaml").write_text(KB_YAML)

        config = parse_config(tmp_path)

        assert "docs/**" in config.provenance_paths
        assert "policies/**" in config.provenance_paths

    def test_stops_at_sibling_block_after_canonical(self, tmp_path: Path) -> None:
        (tmp_path / "knowledge-base.yaml").write_text(
            "sources:\n"
            "  canonical:\n"
            "    - path: docs/decisions/**\n"
            "      note: \"Decisions are canonical\"\n"
            "  interpretation:\n"
            "    - path: docs/README.md\n"
            "    - path: docs/agents.md\n"
            "  tracking:\n"
            "    - path: notes/**\n"
        )

        config = parse_config(tmp_path)

        assert config.provenance_paths == ["docs/decisions/**"]

    def test_reads_canonical_when_last_block(self, tmp_path: Path) -> None:
        (tmp_path / "knowledge-base.yaml").write_text(
            "sources:\n"
            "  interpretation:\n"
            "    - path: docs/README.md\n"
            "  canonical:\n"
            "    - path: docs/**\n"
            "      note: \"Docs are canonical\"\n"
            "\n"
            "    # comment inside the block\n"
            "    - path: policies/**\n"
        )

        config = parse_config(tmp_path)

        assert config.provenance_paths == ["docs/**", "policies/**"]

    def test_skips_nested_note_lines(self, tmp_path: Path) -> None:
        (tmp_path / "knowledge-base.yaml").write_text(KB_YAML)

        config = parse_config(tmp_path)

        assert config.provenance_paths == ["docs/**", "policies/**"]

    def test_reads_list_items_at_same_indent_as_canonical(self, tmp_path: Path) -> None:
        (tmp_path / "knowledge-base.yaml").write_text(
            "sources:\n"
            "  canonical:\n"
            "  - path: docs/**\n"
            "    note: \"Docs\"\n"
            "  tracking:\n"
            "  - path: notes/**\n"
        )

        config = parse_config(tmp_path)

        assert config.provenance_paths == ["docs/**"]

    def test_raises_when_config_missing(self, tmp_path: Path) -> None:
        import pytest

        with pytest.raises(FileNotFoundError):
            parse_config(tmp_path)


class TestFrontmatterValidation:
    def test_reports_missing_frontmatter(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/no-frontmatter.md").write_text("# No Frontmatter\n\nContent here.")

        result = lint(str(tmp_path))

        rules = [v.rule for v in result.violations if v.file == "docs/no-frontmatter.md"]
        assert "missing-frontmatter" in rules

    def test_reports_missing_status(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/no-status.md").write_text("---\ntitle: Test\n---\n\n# Content\n")

        result = lint(str(tmp_path))

        rules = [v.rule for v in result.violations if v.file == "docs/no-status.md"]
        assert "missing-status" in rules

    def test_reports_invalid_status(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/bad-status.md").write_text(
            "---\nstatus: experimental\n---\n\n# Content\n\n## Sources\n- src\n"
        )

        result = lint(str(tmp_path))

        violations = [v for v in result.violations if v.rule == "invalid-status"]
        assert len(violations) == 1
        assert "experimental" in violations[0].message
        assert "draft" in violations[0].message

    def test_passes_valid_status(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/valid.md").write_text(
            "---\nstatus: working\n---\n\n# Content\n\n## Sources\n- src\n"
        )

        result = lint(str(tmp_path))

        files_with_violations = {v.file for v in result.violations}
        assert "docs/valid.md" not in files_with_violations

    def test_accepts_all_defined_statuses(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        for status in ["draft", "working", "stable", "deprecated"]:
            (tmp_path / f"docs/{status}.md").write_text(
                f"---\nstatus: {status}\n---\n\n# {status}\n\n## Sources\n- src\n"
            )

        result = lint(str(tmp_path))

        status_violations = [v for v in result.violations if v.rule == "invalid-status"]
        assert status_violations == []

    def test_accepts_decision_statuses_in_decisions_dir(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/decisions").mkdir()
        for i, status in enumerate(
            ["proposed", "accepted", "rejected", "superseded", "deprecated", "working"]
        ):
            (tmp_path / f"docs/decisions/000{i}-x.md").write_text(
                f"---\nstatus: {status}\n---\n\n# ADR\n\n## Sources\n- x\n"
            )

        result = lint(str(tmp_path))

        assert result.violations == []

    def test_accepts_decision_status_in_adr_named_file(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/specifications/grove").mkdir(parents=True)
        (tmp_path / "docs/specifications/grove/adr-001-layout.md").write_text(
            "---\nstatus: accepted\n---\n\n# ADR\n\n## Sources\n- x\n"
        )
        (tmp_path / "docs/ADR-002-other.md").write_text(
            "---\nstatus: superseded\n---\n\n# ADR\n\n## Sources\n- x\n"
        )

        result = lint(str(tmp_path))

        assert result.violations == []

    def test_rejects_decision_status_outside_decision_records(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/guide.md").write_text(
            "---\nstatus: accepted\n---\n\n# Guide\n\n## Sources\n- x\n"
        )
        (tmp_path / "docs/decisions.md").write_text(
            "---\nstatus: accepted\n---\n\n# Not a record\n\n## Sources\n- x\n"
        )
        (tmp_path / "docs/my-adr-notes.md").write_text(
            "---\nstatus: accepted\n---\n\n# Not a record\n\n## Sources\n- x\n"
        )

        result = lint(str(tmp_path))

        invalid = sorted(v.file for v in result.violations if v.rule == "invalid-status")
        assert invalid == ["docs/decisions.md", "docs/guide.md", "docs/my-adr-notes.md"]

    def test_decision_record_with_unknown_status_lists_both_sets(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/decisions").mkdir()
        (tmp_path / "docs/decisions/0001-x.md").write_text(
            "---\nstatus: maybe\n---\n\n# ADR\n\n## Sources\n- x\n"
        )

        result = lint(str(tmp_path))

        assert [v.rule for v in result.violations] == ["invalid-status"]
        assert "accepted" in result.violations[0].message
        assert "draft" in result.violations[0].message

    def test_decision_record_still_requires_status(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/decisions").mkdir()
        (tmp_path / "docs/decisions/0001-x.md").write_text(
            "---\ntitle: x\n---\n\n# ADR\n\n## Sources\n- x\n"
        )

        result = lint(str(tmp_path))

        assert [v.rule for v in result.violations] == ["missing-status"]

    def test_reports_empty_frontmatter(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/empty-fm.md").write_text("---\n---\n\n# Content\n")

        result = lint(str(tmp_path))

        rules = [v.rule for v in result.violations if v.file == "docs/empty-fm.md"]
        assert "missing-status" in rules

    def test_malformed_frontmatter_reports_missing_status(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/malformed.md").write_text("---\n{not: [valid yaml\n---\n\n# Content\n")

        result = lint(str(tmp_path))

        rules = [v.rule for v in result.violations if v.file == "docs/malformed.md"]
        assert "missing-status" in rules


class TestProvenanceValidation:
    def test_reports_missing_sources_in_docs(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/no-sources.md").write_text(
            "---\nstatus: working\n---\n\n# Content\n\nNo sources here.\n"
        )

        result = lint(str(tmp_path))

        rules = [v.rule for v in result.violations if v.file == "docs/no-sources.md"]
        assert "missing-provenance" in rules

    def test_reports_missing_sources_in_policies(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "policies/rule.md").write_text(
            "---\nstatus: working\n---\n\n# A Policy\n\nNo sources.\n"
        )

        result = lint(str(tmp_path))

        rules = [v.rule for v in result.violations if v.file == "policies/rule.md"]
        assert "missing-provenance" in rules

    def test_passes_with_sources_section(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/with-sources.md").write_text(
            "---\nstatus: working\n---\n\n# Content\n\n## Sources\n\n- [Ref](url)\n"
        )

        result = lint(str(tmp_path))

        files_with_violations = {v.file for v in result.violations}
        assert "docs/with-sources.md" not in files_with_violations

    def test_does_not_require_sources_in_playbooks(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "playbooks/guide.md").write_text(
            "---\nstatus: working\n---\n\n# Guide\n\nNo sources needed.\n"
        )

        result = lint(str(tmp_path))

        provenance_violations = [
            v
            for v in result.violations
            if v.file == "playbooks/guide.md" and v.rule == "missing-provenance"
        ]
        assert provenance_violations == []

    def test_readme_in_canonical_path_needs_no_sources(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/decisions").mkdir()
        (tmp_path / "docs/README.md").write_text("---\nstatus: working\n---\n\n# Index\n")
        (tmp_path / "docs/decisions/README.md").write_text(
            "---\nstatus: working\n---\n\n# Decisions\n"
        )

        result = lint(str(tmp_path))

        assert result.violations == []

    def test_readme_exemption_is_provenance_only(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/decisions").mkdir()
        (tmp_path / "docs/decisions/README.md").write_text("---\nstatus: bogus\n---\n\n# D\n")
        (tmp_path / "docs/decisions/0001-x.md").write_text(
            "---\nstatus: accepted\n---\n\n# ADR without sources\n"
        )

        result = lint(str(tmp_path))

        found = sorted((v.file, v.rule) for v in result.violations)
        assert found == [
            ("docs/decisions/0001-x.md", "missing-provenance"),
            ("docs/decisions/README.md", "invalid-status"),
        ]

    def test_checks_subdirectories(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/sub").mkdir()
        (tmp_path / "docs/sub/nested.md").write_text(
            "---\nstatus: working\n---\n\n# Nested\n\nNo sources.\n"
        )

        result = lint(str(tmp_path))

        rules = [v.rule for v in result.violations if v.file == "docs/sub/nested.md"]
        assert "missing-provenance" in rules


class TestScannedPaths:
    def test_does_not_scan_notes(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "notes").mkdir()
        (tmp_path / "notes/exploration.md").write_text("# No frontmatter, no problem")
        # Add a docs file so we can confirm scanning works at all
        (tmp_path / "docs/real.md").write_text("---\nstatus: working\n---\n\n## Sources\n- x\n")

        result = lint(str(tmp_path))

        assert result.files_checked == 1  # Only the docs file
        assert not any(v.file.startswith("notes/") for v in result.violations)

    def test_does_not_scan_specs(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "specs").mkdir()
        (tmp_path / "specs/tool.md").write_text("# No frontmatter")

        result = lint(str(tmp_path))

        files_checked = [v.file for v in result.violations]
        assert not any(f.startswith("specs/") for f in files_checked)

    def test_counts_files_checked(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/a.md").write_text("---\nstatus: working\n---\n\n## Sources\n- x\n")
        (tmp_path / "docs/b.md").write_text("---\nstatus: working\n---\n\n## Sources\n- x\n")
        (tmp_path / "policies/c.md").write_text("---\nstatus: working\n---\n\n## Sources\n- x\n")

        result = lint(str(tmp_path))

        assert result.files_checked == 3

    def test_handles_missing_content_dirs(self, tmp_path: Path) -> None:
        (tmp_path / "knowledge-base.yaml").write_text(KB_YAML)
        # No docs/, policies/, playbooks/ directories

        result = lint(str(tmp_path))

        assert result.files_checked == 0
        assert result.violations == []


class TestEdgeCases:
    def test_skips_non_markdown_files(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/image.png").write_text("not markdown")

        result = lint(str(tmp_path))

        assert result.files_checked == 0

    def test_includes_readme_files(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/README.md").write_text(
            "---\nstatus: working\n---\n\n# Index\n\n## Sources\n- x\n"
        )

        result = lint(str(tmp_path))

        assert result.files_checked == 1
        assert result.violations == []
