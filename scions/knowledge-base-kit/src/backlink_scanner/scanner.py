# spec: specs/backlink-scanner.md
# spec-section: Behavior/Scanning for backlinks

"""Core scanning logic for finding spec backlink annotations in source files."""

import os
import re
from dataclasses import dataclass, field
from pathlib import Path

from tool_cli.graft import is_graft_dir

SPEC_PATTERN = re.compile(r"^\s*(?://|#)\s*spec:\s*([\w./\-]+)\s*$")
SPEC_SECTION_PATTERN = re.compile(r"^\s*(?://|#)\s*spec-section:\s*(.+?)\s*$")
FENCE_PATTERN = re.compile(r"^\s*```")

BINARY_EXTENSIONS = frozenset(
    {
        ".png",
        ".jpg",
        ".jpeg",
        ".gif",
        ".ico",
        ".pdf",
        ".zip",
        ".tar",
        ".gz",
        ".bin",
        ".exe",
        ".woff",
        ".woff2",
        ".ttf",
        ".eot",
        ".mp3",
        ".mp4",
        ".wav",
        ".avi",
    }
)

SKIP_DIRS = frozenset({".git", ".graft", ".venv", "node_modules", "__pycache__"})


@dataclass
class SpecEntry:
    """A spec with its implementors and section references."""

    implementors: list[str] = field(default_factory=list)
    sections: dict[str, list[str]] = field(default_factory=dict)


@dataclass
class ScanResult:
    """Result of scanning a directory for backlink annotations."""

    specs: dict[str, SpecEntry] = field(default_factory=dict)
    dangling: list[str] = field(default_factory=list)
    orphans: list[str] = field(default_factory=list)


@dataclass
class _FileAnnotations:
    """Annotations found in a single file."""

    spec_paths: list[str] = field(default_factory=list)
    sections: dict[str, list[str]] = field(default_factory=dict)


def _is_binary(path: str) -> bool:
    ext = os.path.splitext(path)[1].lower()
    return ext in BINARY_EXTENSIONS


def _get_files(root: Path) -> list[str]:
    """Recursively collect file paths relative to root, skipping hidden/ignored dirs."""
    files: list[str] = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [
            d
            for d in dirnames
            if d not in SKIP_DIRS and not d.startswith(".") and not is_graft_dir(Path(dirpath, d))
        ]
        for filename in filenames:
            full_path = os.path.join(dirpath, filename)
            rel_path = os.path.relpath(full_path, root)
            files.append(rel_path)
    return files


def _scan_file(root: Path, file: str) -> _FileAnnotations:
    """Extract spec paths and section references from annotations in a file."""
    if _is_binary(file):
        return _FileAnnotations()

    full_path = root / file
    try:
        content = full_path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return _FileAnnotations()

    is_markdown = file.endswith(".md")
    in_code_fence = False
    annotations = _FileAnnotations()
    current_spec: str | None = None

    for line in content.splitlines():
        if is_markdown and FENCE_PATTERN.match(line):
            in_code_fence = not in_code_fence
            continue
        if in_code_fence:
            continue

        spec_match = SPEC_PATTERN.match(line)
        if spec_match:
            spec_path = spec_match.group(1)
            annotations.spec_paths.append(spec_path)
            current_spec = spec_path
            continue

        section_match = SPEC_SECTION_PATTERN.match(line)
        if section_match and current_spec is not None:
            section_name = section_match.group(1)
            if current_spec not in annotations.sections:
                annotations.sections[current_spec] = []
            annotations.sections[current_spec].append(section_name)

    return annotations


def scan(root_dir: str) -> ScanResult:
    """Scan a directory for spec backlink annotations.

    Finds files containing annotations like `// spec: path/to/spec.md`
    or `# spec: path/to/spec.md` and builds a mapping of specs to implementors.

    Args:
        root_dir: The directory to scan.

    Returns:
        ScanResult with specs, dangling references, and orphan specs.
    """
    root = Path(root_dir).resolve()
    files = _get_files(root)
    spec_implementors: dict[str, set[str]] = {}
    spec_sections: dict[str, dict[str, set[str]]] = {}

    for file in files:
        annotations = _scan_file(root, file)
        for spec_path in annotations.spec_paths:
            if spec_path not in spec_implementors:
                spec_implementors[spec_path] = set()
            spec_implementors[spec_path].add(file)

        for spec_path, section_names in annotations.sections.items():
            if spec_path not in spec_sections:
                spec_sections[spec_path] = {}
            for section_name in section_names:
                if section_name not in spec_sections[spec_path]:
                    spec_sections[spec_path][section_name] = set()
                spec_sections[spec_path][section_name].add(file)

    # Find spec files in the specs/ directory (excluding README/index navigation files)
    spec_files = [
        f
        for f in files
        if f.startswith("specs/")
        and f.endswith(".md")
        and not f.endswith("/README.md")
        and f != "specs/README.md"
    ]

    # Identify dangling references (referenced specs that don't exist)
    dangling = [sp for sp in spec_implementors if not (root / sp).exists()]

    # Identify orphan specs (spec files with no references)
    orphans = [f for f in spec_files if f not in spec_implementors]

    # Build output with sorted implementors and sections
    specs: dict[str, SpecEntry] = {}
    for sp, implementors in spec_implementors.items():
        sections: dict[str, list[str]] = {}
        if sp in spec_sections:
            sections = {name: sorted(files) for name, files in spec_sections[sp].items()}
        specs[sp] = SpecEntry(implementors=sorted(implementors), sections=sections)

    return ScanResult(specs=specs, dangling=dangling, orphans=orphans)
