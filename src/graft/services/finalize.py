"""Finalize service for recording provenance of artifact changes."""
from __future__ import annotations
import json
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

from ..adapters.filesystem import FileSystemPort


@dataclass
class AgentInfo:
    """Information about the agent that made changes."""

    name: str
    model: Optional[str] = None
    params: Optional[str] = None

    def to_dict(self) -> dict:
        """Convert to dictionary for JSON output."""
        return {
            "name": self.name,
            "model": self.model,
            "params": self.params,
        }


@dataclass
class FinalizeResult:
    """Result of finalizing an artifact.

    Contains the path to the created provenance file and the provenance data.
    """

    provenance_path: Path
    artifact: str
    finalized_at: str
    agent: Optional[AgentInfo]
    change_origin: str

    def to_dict(self) -> dict:
        """Convert to dictionary for JSON output."""
        return {
            "artifact": self.artifact,
            "finalized_at": self.finalized_at,
            "agent": self.agent.to_dict() if self.agent else None,
            "change_origin": self.change_origin,
        }


class FinalizeService:
    """Service for finalizing artifacts and recording provenance."""

    def __init__(self, filesystem: FileSystemPort):
        self.filesystem = filesystem

    def finalize(
        self,
        artifact_path: Path,
        agent: Optional[AgentInfo] = None,
    ) -> FinalizeResult:
        """
        Finalize artifact changes and write provenance record.

        Args:
            artifact_path: Path to the artifact directory
            agent: Optional information about the agent that made changes

        Returns:
            FinalizeResult containing provenance information

        Note:
            Current implementation always sets change_origin to "authored".
            Future implementations may detect the actual origin based on file analysis.
        """
        # Prepare provenance data
        finalized_at = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

        result = FinalizeResult(
            provenance_path=artifact_path / ".graft" / "provenance" / "finalize.json",
            artifact=str(artifact_path),
            finalized_at=finalized_at,
            agent=agent,
            change_origin="authored",
        )

        # Write provenance file
        provenance_dir = artifact_path / ".graft" / "provenance"
        self.filesystem.mkdir(provenance_dir, parents=True, exist_ok=True)

        provenance_file = provenance_dir / "finalize.json"
        self.filesystem.write_text(provenance_file, json.dumps(result.to_dict(), indent=2))

        return result
