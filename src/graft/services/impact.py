"""Impact service for analyzing downstream artifacts affected by changes."""
from __future__ import annotations
from dataclasses import dataclass
from pathlib import Path


@dataclass
class ImpactResult:
    """Result of analyzing impact of changes to an artifact.

    Contract: Returns { artifact, downstream[] }
    where downstream is an array of artifacts that depend on this one.
    """

    artifact: str
    downstream: list[dict]

    def to_dict(self) -> dict:
        """Convert to dictionary for JSON output."""
        return {
            "artifact": self.artifact,
            "downstream": self.downstream,
        }


class ImpactService:
    """Service for analyzing downstream artifacts affected by changes."""

    def impact(self, artifact_path: Path) -> ImpactResult:
        """
        Analyze which downstream artifacts are affected by changes to this artifact.

        Args:
            artifact_path: Path to the artifact directory

        Returns:
            ImpactResult containing list of affected downstream artifacts

        Note:
            Current implementation returns empty downstream list as a stub.
            Future implementations will:
            - Parse project-wide graft.yaml files to build dependency graph
            - Identify artifacts that depend on this one via materials
            - Return list of affected downstream artifacts with paths
        """
        return ImpactResult(
            artifact=str(artifact_path),
            downstream=[],
        )
