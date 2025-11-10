"""Status service for reporting on artifact change status."""
from __future__ import annotations
from dataclasses import dataclass
from pathlib import Path


@dataclass
class StatusResult:
    """Result of checking artifact status.

    Contract: Returns { artifact, change_origin, downstream[] }
    where change_origin is a string classification and downstream
    is an array of affected artifacts.
    """

    artifact: str
    change_origin: str
    downstream: list[dict]

    def to_dict(self) -> dict:
        """Convert to dictionary for JSON output."""
        return {
            "artifact": self.artifact,
            "change_origin": self.change_origin,
            "downstream": self.downstream,
        }


class StatusService:
    """Service for checking artifact change status and downstream impacts."""

    def status(self, artifact_path: Path) -> StatusResult:
        """
        Report on artifact change status and downstream impacts.

        Args:
            artifact_path: Path to the artifact directory

        Returns:
            StatusResult containing change origin and downstream artifacts

        Note:
            Current implementation returns "unknown" for change_origin as a stub.
            Future implementations will classify changes as:
            - "derived": Created by running graft derivations
            - "authored": Created by direct human/agent editing
            - "unknown": Unable to determine origin
        """
        return StatusResult(
            artifact=str(artifact_path),
            change_origin="unknown",
            downstream=[],
        )
