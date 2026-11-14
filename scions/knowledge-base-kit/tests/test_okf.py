# spec: specs/tool-cli.md
# spec-section: OKF bundle

"""Tests for OKF bundles: configuration, .okfignore, conformance, structured provenance, link classes."""

import json
import subprocess
import sys
from pathlib import Path

import pytest

from kb_linter.linter import lint
from link_validator.anchors import github_slug, has_anchor
from link_validator.validator import validate
from tool_cli.config import ConfigError, parse_subset
from tool_cli.okf import load_bundle, parse_ignore, split_frontmatter, structured_sources

KB_YAML = """\
rules:
  lifecycle:
    statuses: [draft, working, stable, deprecated]
  checks:
    links:
      include: ["**"]
    lifecycle:
      include: [docs/**, notebook/wiki/**]
    provenance:
      include: [notebook/wiki/**]
    okf:
      bundle: notebook
      version: "0.2"
"""

ROOT_INDEX = '---\nokf_version: "0.2"\n---\n# Notebook\n'


def _kb(tmp_path: Path, files: dict[str, str], kb_yaml: str = KB_YAML) -> Path:
    (tmp_path / "knowledge-base.yaml").write_text(kb_yaml)
    (tmp_path / "notebook").mkdir()
    files = {"notebook/index.md": ROOT_INDEX, **files}
    for path, text in files.items():
        (tmp_path / path).parent.mkdir(parents=True, exist_ok=True)
        (tmp_path / path).write_text(text)
    return tmp_path


def _rules(violations) -> list[tuple[str, str]]:
    return sorted((v.file, v.rule) for v in violations)


class TestConfig:
    def test_flow_mappings_and_nested_inline_lists(self) -> None:
        config = parse_subset(
            "generated: { by: agent/1.0, at: 2026-05-28T14:30:00Z }\n"
            "verified: [{ by: human:dan, at: 2026-06-01T00:00:00Z }]\n"
            "sources:\n  - { id: a, resource: 'https://x.test/a?b=1, c' }\n"
        )
        assert config["generated"] == {"by": "agent/1.0", "at": "2026-05-28T14:30:00Z"}
        assert config["verified"] == [{"by": "human:dan", "at": "2026-06-01T00:00:00Z"}]
        assert config["sources"] == [{"id": "a", "resource": "https://x.test/a?b=1, c"}]

    def test_no_okf_block_means_no_bundle(self, tmp_path: Path) -> None:
        assert load_bundle(tmp_path, {}) is None

    def test_bundle_must_be_a_directory(self, tmp_path: Path) -> None:
        with pytest.raises(ConfigError, match="rules.checks.okf.bundle"):
            load_bundle(tmp_path, parse_subset("rules:\n  checks:\n    okf:\n      bundle: nope\n"))

    def test_bundle_dot_is_the_repository(self, tmp_path: Path) -> None:
        bundle = load_bundle(
            tmp_path, parse_subset("rules:\n  checks:\n    okf:\n      bundle: .\n")
        )
        assert bundle is not None and bundle.root == "" and bundle.contains("a/b.md")

    def test_malformed_bundle_config_exits_2(self, tmp_path: Path) -> None:
        _kb(tmp_path, {}, "rules:\n  checks:\n    okf: notebook\n")
        for module in ("kb_linter", "link_validator"):
            proc = subprocess.run(
                [sys.executable, "-m", module, str(tmp_path)], capture_output=True, text=True
            )
            assert proc.returncode == 2, module


class TestOkfignore:
    @pytest.mark.parametrize(
        ("patterns", "path", "ignored"),
        [
            ("templates/\n", "notebook/templates/Daily.md", True),
            ("templates/\n", "notebook/wiki/templates.md", False),
            ("templates\n", "notebook/wiki/templates/x.md", True),  # unanchored: any depth
            ("/templates/\n", "notebook/wiki/templates/x.md", False),  # anchored to the root
            ("*.draft.md\n", "notebook/wiki/a.draft.md", True),
            ("raw/**/*.md\n!raw/keep.md\n", "notebook/raw/keep.md", False),
            ("raw/**/*.md\n", "notebook/raw/a/b.md", True),
            ("# comment\n\n", "notebook/wiki/a.md", False),
            ("t[0-9].md\n", "notebook/t1.md", True),
            (
                "dir/\n!dir/x.md\n",
                "notebook/dir/x.md",
                True,
            ),  # a file under an excluded dir stays out
        ],
    )
    def test_gitignore_rules(self, tmp_path: Path, patterns: str, path: str, ignored: bool) -> None:
        _kb(tmp_path, {"notebook/.okfignore": patterns})
        bundle = load_bundle(tmp_path, parse_subset(KB_YAML))
        assert bundle is not None
        assert bundle.is_ignored(path) is ignored

    def test_parse_ignore_skips_blank_and_comment_lines(self) -> None:
        assert len(parse_ignore("# c\n\n  \nx\n")) == 1


