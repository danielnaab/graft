"""Adapter layer - implementations of protocol interfaces.

Adapters implement external system integrations (databases, APIs, file systems, etc.)
All adapters implement protocols from the protocols layer.
"""

from graft.adapters.repository import InMemoryRepository

__all__ = [
    "InMemoryRepository",
]
