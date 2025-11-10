"""Explain service for showing artifact configuration."""
from __future__ import annotations
from dataclasses import dataclass
from pathlib import Path

from ..domain.entities import Artifact
from ..adapters.config import ConfigAdapter


@dataclass
class ExplainResult:
    """Result of explaining an artifact.

    Contract: Returns { artifact, graft, policy?, inputs?, derivations[] }
    where derivations is an array of full derivation objects.
    """
    artifact: str
    graft: str
    policy: dict
    inputs: dict
    derivations: list[dict]

    def to_dict(self) -> dict:
        """Convert to dictionary for JSON output."""
        return {
            "artifact": self.artifact,
            "graft": self.graft,
            "policy": self.policy,
            "inputs": self.inputs,
            "derivations": self.derivations,
        }


class ExplainService:
    """Service for explaining artifact configuration."""

    def __init__(self, config_adapter: ConfigAdapter):
        self.config_adapter = config_adapter

    def explain(self, artifact_path: Path) -> ExplainResult:
        """
        Explain the configuration for a given artifact.

        Args:
            artifact_path: Path to the artifact directory

        Returns:
            ExplainResult containing the artifact configuration

        Raises:
            FileNotFoundError: If graft.yaml is not found
        """
        artifact = self.config_adapter.load_artifact(artifact_path)

        return ExplainResult(
            artifact=str(artifact.path),
            graft=artifact.config.graft,
            policy=self._policy_to_dict(artifact.config.policy),
            inputs=self._inputs_to_dict(artifact.config.inputs),
            derivations=[self._derivation_to_dict(d) for d in artifact.config.derivations],
        )

    def _policy_to_dict(self, policy) -> dict:
        """Convert policy entity to dict."""
        if not policy:
            return {}
        return {
            "deterministic": policy.deterministic,
            "network": policy.network,
            "attest": policy.attest,
            "direct_edit": policy.direct_edit,
        }

    def _inputs_to_dict(self, inputs) -> dict:
        """Convert inputs entity to dict."""
        if not inputs or not inputs.materials:
            return {}
        return {
            "materials": [
                {"path": m.path, "rev": m.rev}
                for m in inputs.materials
            ]
        }

    def _derivation_to_dict(self, derivation) -> dict:
        """Convert derivation entity to dict."""
        result = {
            "id": derivation.id,
            "transformer": self._transformer_to_dict(derivation.transformer),
            "outputs": [self._output_to_dict(o) for o in derivation.outputs],
        }

        # Optional fields
        if derivation.template:
            result["template"] = self._template_to_dict(derivation.template)

        if derivation.policy:
            result["policy"] = self._policy_to_dict(derivation.policy)

        return result

    def _transformer_to_dict(self, transformer) -> dict:
        """Convert transformer entity to dict."""
        result = {}

        if transformer.build:
            result["build"] = {
                "image": transformer.build.image,
                "context": transformer.build.context,
            }

        if transformer.ref:
            result["ref"] = transformer.ref

        if transformer.params:
            result["params"] = transformer.params

        return result

    def _output_to_dict(self, output) -> dict:
        """Convert output entity to dict."""
        result = {"path": output.path}
        if output.schema:
            result["schema"] = output.schema
        return result

    def _template_to_dict(self, template) -> dict:
        """Convert template entity to dict."""
        result = {
            "source": template.source,
            "engine": template.engine,
            "content_type": template.content_type,
            "file": template.file,
        }
        if template.persist:
            result["persist"] = template.persist
        if template.persist_path:
            result["persist_path"] = template.persist_path
        return result
