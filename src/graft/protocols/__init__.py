"""Protocol layer - interface definitions using typing.Protocol.

Protocols define interfaces via structural typing (duck typing with type safety).
No inheritance needed - any class implementing the methods satisfies the protocol.
"""

from graft.protocols.repository import Repository

__all__ = [
    "Repository",
]
