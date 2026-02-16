"""Entry point for python -m graft.

DEPRECATED: This Python implementation is deprecated as of February 2026.
The Rust implementation is production-ready and is the recommended version.
See src/graft/DEPRECATED.md for migration information.
"""

import sys
import warnings

import typer

from graft.cli.commands.exec_command import exec_dependency_command
from graft.cli.main import app

# Show deprecation warning when the Python CLI is invoked
warnings.warn(
    "The Python implementation of Graft is deprecated. "
    "Please migrate to the Rust CLI: cargo install --path crates/graft-cli. "
    "See src/graft/DEPRECATED.md for details.",
    DeprecationWarning,
    stacklevel=2,
)

if __name__ == "__main__":
    # Check if first argument is dep:command syntax
    if len(sys.argv) > 1 and ":" in sys.argv[1]:
        # Parse dep:command syntax
        dep_command = sys.argv[1]
        if dep_command.count(":") == 1:
            dep_name, command_name = dep_command.split(":", 1)
            # Get additional args if provided
            extra_args = sys.argv[2:] if len(sys.argv) > 2 else None
            # Execute the command
            try:
                exec_dependency_command(dep_name, command_name, extra_args)
            except typer.Exit as e:
                sys.exit(e.exit_code)
        else:
            # Invalid syntax, fall through to normal typer handling
            app()
    else:
        # Normal command routing through typer
        app()
