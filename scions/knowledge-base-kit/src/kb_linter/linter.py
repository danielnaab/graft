# spec: specs/kb-linter.md
# spec-section: Behavior/Frontmatter validation

"""Core linting logic for validating KB content against declared rules."""

import re
from dataclasses import dataclass, field
from pathlib import Path

from tool_cli import scope
from tool_cli.config import get_list, load_config
from tool_cli.okf import RESERVED_NAMES, Bundle, load_bundle, split_frontmatter, structured_sources

FRONTMATTER_PATTERN = re.compile(r"^---\s*\n(.*?)^---\s*\n", re.DOTALL | re.MULTILINE)
STATUS_PATTERN = re.compile(r"^status:\s*(.+?)\s*$", re.MULTILINE)
# "## Sources", optionally followed by more words ("## Sources and verification").
SOURCES_HEADING_PATTERN = re.compile(r"^## Sources(?:[ \t].*)?$", re.MULTILINE)

CONTENT_DIRS = ("docs", "policies", "playbooks")

# Decision records (ADRs) record a decision state, a separate axis from
# document lifecycle, so they may use these statuses as well as the KB's own.
DECISION_STATUSES = ("proposed", "accepted", "rejected", "superseded", "deprecated")

# An OKF index entry (§8): "* [Title](url)" with an optional " - description".
INDEX_ENTRY_PATTERN = re.compile(r"^[*-] \[[^\]]+\]\([^)]+\)(?: - .+)?$")
HEADING_PATTERN = re.compile(r"^#{1,6} \S")
COMMENT_LINE_PATTERN = re.compile(r"^<!--.*-->$")
FENCE_PATTERN = re.compile(r"^(`{3,}|~{3,})")
LOG_DATE_HEADING_PATTERN = re.compile(r"^## \d{4}-\d{2}-\d{2}$")

# Always allowed: marks an always-current document (docs/policies/lifecycle.md, temporal-layers.md).
LIVING_STATUS = "living"


@dataclass
class Violation:
    """A single linting violation."""

    file: str
    rule: str
    message: str


@dataclass
class LintConfig:
    """Configuration read from knowledge-base.yaml."""

    valid_statuses: list[str] = field(default_factory=list)
    # sources.canonical paths: the provenance scope unless provenance_include is set.
    provenance_paths: list[str] = field(default_factory=list)
    extra_statuses: list[str] = field(default_factory=list)
    lifecycle_include: list[str] = field(default_factory=list)
    lifecycle_exclude: list[str] = field(default_factory=list)
    provenance_include: list[str] | None = None
    provenance_exclude: list[str] = field(default_factory=list)
    # rules.checks.okf: the OKF bundle whose notes are checked for conformance.
    bundle: Bundle | None = None


@dataclass
class LintResult:
    """Result of linting a KB directory."""

    violations: list[Violation] = field(default_factory=list)
    files_checked: int = 0


def parse_config(root: Path) -> LintConfig:
    """Read linting configuration from knowledge-base.yaml.

    Raises FileNotFoundError when the file is missing and ConfigError when a value the
    linter enforces is present but unreadable (the tool must not silently check nothing).
    """
    config = load_config(root, required=True)
    bundle = load_bundle(root, config)

    canonical = _canonical_paths(config)
    lifecycle_include = get_list(config, "rules.checks.lifecycle.include")
    return LintConfig(
        valid_statuses=get_list(config, "rules.lifecycle.statuses") or [],
        provenance_paths=canonical,
        extra_statuses=get_list(config, "rules.checks.lifecycle.extraStatuses") or [],
        lifecycle_include=(
            lifecycle_include
            if lifecycle_include is not None
            else [f"{d}/**" for d in CONTENT_DIRS]
        ),
        lifecycle_exclude=get_list(config, "rules.checks.lifecycle.exclude") or [],
        provenance_include=get_list(config, "rules.checks.provenance.include"),
        provenance_exclude=get_list(config, "rules.checks.provenance.exclude") or [],
        bundle=bundle,
    )


