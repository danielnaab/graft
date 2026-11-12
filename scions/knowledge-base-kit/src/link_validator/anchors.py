# spec: specs/link-validator.md
# spec-section: Behavior/Anchors

"""The fragments a markdown file answers to: heading slugs, heading text, block ids, HTML ids."""

import html
import re
from functools import cache
from pathlib import Path

from tool_cli.okf import split_frontmatter

ATX_HEADING = re.compile(r"^ {0,3}(#{1,6})(?:[ \t]+(.*?))?(?:[ \t]+#+)?[ \t]*$")
SETEXT_UNDERLINE = re.compile(r"^ {0,3}(=+|-+)[ \t]*$")
FENCE = re.compile(r"^ {0,3}(`{3,}|~{3,})")
BLOCK_ID = re.compile(r"(?:^|\s)\^([A-Za-z0-9-]+)[ \t]*$")
HTML_ID = re.compile(r"""<[A-Za-z][^>]*?\s(?:id|name)\s*=\s*["']([^"']+)["']""")
MARKDOWN_IMAGE = re.compile(r"!\[[^\]]*\]\([^)]*\)")
MARKDOWN_LINK = re.compile(r"\[([^\]]*)\]\([^)]*\)")
HTML_TAG = re.compile(r"<[^>]+>")
# Blockquote markers and a list marker in front of a heading (> ## A, - ## B, > 1. ## C).
CONTAINER_PREFIX = re.compile(r"^(?:[ \t]*>[ \t]?)*(?:[ \t]*(?:[-*+]|\d+[.)])[ \t]+)?")
GITHUB_DROP = re.compile(r"[^\w\- ]")
NON_WORD_RUN = re.compile(r"[^\w]+")
OBSIDIAN_DROP = re.compile(r"[\[\]#|^:\\]")


def _plain(text: str) -> str:
    """Heading text as rendered: images and HTML tags dropped, link text kept, entities decoded."""
    plain = MARKDOWN_LINK.sub(r"\1", MARKDOWN_IMAGE.sub("", text))
    plain = html.unescape(HTML_TAG.sub("", plain))
    return re.sub(r"[`*~]", "", plain)  # emphasis and code marks; "_" stays, as in snake_case


def github_slug(text: str) -> str:
    """GitHub's heading id: plain text, lower case, punctuation dropped, spaces to hyphens."""
    return GITHUB_DROP.sub("", _plain(text).strip().lower()).replace(" ", "-")


def dashed_slug(text: str) -> str:
    """The slug some renderers write instead: every run of punctuation and space becomes one "-"."""
    return NON_WORD_RUN.sub("-", _plain(text).strip().lower()).strip("-")


def _heading_in_list(line: str) -> str:
    """A list item holding a heading (- ## Heading) as the heading line; other lines unchanged."""
    stripped = CONTAINER_PREFIX.sub("", line, count=1)
    return stripped if stripped != line and ATX_HEADING.match(stripped) else line


def _obsidian_key(text: str) -> str:
    """Heading text as Obsidian compares it in a link: link syntax and spacing ignored, any case."""
    return " ".join(OBSIDIAN_DROP.sub(" ", text).split()).lower()


@cache
def _anchors(path: Path, mtime_ns: int) -> tuple[frozenset[str], frozenset[str], frozenset[str]]:
    """(GitHub slugs, Obsidian heading keys, block and HTML ids) of a markdown file."""
    del mtime_ns  # part of the cache key only
    try:
        content = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return frozenset(), frozenset(), frozenset()
    _, body = split_frontmatter(content)

    headings: list[str] = []
    ids: set[str] = set()
    fence = ""  # the open fence's marker; a closing fence repeats its character, at least as long
    previous = ""
    for raw in body.splitlines():
        line = CONTAINER_PREFIX.sub("", raw, count=1) if raw.lstrip().startswith(">") else raw
        line = line if ATX_HEADING.match(line) else _heading_in_list(line)
        marker = FENCE.match(line)
        if fence:
            closes = marker and line.strip() == marker.group(1)
            if closes and marker.group(1)[0] == fence[0] and len(marker.group(1)) >= len(fence):
                fence = ""
            continue
        if marker:
            fence = marker.group(1)
            previous = ""
            continue
        atx = ATX_HEADING.match(line)
        if atx:
            headings.append(atx.group(2) or "")
            previous = ""
            continue
        if (
            previous.strip()
            and SETEXT_UNDERLINE.match(line)
            and not previous.startswith(("-", "*", ">"))
        ):
            headings.append(previous.strip())
            previous = ""
            continue
        block = BLOCK_ID.search(line)
        if block:
            ids.add("^" + block.group(1))
        ids.update(HTML_ID.findall(line))
        previous = line

    # github-slugger: a repeat takes the next free "-N", skipping slugs already taken (Foo, Foo, Foo-1
    # give foo, foo-1, foo-1-1).
    slugs: set[str] = set()
    seen: dict[str, int] = {}
    for heading in headings:
        base = github_slug(heading)
        slug = base
        while slug in slugs:
            seen[base] = seen.get(base, 0) + 1
            slug = f"{base}-{seen[base]}"
        slugs.add(slug)
    slugs.update(dashed_slug(h) for h in headings)
    keys = frozenset(_obsidian_key(h) for h in headings)
    return frozenset(slugs), keys, frozenset(ids)


def has_anchor(path: Path, fragment: str) -> bool:
    """True if the file answers to the (decoded) fragment.

    Accepted: a GitHub heading slug (any case), Obsidian heading text (`Heading text`, or nested
    `Parent#Child`, matched on the last part), an Obsidian block id (`^id`), or an HTML id/name.
    """
    try:
        mtime = path.stat().st_mtime_ns
    except OSError:
        return False
    slugs, keys, ids = _anchors(path, mtime)
    if fragment in ids:
        return True
    if fragment.lower() in slugs:
        return True
    last = fragment.split("#")[-1]
    return _obsidian_key(last) in keys or github_slug(last) in slugs
