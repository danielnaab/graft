"""The OKF bundle a knowledge base declares: where it is, what .okfignore excludes, and frontmatter."""
# spec: specs/tool-cli.md
# spec-section: OKF bundle

from __future__ import annotations

import posixpath
import re
from dataclasses import dataclass, field
from functools import cache
from pathlib import Path
from typing import Any

from tool_cli import scope
from tool_cli.config import CONFIG_FILE, ConfigError, get_value, parse_subset

IGNORE_FILE = ".okfignore"
RESERVED_NAMES = ("index.md", "log.md")

FRONTMATTER_PATTERN = re.compile(
    r"\A---[ \t]*\r?\n(.*?)^---[ \t]*\r?$\n?", re.DOTALL | re.MULTILINE
)


@dataclass
class Bundle:
    """An OKF bundle inside the repository, from rules.checks.okf."""

    root: str  # repository-relative POSIX path; "" when the repository root is the bundle
    version: str | None = None
    ignore: list[tuple[re.Pattern[str], bool, bool]] = field(default_factory=list)

    def contains(self, path: str) -> bool:
        """True if a repository-relative path is inside the bundle (the root itself included)."""
        if not self.root:
            return not path.startswith("..")
        return path == self.root or path.startswith(self.root + "/")

    def relative(self, path: str) -> str:
        """A repository-relative path inside the bundle, relative to the bundle root."""
        return path[len(self.root) + 1 :] if self.root else path

    def all_markdown(self, root: Path) -> list[str]:
        """Every .md file in the bundle outside hidden directories (.obsidian/, .trash/)."""
        found = scope.collect(root, [f"{self.root}/**" if self.root else "**"])
        return [
            f for f in found if not any(p.startswith(".") for p in self.relative(f).split("/")[:-1])
        ]

    def notes(self, root: Path) -> list[str]:
        """The bundle's notes: all_markdown minus what .okfignore excludes."""
        return [f for f in self.all_markdown(root) if not self.is_ignored(f)]

    def is_ignored(self, path: str) -> bool:
        """True if .okfignore excludes this repository-relative file, or a directory above it."""
        if not self.ignore:
            return False
        parts = self.relative(path).split("/")
        for depth in range(1, len(parts)):
            if _ignored("/".join(parts[:depth]), True, self.ignore):
                return True
        return _ignored("/".join(parts), False, self.ignore)


def load_bundle(root: Path, config: dict[str, Any]) -> Bundle | None:
    """The bundle declared under rules.checks.okf, or None when the KB declares none.

    `bundle` must be a directory inside the repository; `version`, when set, is the
    okf_version the bundle-root index.md must declare.
    """
    okf = get_value(config, "rules.checks.okf")
    if okf is None:
        return None
    if not isinstance(okf, dict):
        raise ConfigError(f"{CONFIG_FILE}: 'rules.checks.okf' must be a mapping")
    bundle = okf.get("bundle")
    if not isinstance(bundle, str) or not bundle.strip():
        raise ConfigError(f"{CONFIG_FILE}: 'rules.checks.okf.bundle' must be a directory path")
    normalized = posixpath.normpath(bundle.strip().strip("/") or ".")
    normalized = "" if normalized == "." else normalized
    if normalized.startswith("..") or not (root / normalized).is_dir():
        raise ConfigError(
            f"{CONFIG_FILE}: 'rules.checks.okf.bundle' is not a directory in the repository: {bundle}"
        )
    version = okf.get("version")
    if version is not None and not isinstance(version, str):
        raise ConfigError(f"{CONFIG_FILE}: 'rules.checks.okf.version' must be a string")
    ignore_path = root / normalized / IGNORE_FILE
    ignore = parse_ignore(ignore_path.read_text(encoding="utf-8")) if ignore_path.is_file() else []
    return Bundle(root=normalized, version=version, ignore=ignore)


def parse_ignore(text: str) -> list[tuple[re.Pattern[str], bool, bool]]:
    """Read .okfignore with .gitignore pattern rules: (regex, negated, directory-only) per line."""
    rules: list[tuple[re.Pattern[str], bool, bool]] = []
    for raw in text.splitlines():
        line = re.sub(r"(?<!\\) +$", "", raw)
        if not line or line.startswith("#"):
            continue
        negated = line.startswith("!")
        if negated or line.startswith(("\\!", "\\#")):
            line = line[1:]
        dir_only = line.endswith("/")
        line = line.rstrip("/")
        if not line:
            continue
        anchored = "/" in line
        line = line.lstrip("/")
        rules.append((_ignore_regex(line, anchored), negated, dir_only))
    return rules


def _ignored(path: str, is_dir: bool, rules: list[tuple[re.Pattern[str], bool, bool]]) -> bool:
    """The last matching rule decides, as in .gitignore."""
    ignored = False
    for regex, negated, dir_only in rules:
        if dir_only and not is_dir:
            continue
        if regex.match(path):
            ignored = not negated
    return ignored


@cache
def _ignore_regex(pattern: str, anchored: bool) -> re.Pattern[str]:
    """Translate a .gitignore pattern. Unanchored patterns match a name at any depth."""
    out = ""
    i = 0
    while i < len(pattern):
        # ** is special only as a whole segment (**/x, x/**, x/**/y); elsewhere it is a *.
        whole = pattern.startswith("**", i) and (i == 0 or pattern[i - 1] == "/")
        if whole and pattern.startswith("**/", i):
            out += "(?:.*/)?"
            i += 3
        elif whole and i + 2 == len(pattern):
            out += ".*"
            i += 2
        elif pattern.startswith("**", i):
            out += "[^/]*"
            i += 2
        elif pattern[i] == "*":
            out += "[^/]*"
            i += 1
        elif pattern[i] == "?":
            out += "[^/]"
            i += 1
        elif pattern[i] == "[" and "]" in pattern[i + 2 :]:
            end = pattern.index("]", i + 2)
            body = pattern[i + 1 : end]
            if body.startswith("!"):
                body = "^" + body[1:]
            out += "[" + body.replace("\\", "\\\\") + "]"
            i = end + 1
        elif pattern[i] == "\\" and i + 1 < len(pattern):
            out += re.escape(pattern[i + 1])
            i += 2
        else:
            out += re.escape(pattern[i])
            i += 1
    prefix = "" if anchored else "(?:.*/)?"
    return re.compile(prefix + out + r"\Z")


def split_frontmatter(content: str) -> tuple[dict[str, Any] | None, str]:
    """(frontmatter, body). Frontmatter is None when the file has no delimited block at its start."""
    match = FRONTMATTER_PATTERN.match(content)
    if not match:
        return None, content
    return parse_subset(match.group(1)), content[match.end() :]


def structured_sources(frontmatter: dict[str, Any] | None) -> tuple[bool, str | None]:
    """(has sources, problem) for a `sources` frontmatter list (meta-KB provenance, OKF §5.1).

    Sources count when the list is non-empty and every entry is a mapping with a non-empty
    `resource`; otherwise the problem says what is wrong. Absent `sources` is (False, None).
    """
    if not frontmatter or "sources" not in frontmatter:
        return False, None
    sources = frontmatter["sources"]
    if not isinstance(sources, list) or not sources:
        return False, "'sources' must be a non-empty list"
    for index, entry in enumerate(sources):
        if not isinstance(entry, dict):
            return False, f"sources[{index}] must be a mapping with a 'resource'"
        resource = entry.get("resource")
        if not isinstance(resource, str) or not resource.strip():
            return False, f"sources[{index}] has no 'resource'"
    return True, None