class TestConformance:
    def test_conformant_bundle(self, tmp_path: Path) -> None:
        _kb(
            tmp_path,
            {
                "notebook/records/daily/2026-11-06.md": "---\ntype: Daily Note\n---\n# Day\n",
                "notebook/wiki/index.md": "# Wiki\n\n* [Topic](topic.md) - a topic\n- [Other](other.md)\n",
                "notebook/wiki/topic.md": (
                    "---\ntype: Topic\nstatus: draft\nsources:\n"
                    "  - { id: s, resource: https://x.test }\n---\nA claim.[^s]\n\n[^s]: X\n"
                ),
                "notebook/wiki/other.md": "---\ntype: Topic\nstatus: draft\n---\n## Sources\n- x\n",
                "notebook/log.md": "# Log\n\n## 2026-11-06\n* **Creation**: started\n",
            },
        )
        assert lint(str(tmp_path)).violations == []

    def test_missing_frontmatter_and_type(self, tmp_path: Path) -> None:
        _kb(
            tmp_path,
            {
                "notebook/a.md": "# No frontmatter\n",
                "notebook/b.md": "---\ntitle: B\n---\n",
                "notebook/c.md": "---\ntype: ''\n---\n",
            },
        )
        assert _rules(lint(str(tmp_path)).violations) == [
            ("notebook/a.md", "okf-missing-frontmatter"),
            ("notebook/b.md", "okf-missing-type"),
            ("notebook/c.md", "okf-missing-type"),
        ]

    def test_okfignore_and_hidden_directories_are_not_checked(self, tmp_path: Path) -> None:
        _kb(
            tmp_path,
            {
                "notebook/.okfignore": "templates/\n",
                "notebook/templates/Daily.md": "# {{date}}\n",
                "notebook/.obsidian/x.md": "# settings\n",
            },
        )
        assert lint(str(tmp_path)).violations == []

    def test_files_outside_the_bundle_need_no_type(self, tmp_path: Path) -> None:
        _kb(tmp_path, {"docs/guide.md": "---\nstatus: draft\n---\n# Guide\n"})
        assert lint(str(tmp_path)).violations == []

    def test_index_frontmatter(self, tmp_path: Path) -> None:
        _kb(
            tmp_path,
            {
                "notebook/index.md": '---\nokf_version: "0.2"\ntitle: x\n---\n',
                "notebook/wiki/index.md": "---\ntype: Index\n---\n",
            },
        )
        assert _rules(lint(str(tmp_path)).violations) == [
            ("notebook/index.md", "okf-index-frontmatter"),
            ("notebook/wiki/index.md", "okf-index-frontmatter"),
        ]

    def test_index_body_is_headings_and_entries(self, tmp_path: Path) -> None:
        _kb(
            tmp_path,
            {
                "notebook/wiki/index.md": (
                    "<!-- generated -->\n# Wiki\n\nSome prose.\n\n* [A](a.md) - fine\n```\nx\n```\n"
                )
            },
        )
        violations = lint(str(tmp_path)).violations
        assert _rules(violations) == [("notebook/wiki/index.md", "okf-index-entry")]
        assert "line 4" in violations[0].message

    def test_log_headings_are_dates(self, tmp_path: Path) -> None:
        _kb(tmp_path, {"notebook/log.md": "# Log\n\n## 2026-11-06\n\n## Yesterday\n"})
        assert _rules(lint(str(tmp_path)).violations) == [("notebook/log.md", "okf-log-date")]

    def test_okf_version(self, tmp_path: Path) -> None:
        _kb(tmp_path, {"notebook/index.md": "# Notebook\n"})
        assert _rules(lint(str(tmp_path)).violations) == [("notebook/index.md", "okf-version")]

    def test_no_version_pin_means_no_version_check(self, tmp_path: Path) -> None:
        _kb(
            tmp_path,
            {"notebook/index.md": "# Notebook\n"},
            KB_YAML.replace('      version: "0.2"\n', ""),
        )
        assert lint(str(tmp_path)).violations == []


