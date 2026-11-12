# spec: specs/link-validator.md
# spec-section: Behavior/Exit codes

"""CLI entry point for the link validator."""

from link_validator.validator import LinkViolation, ValidateResult, validate
from tool_cli import run_tool


def _finding(v: LinkViolation) -> dict:
    return {
        "file": v.file,
        "target": v.target,
        "resolved": v.resolved,
        "rule": v.rule,
        "message": v.message,
    }


def _serialize(result: ValidateResult) -> dict:
    """Convert ValidateResult to a JSON-serializable dict. Notices never fail the run."""
    return {
        "violations": [_finding(v) for v in result.violations],
        "notices": [_finding(v) for v in result.notices],
        "summary": {
            "files_checked": result.files_checked,
            "links_checked": result.links_checked,
            "broken": len(result.violations),
            "unwritten": sum(1 for v in result.notices if v.rule == "unwritten-link"),
            "likely_typos": sum(1 for v in result.notices if v.rule == "likely-typo"),
        },
    }


def main() -> None:
    run_tool(
        runner=validate,
        serializer=_serialize,
        has_failures=lambda r: bool(r.violations),
    )


if __name__ == "__main__":
    main()
