# spec: specs/tool-cli.md
# spec-section: Path scopes

"""Tests for glob matching and scope-driven file collection."""

from pathlib import Path

from tool_cli.scope import collect, matches


class TestMatches:
    def test_double_star_suffix_matches_beneath_dir(self) -> None:
        assert matches("docs/a.md", "docs/**")
        assert matches("docs/sub/dir/a.md", "docs/**")
        assert not matches("docs", "docs/**")
        assert not matches("docsx/a.md", "docs/**")

    def test_double_star_prefix_matches_any_depth_including_root(self) -> None:
        assert matches("x.md", "**/x.md")
        assert matches("a/b/x.md", "**/x.md")
        assert not matches("a/b/y.md", "**/x.md")

    def test_single_star_stays_within_a_segment(self) -> None:
        assert matches("docs/a.md", "docs/*.md")
        assert not matches("docs/sub/a.md", "docs/*.md")

    def test_bare_name_matches_only_the_root_file(self) -> None:
        assert matches("README.md", "README.md")
        assert not matches("docs/README.md", "README.md")

    def test_question_mark_and_regex_characters(self) -> None:
        assert matches("a1.md", "a?.md")
        assert not matches("a/.md", "a?.md")
        assert not matches("axmd", "a.md")


def _tree(root: Path, files: list[str]) -> None:
    for f in files:
        path = root / f
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("# x\n")


class TestCollect:
    def test_include_and_exclude(self, tmp_path: Path) -> None:
        _tree(tmp_path, ["README.md", "docs/a.md", "notebook/daily/d.md", "notebook/wiki/w.md"])

        found = collect(tmp_path, ["README.md", "docs/**", "notebook/**"], ["notebook/daily/**"])

        assert found == ["README.md", "docs/a.md", "notebook/wiki/w.md"]

    def test_only_markdown_files(self, tmp_path: Path) -> None:
        _tree(tmp_path, ["docs/a.md", "docs/b.py", "docs/c.txt"])

        assert collect(tmp_path, ["docs/**"]) == ["docs/a.md"]

    def test_hidden_dirs_walked_only_when_an_include_reaches_them(self, tmp_path: Path) -> None:
        _tree(tmp_path, [".pi/skills/s/SKILL.md", ".obsidian/x.md", "docs/a.md"])

        assert collect(tmp_path, ["docs/**"]) == ["docs/a.md"]
        assert collect(tmp_path, ["docs/**", ".pi/skills/**"]) == [
            ".pi/skills/s/SKILL.md",
            "docs/a.md",
        ]
        assert ".obsidian/x.md" in collect(tmp_path, ["**/*.md"])

    def test_skips_standard_and_graft_dirs(self, tmp_path: Path) -> None:
        _tree(tmp_path, ["docs/a.md", "node_modules/m.md", ".git/g.md", ".scions/dep/d.md"])
        (tmp_path / ".scions" / "README.md").write_text(
            "<!-- managed by graft: https://github.com/danielnaab/graft -->\n"
        )

        assert collect(tmp_path, ["**/*.md"]) == ["docs/a.md"]

    def test_pattern_shorter_than_tree_does_not_descend(self, tmp_path: Path) -> None:
        _tree(tmp_path, ["docs/a.md", "docs/sub/b.md"])

        assert collect(tmp_path, ["docs/*.md"]) == ["docs/a.md"]
