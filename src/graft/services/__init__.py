"""Service layer - business logic as pure functions.

Service functions take ServiceContext as first parameter, providing all dependencies.
Functions are pure, testable, and composable.
"""

from graft.services.context import ServiceContext
from graft.services.example_service import (
    create_example,
    get_example,
    list_examples,
)

__all__ = [
    "ServiceContext",
    "create_example",
    "get_example",
    "list_examples",
]
