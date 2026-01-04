"""Entry point for python -m graft."""

import sys

import typer

from graft.cli.commands.exec_command import exec_dependency_command
from graft.cli.main import app

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