class TestStructuredProvenance:
    def test_sources_frontmatter_counts_as_provenance(self, tmp_path: Path) -> None:
        _kb(
            tmp_path,
            {
                "notebook/wiki/a.md": (
                    "---\ntype: T\nstatus: draft\nsources:\n  - id: x\n    resource: ../raw/x.md\n---\n"
                ),
                "notebook/wiki/b.md": "---\ntype: T\nstatus: draft\n---\nNo sources.\n",
            },
        )
        assert _rules(lint(str(tmp_path)).violations) == [
            ("notebook/wiki/b.md", "missing-provenance")
        ]

    def test_invalid_sources(self, tmp_path: Path) -> None:
        _kb(
            tmp_path,
            {
                "notebook/wiki/a.md": "---\ntype: T\nstatus: draft\nsources: []\n---\n",
                "notebook/wiki/b.md": "---\ntype: T\nstatus: draft\nsources:\n  - id: x\n---\n",
            },
        )
        assert _rules(lint(str(tmp_path)).violations) == [
            ("notebook/wiki/a.md", "invalid-sources"),
            ("notebook/wiki/b.md", "invalid-sources"),
        ]

    def test_without_a_bundle_sources_still_count(self, tmp_path: Path) -> None:
        (tmp_path / "knowledge-base.yaml").write_text(
            "rules:\n  lifecycle:\n    statuses: [draft]\nsources:\n  canonical:\n    - path: docs/**\n"
        )
        (tmp_path / "docs").mkdir()
        (tmp_path / "docs/a.md").write_text(
            "---\nstatus: draft\nsources:\n  - resource: https://x.test\n---\n"
        )
        assert lint(str(tmp_path)).violations == []

    def test_structured_sources_helper(self) -> None:
        assert structured_sources(None) == (False, None)
        assert structured_sources({"sources": [{"resource": "x"}]}) == (True, None)
        assert structured_sources({"sources": "x"})[1] is not None

    def test_split_frontmatter(self) -> None:
        assert split_frontmatter("---\ntype: A\n---\nbody\n") == ({"type": "A"}, "body\n")
        assert split_frontmatter("# no\n") == (None, "# no\n")
        assert split_frontmatter("---\n---\n")[0] == {}


class TestLinkClasses:
    def _validate(self, tmp_path: Path, files: dict[str, str]):
        _kb(tmp_path, files)
        return validate(str(tmp_path))

    def test_unwritten_note_is_a_notice(self, tmp_path: Path) -> None:
        result = self._validate(
            tmp_path, {"notebook/records/a.md": "[Someone](../wiki/people/Someone%20New.md)\n"}
        )
        assert result.violations == []
        assert [(n.rule, n.resolved) for n in result.notices] == [
            ("unwritten-link", "notebook/wiki/people/Someone New.md")
        ]

    def test_missing_non_note_is_broken(self, tmp_path: Path) -> None:
        result = self._validate(tmp_path, {"notebook/a.md": "![x](img/missing.png)\n"})
        assert _rules(result.violations) == [("notebook/a.md", "broken-link")]

    def test_missing_note_outside_the_bundle_is_broken(self, tmp_path: Path) -> None:
        result = self._validate(tmp_path, {"docs/a.md": "[x](missing.md)\n"})
        assert _rules(result.violations) == [("docs/a.md", "broken-link")]
        assert result.notices == []

    def test_link_escaping_the_bundle_fails(self, tmp_path: Path) -> None:
        result = self._validate(
            tmp_path,
            {"notebook/a.md": "[guide](../docs/guide.md)\n", "docs/guide.md": "# Guide\n"},
        )
        assert _rules(result.violations) == [("notebook/a.md", "escapes-bundle")]

    def test_link_into_the_bundle_from_outside_is_fine(self, tmp_path: Path) -> None:
        result = self._validate(
            tmp_path, {"docs/a.md": "[n](../notebook/wiki/n.md)\n", "notebook/wiki/n.md": "x\n"}
        )
        assert result.violations == [] and result.notices == []

    def test_root_anchored_links_resolve_from_the_bundle_root(self, tmp_path: Path) -> None:
        result = self._validate(
            tmp_path, {"notebook/records/a.md": "[n](/wiki/n.md)\n", "notebook/wiki/n.md": "x\n"}
        )
        assert result.violations == [] and result.notices == []

    def test_misdirected_link_fails(self, tmp_path: Path) -> None:
        result = self._validate(
            tmp_path,
            {
                "notebook/records/meetings/m.md": "[Pat](../wiki/people/Pat%20Lee.md)\n",
                "notebook/wiki/people/Pat Lee.md": "x\n",
            },
        )
        assert _rules(result.violations) == [("notebook/records/meetings/m.md", "misdirected-link")]
        assert "notebook/wiki/people/Pat Lee.md" in result.violations[0].message

    def test_close_name_is_a_likely_typo_notice(self, tmp_path: Path) -> None:
        result = self._validate(
            tmp_path,
            {
                "notebook/wiki/a.md": "[Pat](people/Pat%20Leee.md)\n",
                "notebook/wiki/people/Pat Lee.md": "x\n",
            },
        )
        assert result.violations == []
        assert [n.rule for n in result.notices] == ["likely-typo"]

    def test_wikilinks_fail_in_the_bundle_only(self, tmp_path: Path) -> None:
        result = self._validate(
            tmp_path,
            {
                "notebook/a.md": "See [[Pat Lee]] and `[[code]]`.\n```\n[[fenced]]\n```\n",
                "docs/a.md": "See [[Pat Lee]].\n",
            },
        )
        assert [(v.file, v.rule, v.target) for v in result.violations] == [
            ("notebook/a.md", "wikilink", "[[Pat Lee]]")
        ]

    def test_unencoded_space_is_malformed_in_the_bundle(self, tmp_path: Path) -> None:
        result = self._validate(
            tmp_path,
            {
                "notebook/a.md": "[x](Pat Lee.md) [y](<Pat Lee.md>) [z](Pat%20Lee.md)\n",
                "notebook/Pat Lee.md": "x\n",
            },
        )
        assert [(v.rule, v.target) for v in result.violations] == [("malformed-link", "Pat Lee.md")]

    def test_link_titles_are_not_part_of_the_path(self, tmp_path: Path) -> None:
        result = self._validate(
            tmp_path, {"docs/a.md": '[b](b.md "Title")\n', "docs/b.md": "# B\n"}
        )
        assert result.violations == []

    def test_cli_reports_notices_without_failing(self, tmp_path: Path) -> None:
        _kb(tmp_path, {"notebook/a.md": "[n](n.md)\n"})
        proc = subprocess.run(
            [sys.executable, "-m", "link_validator", str(tmp_path)], capture_output=True, text=True
        )
        output = json.loads(proc.stdout)
        assert proc.returncode == 0
        assert output["summary"]["unwritten"] == 1
        assert output["notices"][0]["rule"] == "unwritten-link"


