# spec: specs/backlink-scanner.md

"""Tests for the backlink scanner."""

from pathlib import Path

from backlink_scanner.scanner import scan


class TestScanningForBacklinks:
    def test_finds_double_slash_annotations(self, tmp_path: Path) -> None:
        (tmp_path / "specs").mkdir()
        (tmp_path / "specs/auth.md").write_text("# Auth")
        (tmp_path / "src.py").write_text("# spec: specs/auth.md\nx = 1")

        result = scan(str(tmp_path))

        assert result.specs["specs/auth.md"].implementors == ["src.py"]

    def test_finds_hash_annotations(self, tmp_path: Path) -> None:
        (tmp_path / "specs").mkdir()
        (tmp_path / "specs/auth.md").write_text("# Auth")
        (tmp_path / "script.py").write_text("# spec: specs/auth.md\nx = 1")

        result = scan(str(tmp_path))

        assert result.specs["specs/auth.md"].implementors == ["script.py"]

    def test_finds_slash_slash_annotations(self, tmp_path: Path) -> None:
        (tmp_path / "specs").mkdir()
        (tmp_path / "specs/auth.md").write_text("# Auth")
        (tmp_path / "src.ts").write_text("// spec: specs/auth.md\nconst x = 1;")

        result = scan(str(tmp_path))

        assert result.specs["specs/auth.md"].implementors == ["src.ts"]

    def test_records_multiple_implementors(self, tmp_path: Path) -> None:
        (tmp_path / "specs").mkdir()
        (tmp_path / "specs/auth.md").write_text("# Auth")
        (tmp_path / "a.py").write_text("# spec: specs/auth.md")
        (tmp_path / "b.py").write_text("# spec: specs/auth.md")

        result = scan(str(tmp_path))

        assert result.specs["specs/auth.md"].implementors == ["a.py", "b.py"]

    def test_records_multiple_specs_from_one_file(self, tmp_path: Path) -> None:
        (tmp_path / "specs").mkdir()
        (tmp_path / "specs/auth.md").write_text("# Auth")
        (tmp_path / "specs/rate.md").write_text("# Rate")
        (tmp_path / "src.py").write_text("# spec: specs/auth.md\n# spec: specs/rate.md")

        result = scan(str(tmp_path))

        assert result.specs["specs/auth.md"].implementors == ["src.py"]
        assert result.specs["specs/rate.md"].implementors == ["src.py"]


class TestSpecSections:
    def test_records_section_with_preceding_spec(self, tmp_path: Path) -> None:
        (tmp_path / "specs").mkdir()
        (tmp_path / "specs/auth.md").write_text("# Auth")
        (tmp_path / "src.py").write_text(
            "# spec: specs/auth.md\n# spec-section: Behavior/Login\ndef login(): pass"
        )

        result = scan(str(tmp_path))

        assert result.specs["specs/auth.md"].sections == {"Behavior/Login": ["src.py"]}

    def test_ignores_section_without_preceding_spec(self, tmp_path: Path) -> None:
        (tmp_path / "src.py").write_text("# spec-section: Behavior/Login\ndef login(): pass")

        result = scan(str(tmp_path))

        assert result.specs == {}

    def test_multiple_sections_per_spec_in_one_file(self, tmp_path: Path) -> None:
        (tmp_path / "specs").mkdir()
        (tmp_path / "specs/auth.md").write_text("# Auth")
        (tmp_path / "src.py").write_text(
            "# spec: specs/auth.md\n"
            "# spec-section: Behavior/Login\n"
            "# spec-section: Behavior/Logout\n"
        )

        result = scan(str(tmp_path))

        assert result.specs["specs/auth.md"].sections == {
            "Behavior/Login": ["src.py"],
            "Behavior/Logout": ["src.py"],
        }

    def test_sections_from_multiple_files(self, tmp_path: Path) -> None:
        (tmp_path / "specs").mkdir()
        (tmp_path / "specs/auth.md").write_text("# Auth")
        (tmp_path / "a.py").write_text("# spec: specs/auth.md\n# spec-section: Behavior/Login\n")
        (tmp_path / "b.py").write_text("# spec: specs/auth.md\n# spec-section: Behavior/Login\n")

        result = scan(str(tmp_path))

        assert result.specs["specs/auth.md"].sections == {"Behavior/Login": ["a.py", "b.py"]}

    def test_section_applies_to_most_recent_spec(self, tmp_path: Path) -> None:
        (tmp_path / "specs").mkdir()
        (tmp_path / "specs/auth.md").write_text("# Auth")
        (tmp_path / "specs/rate.md").write_text("# Rate")
        (tmp_path / "src.py").write_text(
            "# spec: specs/auth.md\n"
            "# spec-section: Behavior/Login\n"
            "# spec: specs/rate.md\n"
            "# spec-section: Behavior/Limiting\n"
        )

        result = scan(str(tmp_path))

        assert result.specs["specs/auth.md"].sections == {"Behavior/Login": ["src.py"]}
        assert result.specs["specs/rate.md"].sections == {"Behavior/Limiting": ["src.py"]}

    def test_slash_slash_section_annotations(self, tmp_path: Path) -> None:
        (tmp_path / "specs").mkdir()
        (tmp_path / "specs/auth.md").write_text("# Auth")
        (tmp_path / "src.ts").write_text(
            "// spec: specs/auth.md\n// spec-section: Behavior/Login\n"
        )

        result = scan(str(tmp_path))

        assert result.specs["specs/auth.md"].sections == {"Behavior/Login": ["src.ts"]}