def _canonical_paths(config: dict) -> list[str]:
    """The `path:` of each sources.canonical entry."""
    sources = config.get("sources")
    entries = sources.get("canonical") if isinstance(sources, dict) else None
    paths: list[str] = []
    for entry in entries or []:
        if isinstance(entry, dict) and isinstance(entry.get("path"), str):
            paths.append(entry["path"])
    return paths


def _is_decision_record(path: str) -> bool:
    """A decision record lives under a decisions/ directory or is named adr-*.md."""
    parts = Path(path).parts
    name = parts[-1]
    return "decisions" in parts[:-1] or name.lower().startswith("adr-")


def _check_file(
    root: Path,
    file: str,
    config: LintConfig,
    check_status: bool,
    check_provenance: bool,
    check_okf: bool = False,
) -> list[Violation]:
    """Check a single file for violations."""
    violations: list[Violation] = []
    full_path = root / file

    try:
        content = full_path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return violations

    # Check frontmatter presence
    fm_match = FRONTMATTER_PATTERN.match(content)
    if check_status and not fm_match:
        violations.append(
            Violation(
                file=file,
                rule="missing-frontmatter",
                message="No YAML frontmatter found",
            )
        )
    elif check_status and fm_match:
        frontmatter = fm_match.group(1)
        status_match = STATUS_PATTERN.search(frontmatter)
        if not status_match:
            violations.append(
                Violation(
                    file=file,
                    rule="missing-status",
                    message="No status field in frontmatter",
                )
            )
        elif config.valid_statuses:
            status_value = status_match.group(1)
            allowed = list(config.valid_statuses)
            allowed += [s for s in (LIVING_STATUS, *config.extra_statuses) if s not in allowed]
            if _is_decision_record(file):
                allowed += [s for s in DECISION_STATUSES if s not in allowed]
            if status_value not in allowed:
                violations.append(
                    Violation(
                        file=file,
                        rule="invalid-status",
                        message=(f"Invalid status '{status_value}', allowed: {allowed}"),
                    )
                )

    frontmatter, body = split_frontmatter(content)
    has_sources, sources_problem = structured_sources(frontmatter)
    if sources_problem:
        violations.append(Violation(file=file, rule="invalid-sources", message=sources_problem))

    if (
        check_provenance
        and not has_sources
        and not sources_problem
        and not SOURCES_HEADING_PATTERN.search(content)
    ):
        violations.append(
            Violation(
                file=file,
                rule="missing-provenance",
                message=(
                    "No Sources section or sources frontmatter found "
                    "(required for canonical content)"
                ),
            )
        )

    if check_okf:
        offset = content[: len(content) - len(body)].count("\n")
        violations.extend(_check_okf(file, frontmatter, body, offset, config.bundle))

    return violations


def _check_okf(
    file: str, frontmatter: dict | None, body: str, offset: int, bundle: Bundle | None
) -> list[Violation]:
    """OKF v0.2 conformance (§11): concepts carry a type; index.md and log.md keep their shape."""
    assert bundle is not None
    name = Path(file).name
    if name == "index.md":
        return _check_index(file, frontmatter, body, offset, bundle)
    if name == "log.md":
        return [
            Violation(
                file=file,
                rule="okf-log-date",
                message=f"line {n}: a log heading must be a date (## YYYY-MM-DD): {line}",
            )
            for n, line in _body_lines(body, offset)
            if line.startswith("## ") and not LOG_DATE_HEADING_PATTERN.match(line)
        ]
    if frontmatter is None:
        return [
            Violation(
                file=file,
                rule="okf-missing-frontmatter",
                message="No YAML frontmatter found (OKF notes need one with a type)",
            )
        ]
    type_value = frontmatter.get("type")
    if not isinstance(type_value, str) or not type_value.strip():
        return [Violation(file=file, rule="okf-missing-type", message="No type in frontmatter")]
    return []


