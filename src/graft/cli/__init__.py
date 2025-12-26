"""CLI layer - command-line interface using Typer.

Thin layer over service functions, wiring context and handling I/O.
"""

from graft.cli.main import app

__all__ = ["app"]
