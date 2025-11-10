"""Run service for executing derivations."""
from __future__ import annotations
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

from jinja2 import Template, TemplateError

from ..domain.entities import Artifact, Derivation
from ..adapters.config import ConfigAdapter
from ..adapters.filesystem import FileSystemPort


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


class RunService:
    """Service for running artifact derivations."""

    def __init__(self, config_adapter: ConfigAdapter, filesystem: FileSystemPort):
        self.config_adapter = config_adapter
        self.filesystem = filesystem

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
        # Skip derivations without templates or with inline/none engines
        # These are stubs for future functionality beyond Slice 1
        if derivation.template is None:
            return []

        if derivation.template.file is None or derivation.template.engine == "none":
            # Skip inline templates and non-templated derivations
            return []

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
