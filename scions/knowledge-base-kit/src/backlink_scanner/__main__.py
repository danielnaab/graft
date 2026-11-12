# spec: specs/backlink-scanner.md
# spec-section: Behavior/Exit codes

"""CLI entry point for the backlink scanner."""

from backlink_scanner.scanner import ScanResult, scan
from tool_cli import run_tool


def _serialize(result: ScanResult) -> dict:
    """Convert ScanResult to a JSON-serializable dict."""
    specs = {}
    for spec_path, entry in result.specs.items():
        specs[spec_path] = {
            "implementors": entry.implementors,
            "sections": entry.sections,
        }
    return {
        "specs": specs,
        "dangling": result.dangling,
        "orphans": result.orphans,
    }


def main() -> None:
    run_tool(
        runner=scan,
        serializer=_serialize,
        has_failures=lambda r: bool(r.dangling or r.orphans),
    )


if __name__ == "__main__":
    main()
