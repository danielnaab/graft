# spec: specs/link-validator.md

"""Tests for the link validator."""

from pathlib import Path

from link_validator.validator import _extract_links, _resolve_path, validate


def _setup_kb(tmp_path: Path) -> None:
    """Create a minimal KB structure for testing."""
    (tmp_path / "docs").mkdir()
    (tmp_path / "policies").mkdir()
    (tmp_path / "playbooks").mkdir()
    (tmp_path / "notes").mkdir()
    (tmp_path / "specs").mkdir()


class TestLinkExtraction:
    def test_extracts_basic_link(self) -> None:
        content = "See [format](format.md) for details."
        assert _extract_links(content) == ["format.md"]

    def test_extracts_multiple_links(self) -> None:
        content = "See [a](a.md) and [b](b.md)."
        assert _extract_links(content) == ["a.md", "b.md"]

    def test_extracts_image_links(self) -> None:
        content = "![diagram](images/arch.png)"
        assert _extract_links(content) == ["images/arch.png"]

    def test_skips_links_in_code_blocks(self) -> None:
        content = "Before\n```markdown\n[example](fake.md)\n```\nAfter [real](real.md)"
        assert _extract_links(content) == ["real.md"]

    def test_skips_links_in_tilde_fences(self) -> None:
        content = "Before\n~~~\n[example](fake.md)\n~~~\nAfter [real](real.md)"
        assert _extract_links(content) == ["real.md"]

    def test_handles_nested_code_fences(self) -> None:
        content = "````\n```\n[inner](x.md)\n```\n````\n[outer](y.md)"
        assert _extract_links(content) == ["y.md"]

    def test_skips_empty_targets(self) -> None:
        content = "[empty]() and [real](real.md)"
        assert _extract_links(content) == ["real.md"]

    def test_extracts_links_with_fragments(self) -> None:
        content = "[section](file.md#heading)"
        assert _extract_links(content) == ["file.md#heading"]

    def test_skips_links_in_inline_code(self) -> None:
        content = "Use `[text](target)` syntax for links. See [real](real.md)."
        assert _extract_links(content) == ["real.md"]

    def test_skips_links_in_double_backtick_code(self) -> None:
        content = "Example: ``[text](fake.md)`` and [real](real.md)"
        assert _extract_links(content) == ["real.md"]

    def test_extracts_link_whose_text_is_inline_code(self) -> None:
        content = "[`notecap_gate.py`](../../.claude/hooks/notecap_gate.py)"
        assert _extract_links(content) == ["../../.claude/hooks/notecap_gate.py"]

    def test_extracts_link_with_inline_code_in_text(self) -> None:
        assert _extract_links("[see `x` here](a.md) and `[no](b.md)`") == ["a.md"]

    def test_still_skips_link_inside_code_span_next_to_link(self) -> None:
        content = "`[x](y.md)` then [`z`](z.md)"
        assert _extract_links(content) == ["z.md"]

    def test_extracts_relative_parent_links(self) -> None:
        content = "[policy](../policies/rules.md)"
        assert _extract_links(content) == ["../policies/rules.md"]


class TestPathResolution:
    def test_resolves_sibling_link(self) -> None:
        result = _resolve_path("format.md", "docs")
        assert result == "docs/format.md"

    def test_resolves_parent_traversal(self) -> None:
        result = _resolve_path("../policies/rules.md", "docs")
        assert result == "policies/rules.md"

    def test_strips_fragment(self) -> None:
        result = _resolve_path("file.md#section", "docs")
        assert result == "docs/file.md"

    def test_strips_query_string(self) -> None:
        result = _resolve_path("file.md?raw=true", "docs")
        assert result == "docs/file.md"

    def test_returns_none_for_external_http(self) -> None:
        result = _resolve_path("https://example.com", "docs")
        assert result is None

    def test_returns_none_for_external_mailto(self) -> None:
        result = _resolve_path("mailto:user@example.com", "docs")
        assert result is None

    def test_returns_none_for_any_uri_scheme(self) -> None:
        for target in (
            "airmail://message?mail=me%40example.com&messageid=abc",
            "tel:+15555550100",
            "file:///Users/daniel/Documents/report.pdf",
            "data:image/png;base64,iVBORw0KGgo=",
            "app://obsidian.md/note",
            "obsidian://open?vault=notes",
            "x-devonthink-item://1234",
            "HTTPS://EXAMPLE.COM",
        ):
            assert _resolve_path(target, "docs") is None, target

    def test_colon_after_path_separator_is_not_a_scheme(self) -> None:
        assert _resolve_path("sub/a:b.md", "docs") == "docs/sub/a:b.md"
        assert _resolve_path("./notes:foo.md", "docs") == "docs/notes:foo.md"
        assert _resolve_path("file.md#section:one", "docs") == "docs/file.md"
        assert _resolve_path("file.md?x=a:b", "docs") == "docs/file.md"

    def test_scheme_must_start_with_letter(self) -> None:
        assert _resolve_path("1abc:foo.md", "docs") == "docs/1abc:foo.md"

    def test_decodes_percent_encoding(self) -> None:
        result = _resolve_path("../transcripts/2026-10-01%20-%20Meeting%2C%20Notes.md", "notes/sub")
        assert result == "notes/transcripts/2026-10-01 - Meeting, Notes.md"

    def test_decodes_after_stripping_fragment(self) -> None:
        assert _resolve_path("a%20b.md%23not-a-fragment#real", "docs") == "docs/a b.md#not-a-fragment"

    def test_strips_angle_brackets(self) -> None:
        assert _resolve_path("<Name with spaces.md>", "notes") == "notes/Name with spaces.md"
        assert _resolve_path("<Name with spaces.md#Heading>", "notes") == "notes/Name with spaces.md"

    def test_angle_bracketed_scheme_is_external(self) -> None:
        assert _resolve_path("<https://example.com/a b>", "docs") is None

    def test_returns_none_for_empty_after_strip(self) -> None:
        result = _resolve_path("#fragment-only", "docs")
        assert result is None

    def test_resolves_absolute_path_from_root(self) -> None:
        result = _resolve_path("/.graft/python-starter/", "docs/sub")
        assert result == ".graft/python-starter"

    def test_detects_path_escape(self) -> None:
        result = _resolve_path("../../../etc/passwd", "docs")
        assert result is not None
        assert result.startswith("..")


