"""Configuration adapter for parsing graft.yaml files."""
from __future__ import annotations
from pathlib import Path
from typing import Any
import yaml

from ..domain.entities import (
    Artifact,
    GraftConfig,
    Derivation,
    Inputs,
    Material,
    Output,
    Policy,
    Template,
    Transformer,
    TransformerBuild,
)
from ..domain.orchestrator import OrchestratorConfig, SyncPolicy
from .filesystem import FileSystemPort


class ConfigAdapter:
    """Adapter for loading and parsing graft configuration files."""

    def __init__(self, fs: FileSystemPort):
        self.fs = fs

    def load_artifact(self, artifact_path: Path) -> Artifact:
        """Load an artifact from a directory containing graft.yaml."""
        config_path = artifact_path / "graft.yaml"

        if not self.fs.exists(config_path):
            raise FileNotFoundError(f"No graft.yaml found at {config_path}")

        raw_yaml = self.fs.read_text(config_path)
        raw_config = yaml.safe_load(raw_yaml)

        config = self._parse_config(raw_config)
        return Artifact(path=artifact_path, config=config)

    def _parse_config(self, raw: dict[str, Any]) -> GraftConfig:
        """Parse raw YAML dict into GraftConfig entity."""
        return GraftConfig(
            graft=raw["graft"],
            derivations=self._parse_derivations(raw.get("derivations", [])),
            inputs=self._parse_inputs(raw.get("inputs", {})),
            policy=self._parse_policy(raw.get("policy", {})),
        )

    def _parse_inputs(self, raw: dict[str, Any]) -> Inputs:
        """Parse inputs section."""
        materials = [
            Material(path=m["path"], rev=m.get("rev", "HEAD"))
            for m in raw.get("materials", [])
        ]
        return Inputs(materials=materials)

    def _parse_derivations(self, raw_list: list[dict[str, Any]]) -> list[Derivation]:
        """Parse derivations list."""
        return [self._parse_derivation(d) for d in raw_list]

    def _parse_derivation(self, raw: dict[str, Any]) -> Derivation:
        """Parse a single derivation."""
        return Derivation(
            id=raw["id"],
            transformer=self._parse_transformer(raw.get("transformer", {})),
            outputs=[Output(path=o["path"], schema=o.get("schema")) for o in raw.get("outputs", [])],
            template=self._parse_template(raw.get("template")) if "template" in raw else None,
            policy=self._parse_policy(raw.get("policy", {})) if "policy" in raw else None,
        )

    def _parse_transformer(self, raw: dict[str, Any]) -> Transformer:
        """Parse transformer specification."""
        build = None
        if "build" in raw:
            build_raw = raw["build"]
            build = TransformerBuild(
                image=build_raw["image"],
                context=build_raw.get("context", ".")
            )

        return Transformer(
            build=build,
            ref=raw.get("ref"),
            params=raw.get("params", {})
        )

    def _parse_template(self, raw: dict[str, Any]) -> Template:
        """Parse template specification."""
        return Template(
            source=raw.get("source", ""),
            engine=raw.get("engine", ""),
            content_type=raw.get("content_type", ""),
            file=raw.get("file", ""),
            persist=raw.get("persist"),
            persist_path=raw.get("persist_path"),
        )

    def _parse_policy(self, raw: dict[str, Any]) -> Policy:
        """Parse policy configuration."""
        return Policy(
            deterministic=raw.get("deterministic", True),
            network=raw.get("network", "off"),
            attest=raw.get("attest", "required"),
            direct_edit=raw.get("direct_edit", False),
        )

    def load_root_config(self, repo_root: Path) -> OrchestratorConfig:
        """Load graft.config.yaml from repo root."""
        config_path = repo_root / "graft.config.yaml"

        # Return defaults if config doesn't exist
        if not self.fs.exists(config_path):
            return OrchestratorConfig()

        try:
            raw_yaml = self.fs.read_text(config_path)
            raw_config = yaml.safe_load(raw_yaml)

            if not raw_config or "orchestrator" not in raw_config:
                return OrchestratorConfig()

            orch_raw = raw_config["orchestrator"]
            return self._parse_orchestrator_config(orch_raw)
        except Exception:
            # Return defaults on any parsing error
            return OrchestratorConfig()

    def _parse_orchestrator_config(self, raw: dict[str, Any]) -> OrchestratorConfig:
        """Parse orchestrator configuration."""
        sync_policy_str = raw.get("sync_policy", "apply")
        try:
            sync_policy = SyncPolicy(sync_policy_str)
        except ValueError:
            sync_policy = SyncPolicy.APPLY

        return OrchestratorConfig(
            type=raw.get("type", "dvc"),
            managed_stage_prefix=raw.get("managed_stage_prefix", "graft:"),
            sync_policy=sync_policy,
            roots=raw.get("roots", ["."]),
        )
