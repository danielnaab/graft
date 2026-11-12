"""Path scopes: include/exclude globs over repository-relative paths, and the walk they drive."""
# spec: specs/tool-cli.md
# spec-section: Path scopes

from __future__ import annotations

import fnmatch
import os
import re
from collections.abc import Iterable
from functools import cache
from pathlib import Path

from tool_cli.graft import is_graft_dir

SKIP_DIRS = frozenset({".git", ".graft", ".venv", "node_modules", "__pycache__"})


@cache
def _glob_regex(pattern: str) -> re.Pattern[str]:
    """Translate a glob: * within a segment, ** across segments, ? one character."""
    out = ""
    i = 0
    while i < len(pattern):
        if pattern.startswith("**/", i):
            out += "(?:.*/)?"
            i += 3
        elif pattern.startswith("**", i):
            out += ".*"
            i += 2
        elif pattern[i] == "*":
            out += "[^/]*"
            i += 1
        elif pattern[i] == "?":
            out += "[^/]"
            i += 1
        else:
            out += re.escape(pattern[i])
            i += 1
    return re.compile(out + r"\Z")


def matches(path: str, pattern: str) -> bool:
    """True if a repository-relative POSIX path matches the glob.

    `dir/**` matches everything beneath dir/ (a trailing ** needs the separator).
    """
    return bool(_glob_regex(pattern).match(path))


def matches_any(path: str, patterns: Iterable[str]) -> bool:
    return any(matches(path, p) for p in patterns)


def _can_reach(directory: str, pattern: str) -> bool:
    """True if a glob could match something at or beneath this directory."""
    dir_parts = directory.split("/")
    for index, part in enumerate(pattern.split("/")):
        if part == "**":
            return True
        if index >= len(dir_parts):
            return True
        if not fnmatch.fnmatchcase(dir_parts[index], part):
            return False
    return False  # the pattern ends above this directory and has no ** to go deeper


def collect(root: Path, include: Iterable[str], exclude: Iterable[str] = ()) -> list[str]:
    """Sorted repository-relative .md paths matching an include and no exclude.

    The walk skips SKIP_DIRS and graft directories, and descends only where an include can reach.
    """
    includes = list(include)
    excludes = list(exclude)
    found: list[str] = []
    for dirpath, dirnames, filenames in os.walk(root):
        rel_dir = os.path.relpath(dirpath, root).replace(os.sep, "/")
        rel_dir = "" if rel_dir == "." else rel_dir
        dirnames[:] = sorted(
            d
            for d in dirnames
            if d not in SKIP_DIRS
            and not is_graft_dir(Path(dirpath, d))
            and any(_can_reach(f"{rel_dir}/{d}" if rel_dir else d, p) for p in includes)
        )
        for filename in filenames:
            if not filename.endswith(".md"):
                continue
            rel = f"{rel_dir}/{filename}" if rel_dir else filename
            if matches_any(rel, includes) and not matches_any(rel, excludes):
                found.append(rel)
    return sorted(found)
