# spec: specs/kb-linter.md
# spec-section: Behavior/Exit codes

"""CLI entry point for the KB linter."""

from kb_linter.linter import LintResult, lint
from tool_cli import run_tool


def _serialize(result: LintResult) -> dict:
    """Convert LintResult to a JSON-serializable dict."""
    return {
        "violations": [
            {"file": v.file, "rule": v.rule, "message": v.message} for v in result.violations
        ],
        "summary": {
            "files_checked": result.files_checked,
            "files_passing": result.files_checked - len({v.file for v in result.violations}),
            "violations": len(result.violations),
        },
    }


def main() -> None:
    run_tool(
        runner=lint,
        serializer=_serialize,
        has_failures=lambda r: bool(r.violations),
    )


if __name__ == "__main__":
    main()
