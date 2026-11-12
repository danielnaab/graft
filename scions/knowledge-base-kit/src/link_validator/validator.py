# spec: specs/link-validator.md
# spec-section: Behavior/Link extraction

"""Core validation logic for detecting broken internal markdown links."""

import difflib
import os
import re
from dataclasses import dataclass, field
from pathlib import Path
from urllib.parse import unquote

from link_validator.anchors import has_anchor
from tool_cli import scope
from tool_cli.config import CONFIG_FILE, ConfigError, get_list, get_value, load_config
from tool_cli.okf import Bundle, load_bundle

# Matches [text](target) and ![alt](target)
LINK_PATTERN = re.compile(r"!?\[(?:[^\]]*)\]\(([^)]*)\)")

# [[Wikilinks]]: not Markdown links, so a bundle (Markdown links only) rejects them.
# Not followed by "(": [[1]](#ref) is a citation-style link, not a wikilink.
WIKILINK_PATTERN = re.compile(r"!?\[\[[^\]\n]+\]\](?!\()")

# A destination followed by a CommonMark link title: path "Title" or path 'Title'.
TITLE_PATTERN = re.compile(r"""^(<[^>]*>|\S+)\s+(?:"[^"]*"|'[^']*')\s*$""")

# Filenames many folders share: an unwritten one is not a misdirected link to another folder's.
COMMON_NAMES = frozenset({"index.md", "readme.md", "log.md"})

# How close an unwritten note's name must be to an existing one to be flagged as a likely typo.
TYPO_CUTOFF = 0.85

# Matches opening/closing code fences
FENCE_PATTERN = re.compile(r"^(`{3,}|~{3,})")

# Matches inline code spans (backtick-wrapped)
INLINE_CODE_PATTERN = re.compile(r"(`+)(.+?)\1")

# HTML comments: links inside one are not rendered, so they are not checked.
COMMENT_OPEN = "<!--"
COMMENT_CLOSE = "-->"

CONTENT_DIRS = ("docs", "policies", "playbooks", "notes", "specs")

# A target that begins with a URI scheme (RFC 3986 section 3.1: a letter, then
# letters, digits, "+", "-" or ".", then ":") is external: http:, mailto:,
# airmail:, tel:, file:, data:, app: and so on. A relative path cannot match
# because "/", "?" and "#" are not scheme characters, so the colon must come
# before any of them.
URI_SCHEME_PATTERN = re.compile(r"^[A-Za-z][A-Za-z0-9+.\-]*:")


@dataclass
class LinkViolation:
    """A single link finding: a failure in `violations`, or a notice that does not fail."""

    file: str
    target: str
    resolved: str
    rule: str = "broken-link"
    message: str = "Link target does not exist"


@dataclass
class ValidateResult:
    """Result of validating links across a KB."""

    violations: list[LinkViolation] = field(default_factory=list)
    # Findings that do not fail the run: unwritten notes in an OKF bundle, likely typos.
    notices: list[LinkViolation] = field(default_factory=list)
    files_checked: int = 0
    links_checked: int = 0


def _strip_comments(line: str, in_comment: bool) -> tuple[str, bool]:
    """Remove the parts of a line inside HTML comments, which may span lines.

    Returns the visible text and whether a comment is still open at the end.
    """
    visible = ""
    rest = line
    while rest:
        if in_comment:
            end = rest.find(COMMENT_CLOSE)
            if end == -1:
                return visible, True
            rest = rest[end + len(COMMENT_CLOSE) :]
            in_comment = False
        else:
            start = rest.find(COMMENT_OPEN)
            if start == -1:
                return visible + rest, False
            visible += rest[:start]
            rest = rest[start + len(COMMENT_OPEN) :]
            in_comment = True
    return visible, in_comment


def _visible_lines(content: str) -> list[str]:
    """Each line's rendered text: fenced code blocks and HTML comments dropped, code spans neutral."""
    visible_lines: list[str] = []
    in_fence = False
    in_comment = False
    fence_char = ""
    fence_len = 0

    for line in content.splitlines():
        # Inside an HTML comment nothing counts, fences included, until it closes.
        if in_comment:
            line, in_comment = _strip_comments(line, True)
            if in_comment:
                continue

        stripped = line.strip()
        fence_match = FENCE_PATTERN.match(stripped)

        if fence_match:
            char = fence_match.group(1)[0]
            length = len(fence_match.group(1))

            if not in_fence:
                in_fence = True
                fence_char = char
                fence_len = length
                continue
            elif char == fence_char and length >= fence_len and stripped == char * length:
                # Closing fence: same char, at least as long, nothing else on line
                in_fence = False
                continue

        if in_fence:
            continue

        # Replace inline code spans with a neutral placeholder before extracting
        # links: a link written inside a code span disappears, but a code span
        # used as link text ([`name`](path)) leaves a matchable [code](path).
        # Code spans go first, so a `<!--` written inside one doesn't open a comment.
        line_without_code = INLINE_CODE_PATTERN.sub("code", line)
        visible, in_comment = _strip_comments(line_without_code, False)
        visible_lines.append(visible)

    return visible_lines