class TestValidation:
    def test_no_violations_for_valid_links(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/target.md").write_text("# Target\n")
        (tmp_path / "docs/source.md").write_text("See [target](target.md).\n")

        result = validate(str(tmp_path))

        assert result.violations == []
        assert result.links_checked == 1

    def test_reports_broken_link(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/source.md").write_text("See [missing](nonexistent.md).\n")

        result = validate(str(tmp_path))

        assert len(result.violations) == 1
        assert result.violations[0].file == "docs/source.md"
        assert result.violations[0].target == "nonexistent.md"
        assert result.violations[0].resolved == "docs/nonexistent.md"
        assert result.violations[0].rule == "broken-link"

    def test_valid_cross_directory_link(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "policies/rules.md").write_text("# Rules\n")
        (tmp_path / "docs/source.md").write_text("See [rules](../policies/rules.md).\n")

        result = validate(str(tmp_path))

        assert result.violations == []

    def test_broken_cross_directory_link(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/source.md").write_text("See [gone](../policies/gone.md).\n")

        result = validate(str(tmp_path))

        assert len(result.violations) == 1
        assert result.violations[0].resolved == "policies/gone.md"

    def test_valid_directory_link(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/source.md").write_text("See [specs](../specs/).\n")

        result = validate(str(tmp_path))

        assert result.violations == []

    def test_skips_external_links(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/source.md").write_text(
            "See [ext](https://example.com) and [mail](mailto:a@b.com).\n"
        )

        result = validate(str(tmp_path))

        assert result.links_checked == 0
        assert result.violations == []

    def test_skips_links_with_any_uri_scheme(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "notes/license.md").write_text(
            "[email](airmail://message?mail=me%40example.com&messageid=abc)\n"
            "[call](tel:+15555550100)\n"
            "[pdf](file:///Users/daniel/Documents/report.pdf)\n"
            "![dot](data:image/png;base64,iVBORw0KGgo=)\n"
            "[app](app://some-app/path)\n"
        )

        result = validate(str(tmp_path))

        assert result.links_checked == 0
        assert result.violations == []

    def test_relative_links_still_checked_alongside_schemes(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "notes/target.md").write_text("# Target\n\n## Section\n")
        (tmp_path / "notes/source.md").write_text(
            "## Local anchor\n"
            "[email](airmail://message?mail=x)\n"
            "[ok](target.md)\n"
            "[anchor](target.md#section)\n"
            "[self](#local-anchor)\n"
            "[gone](missing.md)\n"
        )

        result = validate(str(tmp_path))

        assert result.links_checked == 4
        assert len(result.violations) == 1
        assert result.violations[0].target == "missing.md"

    def test_percent_encoded_link_to_existing_file(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "notes/transcripts").mkdir()
        (tmp_path / "notes/transcripts/2026-10-01 - Meeting, Notes.md").write_text("# T\n")
        (tmp_path / "notes/sub").mkdir()
        (tmp_path / "notes/sub/source.md").write_text(
            "[t](../transcripts/2026-10-01%20-%20Meeting%2C%20Notes.md)\n"
            "[gone](../transcripts/Missing%20File.md)\n"
        )

        result = validate(str(tmp_path))

        assert result.links_checked == 2
        assert len(result.violations) == 1
        assert result.violations[0].target == "../transcripts/Missing%20File.md"
        assert result.violations[0].resolved == "notes/transcripts/Missing File.md"

    def test_angle_bracket_link_to_existing_file(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "notes/Name with spaces.md").write_text("# N\n")
        (tmp_path / "notes/source.md").write_text(
            "[n](<Name with spaces.md>)\n[gone](<Not here.md>)\n"
        )

        result = validate(str(tmp_path))

        assert result.links_checked == 2
        assert len(result.violations) == 1
        assert result.violations[0].target == "<Not here.md>"

    def test_reports_broken_link_with_inline_code_text(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/source.md").write_text(
            "[`gate.py`](../hooks/gate.py) but not `[x](missing.md)`\n"
        )

        result = validate(str(tmp_path))

        assert result.links_checked == 1
        assert len(result.violations) == 1
        assert result.violations[0].target == "../hooks/gate.py"

    def test_strips_fragment_before_checking(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "policies/rules.md").write_text("# Rules\n\n## Section\n")
        (tmp_path / "docs/source.md").write_text("See [rules](../policies/rules.md#section).\n")

        result = validate(str(tmp_path))

        assert result.violations == []

    def test_reports_path_escape_as_broken(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/source.md").write_text("See [escape](../../../etc/passwd).\n")

        result = validate(str(tmp_path))

        assert len(result.violations) == 1

    def test_checks_image_links(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/source.md").write_text("![missing](images/gone.png)\n")

        result = validate(str(tmp_path))

        assert len(result.violations) == 1
        assert result.violations[0].target == "images/gone.png"

    def test_multiple_broken_links_same_target(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/a.md").write_text("[x](missing.md)\n")
        (tmp_path / "docs/b.md").write_text("[y](missing.md)\n")

        result = validate(str(tmp_path))

        assert len(result.violations) == 2

    def test_scans_notes_directory(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "notes/session.md").write_text("[broken](../gone.md)\n")

        result = validate(str(tmp_path))

        assert result.files_checked >= 1
        assert any(v.file == "notes/session.md" for v in result.violations)

    def test_scans_specs_directory(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "specs/tool.md").write_text("[broken](../missing.md)\n")

        result = validate(str(tmp_path))

        assert any(v.file == "specs/tool.md" for v in result.violations)

    def test_skips_code_block_links(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/source.md").write_text(
            "Real [link](../specs/).\n\n```markdown\n[example](fake.md)\n```\n"
        )

        result = validate(str(tmp_path))

        assert result.links_checked == 1
        assert result.violations == []

    def test_counts_files_and_links(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/a.md").write_text("[b](b.md)\n")
        (tmp_path / "docs/b.md").write_text("[a](a.md)\n")

        result = validate(str(tmp_path))

        assert result.files_checked == 2
        assert result.links_checked == 2
        assert result.violations == []

    def test_handles_subdirectories(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / "docs/sub").mkdir()
        (tmp_path / "docs/sub/nested.md").write_text("[up](../other.md)\n")

        result = validate(str(tmp_path))

        assert len(result.violations) == 1
        assert result.violations[0].file == "docs/sub/nested.md"

    def test_absolute_path_resolves_from_root(self, tmp_path: Path) -> None:
        _setup_kb(tmp_path)
        (tmp_path / ".graft").mkdir()
        (tmp_path / ".graft/starter").mkdir()
        # Note: .graft is in SKIP_DIRS for scanning, but the target still exists
        (tmp_path / "docs/source.md").write_text("[starter](/.graft/starter/)\n")

        result = validate(str(tmp_path))

        assert result.violations == []


def test_links_in_html_comments_are_skipped():
    content = "\n".join([
        "[real](real.md) <!-- [inline](gone.md) --> [after](after.md)",
        "<!--",
        "- [example](example.md)",
        "```",
        "-->",
        "[visible](visible.md)",
        "<!-- one -->[x](x.md)<!-- two",
        "[hidden](hidden.md) -->",
    ])
    assert _extract_links(content) == ["real.md", "after.md", "visible.md", "x.md"]


def test_comment_markers_inside_code_spans_do_not_open_a_comment():
    content = "Write `<!--` to start a comment. [still](checked.md)"
    assert _extract_links(content) == ["checked.md"]


def test_commented_out_links_are_not_reported(tmp_path: Path):
    (tmp_path / "notes").mkdir()
    (tmp_path / "notes" / "README.md").write_text(
        "# Notes\n\n<!--\n- [example](session-example.md)\n-->\n[missing](missing.md)\n"
    )
    result = validate(str(tmp_path))
    assert [v.target for v in result.violations] == ["missing.md"]
    assert result.links_checked == 1