class TestDanglingReferences:
    def test_reports_nonexistent_specs(self, tmp_path: Path) -> None:
        (tmp_path / "src.py").write_text("# spec: specs/nonexistent.md")

        result = scan(str(tmp_path))

        assert "specs/nonexistent.md" in result.dangling


class TestOrphanSpecs:
    def test_reports_unreferenced_specs(self, tmp_path: Path) -> None:
        (tmp_path / "specs").mkdir()
        (tmp_path / "specs/orphan.md").write_text("# Orphan")
        (tmp_path / "src.py").write_text("x = 1")

        result = scan(str(tmp_path))

        assert "specs/orphan.md" in result.orphans

    def test_excludes_readme_from_orphan_detection(self, tmp_path: Path) -> None:
        (tmp_path / "specs").mkdir()
        (tmp_path / "specs/README.md").write_text("# Specs Index")
        (tmp_path / "specs/real-spec.md").write_text("# Real Spec")
        (tmp_path / "src.py").write_text("# spec: specs/real-spec.md")

        result = scan(str(tmp_path))

        assert "specs/README.md" not in result.orphans
        assert result.orphans == []

    def test_does_not_report_referenced_specs(self, tmp_path: Path) -> None:
        (tmp_path / "specs").mkdir()
        (tmp_path / "specs/used.md").write_text("# Used")
        (tmp_path / "src.py").write_text("# spec: specs/used.md")

        result = scan(str(tmp_path))

        assert result.orphans == []


class TestEdgeCases:
    def test_skips_binary_files(self, tmp_path: Path) -> None:
        (tmp_path / "specs").mkdir()
        (tmp_path / "specs/auth.md").write_text("# Auth")
        (tmp_path / "image.png").write_text("# spec: specs/auth.md")

        result = scan(str(tmp_path))

        assert result.specs == {}

    def test_skips_hidden_dirs(self, tmp_path: Path) -> None:
        (tmp_path / ".git").mkdir()
        (tmp_path / ".git/config").write_text("# spec: specs/auth.md")

        result = scan(str(tmp_path))

        assert result.specs == {}

    def test_skips_ignored_dirs(self, tmp_path: Path) -> None:
        (tmp_path / "node_modules/dep").mkdir(parents=True)
        (tmp_path / "node_modules/dep/index.js").write_text("// spec: specs/auth.md")
        (tmp_path / "__pycache__").mkdir()
        (tmp_path / "__pycache__/mod.pyc").write_text("# spec: specs/auth.md")

        result = scan(str(tmp_path))

        assert result.specs == {}

    def test_handles_empty_directories(self, tmp_path: Path) -> None:
        (tmp_path / "empty").mkdir()

        result = scan(str(tmp_path))

        assert result.specs == {}
        assert result.dangling == []
        assert result.orphans == []

    def test_skips_annotations_inside_code_fences(self, tmp_path: Path) -> None:
        (tmp_path / "specs").mkdir()
        (tmp_path / "specs/auth.md").write_text("# Auth")
        (tmp_path / "example.md").write_text(
            "# Example\n```python\n# spec: specs/auth.md\ndef login():\n    pass\n```\n"
        )

        result = scan(str(tmp_path))

        assert result.specs == {}

    def test_matches_annotations_outside_code_fences(self, tmp_path: Path) -> None:
        (tmp_path / "specs").mkdir()
        (tmp_path / "specs/auth.md").write_text("# Auth")
        (tmp_path / "example.md").write_text(
            "# spec: specs/auth.md\n\n```python\n# not matched\n```\n"
        )

        result = scan(str(tmp_path))

        assert result.specs["specs/auth.md"].implementors == ["example.md"]

    def test_ignores_embedded_annotations(self, tmp_path: Path) -> None:
        (tmp_path / "specs").mkdir()
        (tmp_path / "specs/auth.md").write_text("# Auth")
        (tmp_path / "src.py").write_text('write_file("x", "# spec: specs/auth.md")')

        result = scan(str(tmp_path))

        assert result.specs == {}

    def test_returns_sorted_implementors(self, tmp_path: Path) -> None:
        (tmp_path / "specs").mkdir()
        (tmp_path / "specs/auth.md").write_text("# Auth")
        (tmp_path / "z.py").write_text("# spec: specs/auth.md")
        (tmp_path / "a.py").write_text("# spec: specs/auth.md")
        (tmp_path / "m.py").write_text("# spec: specs/auth.md")

        result = scan(str(tmp_path))

        assert result.specs["specs/auth.md"].implementors == ["a.py", "m.py", "z.py"]
