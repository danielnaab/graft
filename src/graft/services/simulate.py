"""Simulate service for dry-run simulation of artifact builds."""
from __future__ import annotations
from dataclasses import dataclass
from pathlib import Path


@dataclass
class SimulateResult:
    """Result of simulating an artifact build.

    Returns information about what would happen during a build
    without actually modifying any files.
    """

    artifact: str
    cascade: bool
    message: str

    def to_dict(self) -> dict:
        """Convert to dictionary for JSON output."""
        return {
            "artifact": self.artifact,
            "cascade": self.cascade,
            "message": self.message,
        }


class SimulateService:
    """Service for simulating artifact builds without side effects."""

    def simulate(self, artifact_path: Path, cascade: bool = False) -> SimulateResult:
        """
        Simulate an artifact build without modifying the repository.

        Args:
            artifact_path: Path to the artifact directory
            cascade: If True, simulate builds for all downstream artifacts

        Returns:
            SimulateResult containing simulation information

        Note:
            Current implementation is a stub that produces a confirmation message.
            Future implementations will:
            - Validate that all inputs/materials are available
            - Check that templates/transformers are accessible
            - Verify output paths are writable
            - If cascade=True, simulate all downstream artifact builds
            - Report what would be executed without actually running derivations
        """
        cascade_status = "enabled" if cascade else "disabled"
        message = f"Simulation complete for {artifact_path} (cascade={cascade_status})"

        return SimulateResult(
            artifact=str(artifact_path),
            cascade=cascade,
            message=message,
        )
