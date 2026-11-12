"""Shared CLI runner for the knowledge-base-kit's tools.

Provides the common pattern: parse args, run tool, serialize, print JSON, exit.
"""
# spec: specs/tool-cli.md

import json
import sys
from collections.abc import Callable
from typing import Any, TypeVar

from tool_cli.config import ConfigError

T = TypeVar("T")


def run_tool(
    runner: Callable[[str], T],
    serializer: Callable[[T], dict[str, Any]],
    has_failures: Callable[[T], bool],
) -> None:
    """Run a tool with standard CLI conventions.

    All tools share: --report-only flag, optional directory argument,
    JSON output, exit 0/1 based on findings, exit 2 on misconfiguration.

    Args:
        runner: Function taking root_dir, returning a result object.
        serializer: Converts the result to a JSON-serializable dict.
        has_failures: Returns True if the result warrants exit code 1.
    """
    report_only = "--report-only" in sys.argv
    args = [a for a in sys.argv[1:] if a != "--report-only"]
    root_dir = args[0] if args else "."

    try:
        result = runner(root_dir)
    except (FileNotFoundError, ConfigError) as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(2)

    print(json.dumps(serializer(result), indent=2))

    if report_only:
        sys.exit(0)

    if has_failures(result):
        sys.exit(1)