def _extract_links(content: str) -> list[str]:
    """Extract link targets from markdown, skipping fenced code blocks and HTML comments."""
    targets: list[str] = []
    for visible in _visible_lines(content):
        for match in LINK_PATTERN.finditer(visible):
            target = match.group(1).strip()
            if target:
                targets.append(target)
    return targets


def _extract_wikilinks(content: str) -> list[str]:
    """[[Wikilinks]] outside code and comments."""
    return [m.group(0) for line in _visible_lines(content) for m in WIKILINK_PATTERN.finditer(line)]


def _split_title(target: str) -> str:
    """Drop a CommonMark link title ([text](path "Title"), [text](<path> "Title"))."""
    match = TITLE_PATTERN.match(target)
    return match.group(1) if match else target


def _strip_target(target: str) -> str:
    """Strip fragment and query string from a link target."""
    # Strip query string first
    if "?" in target:
        target = target[: target.index("?")]
    # Strip fragment
    if "#" in target:
        target = target[: target.index("#")]
    return target


def _resolve_path(target: str, file_dir: str, root_prefix: str = "") -> str | None:
    """Resolve a link target to a path relative to the file's directory.

    Returns None if the target is external or empty after stripping. A target starting
    with "/" resolves from root_prefix: the repository root, or an OKF bundle's root.
    """
    # CommonMark angle-bracket destination: [text](<Name with spaces.md>)
    if target.startswith("<") and target.endswith(">"):
        target = target[1:-1].strip()

    if URI_SCHEME_PATTERN.match(target):
        return None

    # Decode percent-encoding (%20, %2C) as CommonMark and Obsidian do
    stripped = unquote(_strip_target(target))
    if not stripped:
        return None

    # Absolute paths resolve from root
    if stripped.startswith("/"):
        resolved = os.path.normpath(os.path.join(root_prefix, stripped.lstrip("/")))
    else:
        resolved = os.path.normpath(os.path.join(file_dir, stripped))

    # Check for path escape
    if resolved.startswith(".."):
        return resolved  # Will be reported as broken (escapes root)

    return resolved


def _fragment(target: str) -> str:
    """The decoded fragment of a link target, or "" when it has none."""
    if target.startswith("<") and target.endswith(">"):
        target = target[1:-1].strip()
    if "#" not in target:
        return ""
    return unquote(target[target.index("#") + 1 :])


def _get_content_files(root: Path, config: dict) -> list[str]:
    """Collect the markdown files in the configured scope (rules.checks.links)."""
    include = get_list(config, "rules.checks.links.include")
    if include is None:
        include = [f"{d}/**" for d in CONTENT_DIRS]
    exclude = get_list(config, "rules.checks.links.exclude") or []
    return scope.collect(root, include, exclude)


class _BundleNames:
    """The notes in an OKF bundle by lower-cased filename, for spotting misdirected links."""

    def __init__(self, root: Path, bundle: Bundle) -> None:
        self.by_name: dict[str, list[str]] = {}
        for path in bundle.all_markdown(root):
            self.by_name.setdefault(Path(path).name.lower(), []).append(path)

    def same_name(self, resolved: str) -> list[str]:
        return self.by_name.get(Path(resolved).name.lower(), [])

    def close_names(self, resolved: str) -> list[str]:
        name = Path(resolved).name.lower()
        matches = difflib.get_close_matches(name, list(self.by_name), n=3, cutoff=TYPO_CUTOFF)
        return [p for m in matches for p in self.by_name[m]]