class TestAnchors:
    def test_github_slug(self) -> None:
        assert github_slug("Done when") == "done-when"
        assert github_slug("Step 1: the `move` command!") == "step-1-the-move-command"
        assert github_slug("[Link](x.md) and **bold**") == "link-and-bold"
        assert github_slug("snake_case name") == "snake_case-name"

    def test_longer_fence_holds_shorter_fence_lines(self, tmp_path: Path) -> None:
        note = tmp_path / "n.md"
        note.write_text("````md\n```\n# Inside\n````\n\n## After\n")
        assert has_anchor(note, "after")
        assert not has_anchor(note, "inside")

    def test_dashed_slug_variant(self, tmp_path: Path) -> None:
        note = tmp_path / "n.md"
        note.write_text("## What's still open\n")
        assert has_anchor(note, "whats-still-open")
        assert has_anchor(note, "what-s-still-open")

    def test_heading_forms(self, tmp_path: Path) -> None:
        note = tmp_path / "n.md"
        note.write_text(
            "---\ntitle: x\n---\n# Top\n\n## Done when\n\n## Done when\n\nSetext\n---\n\n"
            'A paragraph. ^block-1\n\n<a id="custom"></a>\n\n```\n# Not a heading\n```\n'
        )
        for fragment in (
            "top",
            "done-when",
            "done-when-1",
            "Done when",
            "Top#Done when",
            "setext",
            "^block-1",
            "custom",
        ):
            assert has_anchor(note, fragment), fragment
        for fragment in ("not-a-heading", "missing", "^nope", "title-x"):
            assert not has_anchor(note, fragment), fragment

    def test_missing_anchor_fails_in_any_kb(self, tmp_path: Path) -> None:
        (tmp_path / "docs").mkdir()
        (tmp_path / "docs/a.md").write_text(
            "# A\n[ok](b.md#done-when) [obsidian](b.md#Done%20when) [self](#a)\n"
            "[bad](b.md#nope) [badself](#nope)\n"
        )
        (tmp_path / "docs/b.md").write_text("## Done when\n")
        result = validate(str(tmp_path))
        assert [(v.rule, v.target) for v in result.violations] == [
            ("missing-anchor", "b.md#nope"),
            ("missing-anchor", "#nope"),
        ]

    def test_fragments_on_non_markdown_targets_are_not_checked(self, tmp_path: Path) -> None:
        (tmp_path / "docs").mkdir()
        (tmp_path / "docs/a.md").write_text("[pdf](b.pdf#page=2) [dir](./#x)\n")
        (tmp_path / "docs/b.pdf").write_text("x")
        assert validate(str(tmp_path)).violations == []


