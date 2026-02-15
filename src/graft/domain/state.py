"""Domain models for state queries.

State queries enable observability by running commands that output JSON
and caching results tied to git commits.
"""

from dataclasses import dataclass
from datetime import datetime
from typing import Any


@dataclass(frozen=True)
class StateCache:
    """Cache configuration for a state query."""

    deterministic: bool
    """If True, cache is keyed by commit hash and valid indefinitely.
    If False, cache has TTL (not implemented in Stage 1)."""

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "StateCache":
        """Create StateCache from YAML data."""
        return cls(deterministic=data.get("deterministic", True))


@dataclass(frozen=True)
class StateQuery:
    """A state query definition from graft.yaml."""

    name: str
    """Name of the state query (e.g., 'coverage', 'test-results')."""

    run: str
    """Command to execute. Must output valid JSON to stdout."""

    cache: StateCache
    """Cache configuration."""

    timeout: int | None = None
    """Command timeout in seconds. Default: 300 (5 minutes)."""

    @classmethod
    def from_dict(cls, name: str, data: dict[str, Any]) -> "StateQuery":
        """Create StateQuery from YAML data.

        Args:
            name: Name of the state query
            data: Dictionary from graft.yaml state section

        Returns:
            StateQuery instance

        Raises:
            ValueError: If required fields are missing
        """
        if "run" not in data:
            raise ValueError(f"State query '{name}' missing required field 'run'")

        cache_data = data.get("cache", {})
        cache = StateCache.from_dict(cache_data)

        timeout = data.get("timeout")
        if timeout is not None and not isinstance(timeout, int):
            raise ValueError(f"State query '{name}' timeout must be an integer")

        return cls(name=name, run=data["run"], cache=cache, timeout=timeout)


@dataclass(frozen=True)
class StateResult:
    """Result of executing a state query."""

    query_name: str
    """Name of the state query."""

    commit_hash: str
    """Git commit hash when query was executed."""

    data: dict[str, Any]
    """JSON data output from the query."""

    timestamp: datetime
    """When the query was executed."""

    command: str
    """The command that was executed."""

    deterministic: bool
    """Whether this result is deterministic (cacheable by commit)."""

    cached: bool = False
    """Whether this result came from cache."""

    @classmethod
    def from_cache_file(cls, data: dict[str, Any]) -> "StateResult":
        """Load StateResult from cache file JSON.

        Args:
            data: Parsed JSON from cache file

        Returns:
            StateResult instance
        """
        metadata = data["metadata"]
        return cls(
            query_name=metadata.get("query_name", "unknown"),
            commit_hash=metadata["commit_hash"],
            data=data["data"],
            timestamp=datetime.fromisoformat(metadata["timestamp"]),
            command=metadata["command"],
            deterministic=metadata["deterministic"],
            cached=True,
        )

    def to_cache_file(self) -> dict[str, Any]:
        """Serialize StateResult to cache file JSON.

        Returns:
            Dictionary ready for JSON serialization
        """
        return {
            "metadata": {
                "query_name": self.query_name,
                "commit_hash": self.commit_hash,
                "timestamp": self.timestamp.isoformat(),
                "command": self.command,
                "deterministic": self.deterministic,
            },
            "data": self.data,
        }
