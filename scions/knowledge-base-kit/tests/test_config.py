# spec: specs/tool-cli.md
# spec-section: Configuration

"""Tests for the shared knowledge-base.yaml subset reader."""

from pathlib import Path

import pytest

from tool_cli.config import ConfigError, get_list, get_value, load_config, parse_subset


class TestParseSubset:
    def test_nested_mappings_and_scalars(self) -> None:
        config = parse_subset("a:\n  b:\n    c: value\n  d: 'quoted'\n")

        assert get_value(config, "a.b.c") == "value"
        assert get_value(config, "a.d") == "quoted"

    def test_inline_list_bare_items(self) -> None:
        config = parse_subset("rules:\n  lifecycle:\n    statuses: [draft, working, stable]\n")

        assert get_list(config, "rules.lifecycle.statuses") == ["draft", "working", "stable"]

    def test_inline_list_quoted_items(self) -> None:
        config = parse_subset('statuses: ["draft", \'working\']\n')

        assert get_list(config, "statuses") == ["draft", "working"]

    def test_inline_list_keeps_commas_inside_quotes(self) -> None:
        config = parse_subset('paths: ["a, b/**", c]\n')

        assert get_list(config, "paths") == ["a, b/**", "c"]

    def test_block_list_of_scalars(self) -> None:
        config = parse_subset("include:\n  - README.md\n  - docs/**\n")

        assert get_list(config, "include") == ["README.md", "docs/**"]

    def test_block_list_at_same_indent_as_key(self) -> None:
        config = parse_subset("include:\n- README.md\n- docs/**\nother: x\n")

        assert get_list(config, "include") == ["README.md", "docs/**"]
        assert get_value(config, "other") == "x"

    def test_list_of_mappings(self) -> None:
        config = parse_subset(
            "sources:\n  canonical:\n    - path: docs/**\n      note: \"Docs\"\n    - path: x.md\n"
        )

        assert get_value(config, "sources.canonical") == [
            {"path": "docs/**", "note": "Docs"},
            {"path": "x.md"},
        ]

    def test_comments_are_ignored_outside_quotes(self) -> None:
        config = parse_subset("# top\na: b # trailing\nc: 'x # kept'\n\n  # indented comment\n")

        assert get_value(config, "a") == "b"
        assert get_value(config, "c") == "x # kept"

    def test_paths_with_glob_and_dot_characters(self) -> None:
        config = parse_subset("include:\n  - .pi/skills/**\n  - '**/README.md'\n  - https://x.test/a\n")

        assert get_list(config, "include") == [".pi/skills/**", "**/README.md", "https://x.test/a"]

    def test_unsupported_constructs_are_ignored(self) -> None:
        config = parse_subset(
            "note: |\n  free text: with colon\n  more\nflow: {a: 1}\nrules:\n  x: [a]\n"
        )

        assert get_list(config, "rules.x") == ["a"]

    def test_empty_and_non_mapping_documents(self) -> None:
        assert parse_subset("") == {}
        assert parse_subset("- a\n- b\n") == {}


class TestGetList:
    def test_absent_key_is_none(self) -> None:
        assert get_list({}, "rules.checks.links.include") is None

    def test_lone_string_is_one_item_list(self) -> None:
        assert get_list(parse_subset("include: docs/**\n"), "include") == ["docs/**"]

    def test_mapping_where_list_expected_raises(self) -> None:
        with pytest.raises(ConfigError, match="rules.lifecycle.statuses"):
            get_list(parse_subset("rules:\n  lifecycle:\n    statuses:\n      a: b\n"), "rules.lifecycle.statuses")

    def test_list_of_mappings_where_strings_expected_raises(self) -> None:
        with pytest.raises(ConfigError):
            get_list(parse_subset("include:\n  - path: x\n"), "include")


class TestLoadConfig:
    def test_missing_file_is_empty_unless_required(self, tmp_path: Path) -> None:
        assert load_config(tmp_path) == {}
        with pytest.raises(FileNotFoundError):
            load_config(tmp_path, required=True)

    def test_reads_file(self, tmp_path: Path) -> None:
        (tmp_path / "knowledge-base.yaml").write_text("name: kb\n")

        assert load_config(tmp_path) == {"name": "kb"}
