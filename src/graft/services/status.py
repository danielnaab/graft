"""Status service for reporting on artifact change status."""
from __future__ import annotations
from dataclasses import dataclass
from pathlib import Path
from ..adapters.config import ConfigAdapter
from ..adapters.git import GitPort


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

    def __init__(self, config_adapter: ConfigAdapter, git_adapter: GitPort):
        """
        Initialize status service.

        Args:
            config_adapter: Adapter for loading artifact configurations
            git_adapter: Adapter for git operations
        """
        self.config_adapter = config_adapter
        self.git_adapter = git_adapter

    def status(self, artifact_path: Path) -> StatusResult:
        """
        Report on artifact change status and downstream impacts.

        Args:
            artifact_path: Path to the artifact directory

        Returns:
            StatusResult containing change origin and downstream artifacts

        The change_origin classification:
        - "fresh": All input materials unchanged since last derivation
        - "stale": One or more input materials have changed
        - "unknown": Unable to determine (not in git repo, or no materials)
        """
        try:
            # Load artifact configuration
            artifact = self.config_adapter.load_artifact(artifact_path)

            # If no materials, we can't determine staleness
            if not artifact.config.inputs.materials:
                return StatusResult(
                    artifact=str(artifact_path),
                    change_origin="unknown",
                    downstream=[],
                )

            # Get repository root
            repo_root = self.git_adapter.get_repo_root(artifact_path)
            if repo_root is None:
                # Not in a git repository
                return StatusResult(
                    artifact=str(artifact_path),
                    change_origin="unknown",
                    downstream=[],
                )

            # Check each material for modifications
            any_modified = False
            for material in artifact.config.inputs.materials:
                # Resolve material path relative to artifact directory
                material_path = artifact_path / material.path
                material_path = material_path.resolve()

                if self.git_adapter.is_file_modified(material_path, material.rev, repo_root):
                    any_modified = True
                    break

            change_origin = "stale" if any_modified else "fresh"

            return StatusResult(
                artifact=str(artifact_path),
                change_origin=change_origin,
                downstream=[],
            )

        except Exception:
            # If anything goes wrong, return unknown
            return StatusResult(
                artifact=str(artifact_path),
                change_origin="unknown",
                downstream=[],
            )
