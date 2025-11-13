"""Simulate service for dry-run simulation of artifact builds."""
from __future__ import annotations
from dataclasses import dataclass
from pathlib import Path
from ..adapters.config import ConfigAdapter
from ..adapters.git import GitPort


@dataclass
class MaterialStatus:
    """Status of a single material."""
    path: str
    status: str  # "fresh" or "stale"
    rev: str


@dataclass
class SimulateResult:
    """Result of simulating an artifact build.

    Returns information about what would happen during a build
    without actually modifying any files.
    """

    artifact: str
    status: str  # "fresh" or "stale"
    changed_materials: list[dict]
    cascade: bool

    def to_dict(self) -> dict:
        """Convert to dictionary for JSON output."""
        return {
            "artifact": self.artifact,
            "status": self.status,
            "changed_materials": self.changed_materials,
            "cascade": self.cascade,
        }


class SimulateService:
    """Service for simulating artifact builds without side effects."""

    def __init__(self, config_adapter: ConfigAdapter, git_adapter: GitPort):
        """
        Initialize simulate service.

        Args:
            config_adapter: Adapter for loading artifact configurations
            git_adapter: Adapter for git operations
        """
        self.config_adapter = config_adapter
        self.git_adapter = git_adapter

    def simulate(self, artifact_path: Path, cascade: bool = False) -> SimulateResult:
        """
        Simulate an artifact build without modifying the repository.

        Analyzes which materials have changed and reports what would be
        regenerated without actually running the build.

        Args:
            artifact_path: Path to the artifact directory
            cascade: If True, simulate builds for all downstream artifacts

        Returns:
            SimulateResult containing simulation information
        """
        try:
            # Load the artifact configuration
            artifact = self.config_adapter.load_artifact(artifact_path)

            # If no materials, artifact is always fresh
            if not artifact.config.inputs.materials:
                return SimulateResult(
                    artifact=str(artifact_path),
                    status="fresh",
                    changed_materials=[],
                    cascade=cascade,
                )

            # Get repository root for git operations
            repo_root = self.git_adapter.get_repo_root(artifact_path)
            if repo_root is None:
                # Not in git repo, can't determine staleness
                return SimulateResult(
                    artifact=str(artifact_path),
                    status="unknown",
                    changed_materials=[],
                    cascade=cascade,
                )

            # Check each material for changes
            changed_materials = []
            for material in artifact.config.inputs.materials:
                # Resolve material path relative to artifact directory
                material_path = artifact_path / material.path
                material_path = material_path.resolve()

                # Check if modified
                is_modified = self.git_adapter.is_file_modified(
                    material_path, material.rev, repo_root
                )

                if is_modified:
                    changed_materials.append({
                        "path": str(material.path),
                        "rev": material.rev,
                        "status": "stale"
                    })

            # Determine overall status
            status = "stale" if changed_materials else "fresh"

            return SimulateResult(
                artifact=str(artifact_path),
                status=status,
                changed_materials=changed_materials,
                cascade=cascade,
            )

        except Exception:
            # If we can't simulate, return unknown status
            return SimulateResult(
                artifact=str(artifact_path),
                status="unknown",
                changed_materials=[],
                cascade=cascade,
            )

    def format_output(self, result: SimulateResult) -> str:
        """
        Format simulation result as human-readable text.

        Args:
            result: SimulateResult to format

        Returns:
            Formatted string output
        """
        lines = []

        if result.status == "fresh":
            lines.append(f"Artifact is up-to-date: {result.artifact}")
            lines.append("No changes detected in source materials.")
        elif result.status == "stale":
            lines.append(f"Artifact needs regeneration: {result.artifact}")
            lines.append(f"\nChanged materials ({len(result.changed_materials)}):")
            for material in result.changed_materials:
                lines.append(f"  - {material['path']} (rev: {material['rev']})")
        else:
            lines.append(f"Unable to determine status for: {result.artifact}")

        if result.cascade:
            lines.append(f"\nCascade mode: enabled")
        else:
            lines.append(f"\nCascade mode: disabled")

        return "\n".join(lines)
