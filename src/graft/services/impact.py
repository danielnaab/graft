"""Impact service for analyzing downstream artifacts affected by changes."""
from __future__ import annotations
from dataclasses import dataclass
from pathlib import Path
from ..adapters.config import ConfigAdapter


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

    def __init__(self, config_adapter: ConfigAdapter):
        """
        Initialize impact service.

        Args:
            config_adapter: Adapter for loading artifact configurations
        """
        self.config_adapter = config_adapter

    def impact(self, artifact_path: Path) -> ImpactResult:
        """
        Analyze which downstream artifacts are affected by changes to this artifact.

        Algorithm:
        1. Load the given artifact to get its output paths
        2. Scan the repository for all other artifacts
        3. For each artifact, check if any of its materials match our outputs
        4. Return list of dependent artifacts

        Args:
            artifact_path: Path to the artifact directory

        Returns:
            ImpactResult containing list of affected downstream artifacts
        """
        try:
            # Load the artifact configuration
            artifact = self.config_adapter.load_artifact(artifact_path)

            # Get all output paths from this artifact
            output_paths = self._get_output_paths(artifact_path, artifact.config.derivations)

            # Find the repository root (go up to find other artifacts)
            repo_root = self._find_repo_root(artifact_path)

            # Scan for all artifacts and check dependencies
            downstream = []
            if repo_root:
                downstream = self._find_dependent_artifacts(repo_root, output_paths, artifact_path)

            return ImpactResult(
                artifact=str(artifact_path),
                downstream=downstream
            )

        except Exception:
            # If we can't analyze, return empty downstream
            return ImpactResult(
                artifact=str(artifact_path),
                downstream=[],
            )

    def _get_output_paths(self, artifact_path: Path, derivations: list) -> set:
        """Get all output file paths from an artifact's derivations.

        Args:
            artifact_path: Base path of the artifact
            derivations: List of derivation configurations

        Returns:
            Set of absolute output paths
        """
        outputs = set()
        for derivation in derivations:
            for output in derivation.outputs:
                output_path = artifact_path / output.path
                outputs.add(output_path.resolve())
        return outputs

    def _find_repo_root(self, artifact_path: Path) -> Path | None:
        """Find repository root by looking for common patterns.

        Args:
            artifact_path: Starting path

        Returns:
            Repository root path, or None if not found
        """
        current = artifact_path.resolve()

        # Go up until we find .git or reach filesystem root
        while current != current.parent:
            if (current / ".git").exists():
                return current
            # Also check for common project markers
            if (current / "graft.yaml").exists():  # Root config
                parent_has_artifacts = (current / "artifacts").exists()
                if parent_has_artifacts:
                    return current
            current = current.parent

        # Try parent of artifacts directory as fallback
        if "artifacts" in artifact_path.parts:
            try:
                artifacts_index = list(artifact_path.parts).index("artifacts")
                if artifacts_index > 0:
                    return Path(*artifact_path.parts[:artifacts_index])
            except (ValueError, IndexError):
                pass

        return None

    def _find_dependent_artifacts(
        self, repo_root: Path, output_paths: set, source_artifact_path: Path
    ) -> list:
        """Scan repository for artifacts that depend on given outputs.

        Args:
            repo_root: Root directory to scan
            output_paths: Set of output file paths to check against
            source_artifact_path: Path of the source artifact (to avoid self-reference)

        Returns:
            List of dictionaries describing dependent artifacts
        """
        downstream = []

        # Find all graft.yaml files in the repository
        for graft_file in repo_root.rglob("graft.yaml"):
            artifact_dir = graft_file.parent

            # Skip the source artifact itself
            if artifact_dir.resolve() == source_artifact_path.resolve():
                continue

            # Skip if inside .graft directory
            if ".graft" in artifact_dir.parts:
                continue

            try:
                # Load the artifact configuration
                artifact = self.config_adapter.load_artifact(artifact_dir)

                # Check if any of its materials match our outputs
                if self._artifact_depends_on_outputs(artifact_dir, artifact.config.inputs.materials, output_paths):
                    # Calculate relative path from repo root
                    try:
                        rel_path = artifact_dir.relative_to(repo_root)
                    except ValueError:
                        rel_path = artifact_dir

                    # Add to downstream list
                    downstream.append({
                        "path": str(rel_path),
                        "name": artifact.config.graft
                    })

            except Exception:
                # Skip artifacts that fail to load
                continue

        return downstream

    def _artifact_depends_on_outputs(
        self, artifact_dir: Path, materials: list, output_paths: set
    ) -> bool:
        """Check if an artifact depends on any of the given output paths.

        Args:
            artifact_dir: Directory of the artifact being checked
            materials: List of material references from the artifact
            output_paths: Set of output paths to check against

        Returns:
            True if artifact depends on any of the outputs
        """
        for material in materials:
            # Resolve material path relative to artifact directory
            material_path = artifact_dir / material.path
            material_path = material_path.resolve()

            # Check if this material matches any of our outputs
            if material_path in output_paths:
                return True

        return False