def _check_index(
    file: str, frontmatter: dict | None, body: str, offset: int, bundle: Bundle
) -> list[Violation]:
    """An index has no frontmatter (the bundle root's may hold okf_version) and lists entries."""
    violations: list[Violation] = []
    is_root = bundle.relative(file) == "index.md"
    if frontmatter is not None:
        extra = sorted(k for k in frontmatter if not (is_root and k == "okf_version"))
        if extra or not is_root:
            allowed = "only okf_version" if is_root else "none"
            violations.append(
                Violation(
                    file=file,
                    rule="okf-index-frontmatter",
                    message=f"Index frontmatter allows {allowed}; found: {', '.join(extra) or 'a block'}",
                )
            )
    for n, line in _body_lines(body, offset):
        if not (
            HEADING_PATTERN.match(line)
            or INDEX_ENTRY_PATTERN.match(line)
            or COMMENT_LINE_PATTERN.match(line)
        ):
            violations.append(
                Violation(
                    file=file,
                    rule="okf-index-entry",
                    message=f"line {n}: not a heading or an entry (* [Title](url) - description): {line}",
                )
            )
    return violations


def _body_lines(body: str, offset: int) -> list[tuple[int, str]]:
    """Non-blank body lines outside code fences, numbered as in the file (offset: frontmatter lines)."""
    lines: list[tuple[int, str]] = []
    in_fence = False
    for n, raw in enumerate(body.splitlines(), start=offset + 1):
        line = raw.strip()
        if FENCE_PATTERN.match(line):
            in_fence = not in_fence
            continue
        if line and not in_fence:
            lines.append((n, line))
    return lines


def _okf_version_violations(root: Path, bundle: Bundle) -> list[Violation]:
    """With rules.checks.okf.version set, the bundle-root index.md must declare that okf_version."""
    if bundle.version is None:
        return []
    index = f"{bundle.root}/index.md" if bundle.root else "index.md"
    try:
        frontmatter, _ = split_frontmatter((root / index).read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError):
        frontmatter = None
    declared = (frontmatter or {}).get("okf_version")
    if declared == bundle.version:
        return []
    return [
        Violation(
            file=index,
            rule="okf-version",
            message=f'okf_version must be "{bundle.version}"; found: {declared!r}',
        )
    ]


def _is_index(file: str) -> bool:
    """A README.md is an index of the docs beside it, not a source of truth: no Sources needed."""
    return Path(file).name == "README.md"


def lint(root_dir: str) -> LintResult:
    """Lint a KB directory against its declared rules.

    Reads configuration from knowledge-base.yaml and validates content files
    for frontmatter status and provenance requirements.

    Args:
        root_dir: The KB root directory.

    Returns:
        LintResult with violations and file count.
    """
    root = Path(root_dir).resolve()
    config = parse_config(root)

    status_files = set(scope.collect(root, config.lifecycle_include, config.lifecycle_exclude))
    if config.provenance_include is None:
        # Canonical paths inside the lifecycle scope; canonical paths elsewhere
        # (AGENTS.md, flake.nix) name sources of truth and are not scanned.
        provenance_files = {
            f for f in status_files if scope.matches_any(f, config.provenance_paths)
        }
    else:
        provenance_files = set(
            scope.collect(root, config.provenance_include, config.provenance_exclude)
        )
    provenance_files = {f for f in provenance_files if not _is_index(f)}

    okf_files: set[str] = set()
    all_violations: list[Violation] = []
    if config.bundle is not None:
        bundle = config.bundle
        okf_files = set(bundle.notes(root))
        # index.md and log.md are OKF listings, not concepts: no status or Sources there.
        reserved = {f for f in okf_files if Path(f).name in RESERVED_NAMES}
        status_files -= reserved
        provenance_files -= reserved
        all_violations.extend(_okf_version_violations(root, bundle))

    files = sorted(status_files | provenance_files | okf_files)
    for file in files:
        all_violations.extend(
            _check_file(
                root,
                file,
                config,
                file in status_files,
                file in provenance_files,
                file in okf_files,
            )
        )

    return LintResult(violations=all_violations, files_checked=len(files))