class TestReviewRegressions:
    """Cases from the 2026-11-06 review of the OKF branch."""

    def test_headings_in_blockquotes_callouts_and_lists(self, tmp_path: Path) -> None:
        note = tmp_path / "n.md"
        note.write_text("> ## Quoted\n\n> [!note]\n> ### In callout\n\n- ## In list\n")
        for fragment in ("quoted", "in-callout", "in-list"):
            assert has_anchor(note, fragment), fragment

    def test_html_entities_and_images_in_headings(self) -> None:
        assert github_slug('<a id="x"></a>Setup') == "setup"
        assert github_slug("Run <code>make</code>") == "run-make"
        assert github_slug("A &amp; B") == "a--b"
        assert github_slug("![logo](l.png) Title") == "title"

    def test_suffixed_slugs_skip_taken_ones(self, tmp_path: Path) -> None:
        note = tmp_path / "n.md"
        note.write_text("# Foo\n# Foo\n# Foo-1\n")
        for fragment in ("foo", "foo-1", "foo-1-1"):
            assert has_anchor(note, fragment), fragment

    def test_angle_destination_with_title_in_bundle(self, tmp_path: Path) -> None:
        _kb(tmp_path, {"notebook/a.md": '[x](<sub/b.md> "t")\n', "notebook/sub/b.md": "x\n"})
        result = validate(str(tmp_path))
        assert result.violations == [] and result.notices == []

    def test_query_links_skip_anchor_checks(self, tmp_path: Path) -> None:
        (tmp_path / "docs").mkdir()
        (tmp_path / "docs/a.md").write_text("[c](b.md?plain=1#L3)\n")
        (tmp_path / "docs/b.md").write_text("x\n")
        assert validate(str(tmp_path)).violations == []

    def test_citation_brackets_are_not_wikilinks(self, tmp_path: Path) -> None:
        _kb(tmp_path, {"notebook/a.md": "Claim [[1]](#ref).\n\n## Ref\n"})
        assert validate(str(tmp_path)).violations == []

    def test_unwritten_common_names_are_not_misdirected(self, tmp_path: Path) -> None:
        _kb(tmp_path, {"notebook/a.md": "[new](projects/new/index.md) [r](x/README.md)\n"})
        result = validate(str(tmp_path))
        assert result.violations == []
        assert [n.rule for n in result.notices] == ["unwritten-link", "unwritten-link"]

    def test_braces_without_pairs_stay_strings(self) -> None:
        config = parse_subset("a: {draft}\ntype: {{type}}\nb:\n  - {a,b}\nc: { by: x }\n")
        assert config["a"] == "{draft}"
        assert config["type"] == "{{type}}"
        assert config["b"] == ["{a,b}"]
        assert config["c"] == {"by": "x"}

    def test_okfignore_double_star_inside_a_segment_is_a_star(self, tmp_path: Path) -> None:
        _kb(tmp_path, {"notebook/.okfignore": "foo**bar\n"})
        bundle = load_bundle(tmp_path, parse_subset(KB_YAML))
        assert bundle is not None
        assert bundle.is_ignored("notebook/fooxbar")
        assert not bundle.is_ignored("notebook/foo/x/bar")

    def test_bundle_path_is_normalized(self, tmp_path: Path) -> None:
        (tmp_path / "notes").mkdir()
        bundle = load_bundle(
            tmp_path, parse_subset("rules:\n  checks:\n    okf:\n      bundle: notes/..\n")
        )
        assert bundle is not None and bundle.root == ""


class TestAnchorOptOut:
    def test_anchors_can_be_turned_off(self, tmp_path: Path) -> None:
        (tmp_path / "docs").mkdir()
        (tmp_path / "docs/a.md").write_text("[b](b.md#nope) [s](#nope)\n")
        (tmp_path / "docs/b.md").write_text("x\n")
        (tmp_path / "knowledge-base.yaml").write_text(
            "rules:\n  checks:\n    links:\n      include: [docs/**]\n      anchors: false\n"
        )
        assert validate(str(tmp_path)).violations == []

    def test_bad_anchors_value_is_a_config_error(self, tmp_path: Path) -> None:
        (tmp_path / "knowledge-base.yaml").write_text(
            "rules:\n  checks:\n    links:\n      anchors: sometimes\n"
        )
        with pytest.raises(ConfigError, match="anchors"):
            validate(str(tmp_path))
