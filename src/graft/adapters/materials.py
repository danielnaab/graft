"""Material loader adapter for resolving and loading materials."""
from __future__ import annotations
from pathlib import Path
from typing import Protocol

from ..domain.entities import Material
from .filesystem import FileSystemPort


class MaterialNotFoundError(FileNotFoundError):
    """Raised when material file not found."""
    pass


class MaterialPort(Protocol):
    """Port for loading materials."""

    def load_materials(
        self,
        artifact_path: Path,
        materials: list[Material]
    ) -> list[Path]:
        """Load materials and return their absolute paths.

        Args:
            artifact_path: Path to the artifact directory
            materials: List of material specifications

        Returns:
            List of absolute paths to material files

        Raises:
            MaterialNotFoundError: If a material file cannot be found
        """
        ...


class LocalMaterialLoader:
    """Load materials from local filesystem."""

    def __init__(self, filesystem: FileSystemPort):
        self.filesystem = filesystem

    def load_materials(
        self,
        artifact_path: Path,
        materials: list[Material]
    ) -> list[Path]:
        """Resolve material paths relative to artifact.

        Args:
            artifact_path: Path to the artifact directory
            materials: List of material specifications

        Returns:
            List of absolute paths to material files

        Raises:
            MaterialNotFoundError: If a material file cannot be found
        """
        material_paths = []

        for material in materials:
            # Resolve path relative to artifact directory
            material_path = artifact_path / material.path

            if not self.filesystem.exists(material_path):
                raise MaterialNotFoundError(
                    f"Material not found: {material.path} (resolved to {material_path})"
                )

            material_paths.append(material_path.absolute())

        return material_paths
