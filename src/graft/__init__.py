"""Graft - Knowledge base tooling with language server support

DEPRECATED: This Python implementation is deprecated as of February 2026.
The Rust implementation is production-ready and is the recommended version.
See src/graft/DEPRECATED.md for migration information.

This package demonstrates functional service layer architecture with
protocol-based dependency injection.

Key patterns:
- Service functions (not classes) with ServiceContext
- Protocol-based interfaces for flexibility
- Domain modeling with dataclasses
- Clean architecture layers

For documentation, see: docs/README.md
"""

__version__ = "0.1.0"

# Public API exports
from graft.domain.entities import Entity
from graft.services.context import ServiceContext
from graft.services.example_service import (
    create_example,
    get_example,
    list_examples,
)

__all__ = [
    "__version__",
    "Entity",
    "ServiceContext",
    "create_example",
    "get_example",
    "list_examples",
]