def _check_link(
    root: Path,
    file: str,
    raw_target: str,
    bundle: Bundle | None,
    names: _BundleNames | None,
    check_anchors: bool = True,
) -> tuple[str | None, LinkViolation | None, bool]:
    """Classify one link: (resolved path or None if not checked, finding or None, is notice)."""
    target = _split_title(raw_target)
    fragment = _fragment(target)
    in_bundle = bundle is not None and bundle.contains(file)

    def finding(resolved: str, rule: str, message: str) -> LinkViolation:
        return LinkViolation(
            file=file, target=raw_target, resolved=resolved, rule=rule, message=message
        )

    if target.startswith("#"):  # a heading in the same file
        if check_anchors and fragment and not has_anchor(root / file, fragment):
            return (
                file,
                finding(file, "missing-anchor", f"No heading or anchor '{fragment}' in this file"),
                False,
            )
        return file, None, False

    prefix = bundle.root if in_bundle and bundle is not None else ""
    resolved = _resolve_path(target, os.path.dirname(file), prefix)
    if resolved is None:
        return None, None, False  # external or empty

    if in_bundle:
        angle = target.startswith("<") and target.endswith(">")
        if not angle and re.search(r"\s", target):
            return (
                resolved,
                finding(
                    resolved,
                    "malformed-link",
                    "Unencoded space in a link destination; use %20 or <angle brackets>",
                ),
                False,
            )
        if bundle is not None and not resolved.startswith("..") and not bundle.contains(resolved):
            return (
                resolved,
                finding(
                    resolved,
                    "escapes-bundle",
                    f"Link leaves the OKF bundle ({bundle.root or '.'}/); use an absolute URL or a backticked path",
                ),
                False,
            )

    resolved_full = root / resolved
    if resolved_full.exists():
        if (
            check_anchors
            and fragment
            and "?" not in target  # file.md?plain=1#L3 is a source view's line, not a heading
            and resolved_full.is_file()
            and resolved_full.suffix == ".md"
            and not has_anchor(resolved_full, fragment)
        ):
            return (
                resolved,
                finding(
                    resolved, "missing-anchor", f"No heading or anchor '{fragment}' in {resolved}"
                ),
                False,
            )
        return resolved, None, False

    if (
        in_bundle
        and names is not None
        and resolved.endswith(".md")
        and not resolved.startswith("..")
    ):
        common = Path(resolved).name.lower() in COMMON_NAMES
        elsewhere = [] if common else names.same_name(resolved)
        if elsewhere:
            return (
                resolved,
                finding(
                    resolved,
                    "misdirected-link",
                    f"No note here, but one with this name exists: {', '.join(elsewhere)}",
                ),
                False,
            )
        close = [] if common else names.close_names(resolved)
        if close:
            return (
                resolved,
                finding(
                    resolved,
                    "likely-typo",
                    f"Unwritten note with a name close to: {', '.join(close)}",
                ),
                True,
            )
        return resolved, finding(resolved, "unwritten-link", "Note not written yet"), True

    return resolved, LinkViolation(file=file, target=raw_target, resolved=resolved), False


def _anchors_enabled(config: dict) -> bool:
    """rules.checks.links.anchors: true (the default) or false; anything else exits 2."""
    value = get_value(config, "rules.checks.links.anchors")
    if value is None:
        return True
    if isinstance(value, str) and value.lower() in ("true", "false"):
        return value.lower() == "true"
    raise ConfigError(f"{CONFIG_FILE}: 'rules.checks.links.anchors' must be true or false")


def validate(root_dir: str) -> ValidateResult:
    """Validate internal links across a KB directory.

    Scans markdown files for links and checks that targets exist. Inside an OKF bundle
    (rules.checks.okf), a missing note is unwritten (a notice), and links must stay in the bundle.

    Args:
        root_dir: The KB root directory.

    Returns:
        ValidateResult with violations, notices, file count, and link count.
    """
    root = Path(root_dir).resolve()
    config = load_config(root)
    bundle = load_bundle(root, config)
    names = _BundleNames(root, bundle) if bundle is not None else None
    check_anchors = _anchors_enabled(config)
    files = _get_content_files(root, config)
    result = ValidateResult(files_checked=len(files))

    for file in files:
        try:
            content = (root / file).read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue

        if bundle is not None and bundle.contains(file):
            result.violations.extend(
                LinkViolation(
                    file=file,
                    target=wikilink,
                    resolved="",
                    rule="wikilink",
                    message="Wikilinks are not Markdown links; write [text](path.md)",
                )
                for wikilink in _extract_wikilinks(content)
            )

        for raw_target in _extract_links(content):
            resolved, found, is_notice = _check_link(
                root, file, raw_target, bundle, names, check_anchors
            )
            if resolved is not None:
                result.links_checked += 1
            if found is not None:
                (result.notices if is_notice else result.violations).append(found)

    return result
