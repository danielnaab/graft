"""Run service for executing derivations."""
from __future__ import annotations
from dataclasses import dataclass
import json
from pathlib import Path
from typing import Optional

from jinja2 import Template, TemplateError

from ..domain.entities import Artifact, Derivation
from ..adapters.config import ConfigAdapter
from ..adapters.filesystem import FileSystemPort
from ..adapters.docker import ContainerPort, BuildError, TransformerExecutionError
from ..adapters.materials import MaterialPort, MaterialNotFoundError


@dataclass
class RunResult:
    """Result of running derivations.

    Tracks which derivations were executed and their outputs.
    """
    artifact: str
    derivations_run: list[str]
    outputs_created: list[str]

    def to_dict(self) -> dict:
        """Convert to dictionary for JSON output."""
        return {
            "artifact": self.artifact,
            "derivations_run": self.derivations_run,
            "outputs_created": self.outputs_created,
        }


class TemplateNotFoundError(FileNotFoundError):
    """Raised when a template file cannot be found."""
    pass


class TemplateRenderError(Exception):
    """Raised when template rendering fails."""
    pass


class OutputMissingError(Exception):
    """Raised when declared output not created."""
    pass


class RunService:
    """Service for running artifact derivations."""

    def __init__(
        self,
        config_adapter: ConfigAdapter,
        filesystem: FileSystemPort,
        material_loader: MaterialPort,
        container_adapter: ContainerPort
    ):
        self.config_adapter = config_adapter
        self.filesystem = filesystem
        self.material_loader = material_loader
        self.container_adapter = container_adapter

    def run(
        self,
        artifact_path: Path,
        derivation_id: Optional[str] = None
    ) -> RunResult:
        """
        Execute derivations for an artifact.

        Args:
            artifact_path: Path to the artifact directory
            derivation_id: Optional derivation ID to run (runs all if None)

        Returns:
            RunResult with execution details

        Raises:
            FileNotFoundError: If graft.yaml or template file not found
            TemplateNotFoundError: If template file not found
            TemplateRenderError: If template rendering fails
        """
        # Load artifact configuration
        artifact = self.config_adapter.load_artifact(artifact_path)

        # Filter derivations if ID specified
        derivations = self._filter_derivations(artifact, derivation_id)

        # Execute each derivation
        derivations_run = []
        outputs_created = []

        for derivation in derivations:
            outputs = self._execute_derivation(artifact, derivation)
            derivations_run.append(derivation.id)
            outputs_created.extend(outputs)

        return RunResult(
            artifact=str(artifact.path),
            derivations_run=derivations_run,
            outputs_created=outputs_created,
        )

    def _filter_derivations(
        self,
        artifact: Artifact,
        derivation_id: Optional[str]
    ) -> list[Derivation]:
        """Filter derivations by ID if specified."""
        if derivation_id is None:
            return list(artifact.config.derivations)

        return [
            d for d in artifact.config.derivations
            if d.id == derivation_id
        ]

    def _execute_derivation(
        self,
        artifact: Artifact,
        derivation: Derivation
    ) -> list[str]:
        """
        Execute a single derivation.

        Returns:
            List of output file paths created
        """
        # Case 1: Template-based derivation (Slice 1)
        if derivation.template and derivation.template.file and derivation.template.engine != "none":
            return self._execute_template_derivation(artifact, derivation)

        # Case 2: Container-based derivation (Slice 2)
        if derivation.transformer.build:
            return self._execute_container_derivation(artifact, derivation)

        # Case 3: Neither (skip - future functionality)
        return []

    def _execute_template_derivation(
        self,
        artifact: Artifact,
        derivation: Derivation
    ) -> list[str]:
        """Execute template-based derivation (Slice 1).

        Returns:
            List of output file paths created
        """
        # Read and render template
        template_path = artifact.path / derivation.template.file
        rendered_content = self._render_template(template_path)

        # Write to all output paths
        output_paths = []
        for output in derivation.outputs:
            output_path = artifact.path / output.path
            self._write_output(output_path, rendered_content)
            output_paths.append(output.path)

        return output_paths

    def _execute_container_derivation(
        self,
        artifact: Artifact,
        derivation: Derivation
    ) -> list[str]:
        """Execute container-based transformation (Slice 2).

        Returns:
            List of output file paths created

        Raises:
            FileNotFoundError: If Dockerfile not found
            MaterialNotFoundError: If material not found
            BuildError: If Docker build fails
            TransformerExecutionError: If container execution fails
            OutputMissingError: If declared output not created
        """
        build_spec = derivation.transformer.build

        # 1. Build Docker image
        dockerfile_path = artifact.path / build_spec.context / "Dockerfile"
        context_path = artifact.path / build_spec.context

        if not self.filesystem.exists(dockerfile_path):
            raise FileNotFoundError(
                f"Dockerfile not found: {dockerfile_path.relative_to(artifact.path)}"
            )

        self.container_adapter.build_image(
            dockerfile_path=dockerfile_path,
            image_tag=build_spec.image,
            context_path=context_path
        )

        # 2. Load materials and determine mount point
        material_paths = []
        if artifact.config.inputs and artifact.config.inputs.materials:
            material_paths = self.material_loader.load_materials(
                artifact.path,
                artifact.config.inputs.materials
            )

        # 3. Find common parent for mounting
        # We need to mount a directory that contains both the artifact and all materials
        # Resolve paths to normalize them (remove .. components)
        artifact_abs = artifact.path.absolute().resolve()
        material_abs = [p.resolve() for p in material_paths]
        all_paths = [artifact_abs] + material_abs
        mount_root = self._find_common_parent(all_paths)

        # Calculate relative paths from mount root
        artifact_rel = artifact_abs.relative_to(mount_root)

        # Convert absolute material paths to container paths
        material_container_paths = []
        if material_abs:
            for mat_path in material_abs:
                mat_rel = mat_path.relative_to(mount_root)
                material_container_paths.append(f"/workspace/{mat_rel}")
        else:
            # No materials - use empty list
            material_container_paths = []

        # 4. Prepare environment variables
        env_vars = {
            "GRAFT_ARTIFACT_DIR": f"/workspace/{artifact_rel}",
            "GRAFT_PARAMS": json.dumps(derivation.transformer.params),
            "GRAFT_OUTPUTS": json.dumps([
                f"/workspace/{artifact_rel}/{output.path}" for output in derivation.outputs
            ]),
            "GRAFT_MATERIALS": json.dumps(material_container_paths)
        }

        # 5. Run container
        exit_code, stdout, stderr = self.container_adapter.run_container(
            image_tag=build_spec.image,
            working_dir=mount_root,
            env_vars=env_vars
        )

        if exit_code != 0:
            raise TransformerExecutionError(
                f"Container execution failed (exit {exit_code}): {stderr}"
            )

        # 5. Validate outputs
        output_paths = []
        for output in derivation.outputs:
            output_path = artifact.path / output.path
            if not self.filesystem.exists(output_path):
                raise OutputMissingError(
                    f"Output not created by transformer: {output.path}"
                )
            output_paths.append(output.path)

        return output_paths

    def _find_common_parent(self, paths: list[Path]) -> Path:
        """Find the common parent directory for all given paths.

        Args:
            paths: List of absolute paths

        Returns:
            Common parent directory
        """
        if not paths:
            return Path.cwd()

        # Start with the first path's parents
        common = paths[0].absolute()

        # Find the deepest directory that contains all paths
        for path in paths[1:]:
            path = path.absolute()
            # Find common parent between current common and this path
            while not path.is_relative_to(common):
                common = common.parent

        return common

    def _render_template(self, template_path: Path) -> str:
        """
        Render a Jinja2 template.

        For Slice 1: Renders with empty context (no variable substitution).
        Future slices will add context loading and variable support.

        Args:
            template_path: Path to the template file

        Returns:
            Rendered template content as string

        Raises:
            TemplateNotFoundError: If template file doesn't exist
            TemplateRenderError: If rendering fails
        """
        if not self.filesystem.exists(template_path):
            raise TemplateNotFoundError(
                f"Template file not found: {template_path}"
            )

        try:
            template_source = self.filesystem.read_text(template_path)
            template = Template(template_source)

            # Slice 1: Render with empty context
            # Future: Load context from materials/inputs
            context = {}
            rendered = template.render(context)

            return rendered
        except TemplateError as e:
            raise TemplateRenderError(
                f"Failed to render template {template_path}: {e}"
            ) from e

    def _write_output(self, output_path: Path, content: str) -> None:
        """
        Write rendered content to output file.

        Creates parent directories if needed.

        Args:
            output_path: Path to the output file
            content: Rendered content to write
        """
        # Create parent directories if needed
        if not self.filesystem.exists(output_path.parent):
            self.filesystem.mkdir(output_path.parent, parents=True, exist_ok=True)

        # Write content
        self.filesystem.write_text(output_path, content)
