"""Orchestrator adapter for DVC integration."""
from __future__ import annotations
import subprocess
import tempfile
from pathlib import Path
from typing import Protocol
import yaml

from ..domain.orchestrator import DVCStage, SyncPlan, StagePlanItem, OrchestratorConfig
from ..domain.entities import Artifact, Derivation
from .filesystem import FileSystemPort


class OrchestratorPort(Protocol):
    """Port for orchestrator operations."""

    def get_repo_root(self) -> Path:
        """Get the git repository root."""
        ...

    def discover_artifacts(self, repo_root: Path, roots: list[str]) -> list[Artifact]:
        """Discover all graft artifacts under the specified roots."""
        ...

    def load_dvc_yaml(self, repo_root: Path) -> dict[str, dict]:
        """Load existing dvc.yaml or return empty dict if not present."""
        ...

    def build_stage(self, repo_root: Path, artifact: Artifact, derivation: Derivation) -> DVCStage:
        """Build a DVC stage for a single derivation."""
        ...

    def compute_plan(
        self,
        repo_root: Path,
        artifacts: list[Artifact],
        existing_stages: dict[str, dict],
        managed_prefix: str
    ) -> SyncPlan:
        """Compute the sync plan (create/update/remove)."""
        ...

    def apply_plan(self, repo_root: Path, plan: SyncPlan, existing_stages: dict[str, dict], managed_prefix: str) -> None:
        """Apply the sync plan by writing dvc.yaml atomically."""
        ...

    def is_dvc_available(self) -> bool:
        """Check if DVC is available (command and .dvc/ directory)."""
        ...


class DVCAdapter:
    """Adapter for DVC orchestration."""

    def __init__(self, fs: FileSystemPort, config_adapter):
        """Initialize with filesystem and config adapter."""
        self.fs = fs
        self.config_adapter = config_adapter

    def get_repo_root(self) -> Path:
        """Get the git repository root using git command."""
        try:
            result = subprocess.run(
                ["git", "rev-parse", "--show-toplevel"],
                capture_output=True,
                text=True,
                check=True
            )
            return Path(result.stdout.strip())
        except (subprocess.CalledProcessError, FileNotFoundError):
            # Fall back to current directory if not in a git repo
            return Path.cwd()

    def discover_artifacts(self, repo_root: Path, roots: list[str]) -> list[Artifact]:
        """Discover all graft artifacts under the specified roots."""
        artifacts = []
        for root_str in roots:
            root_path = (repo_root / root_str).resolve()
            if not root_path.exists():
                continue

            # Find all graft.yaml files under this root
            for graft_yaml in root_path.rglob("graft.yaml"):
                artifact_dir = graft_yaml.parent
                try:
                    artifact = self.config_adapter.load_artifact(artifact_dir)
                    artifacts.append(artifact)
                except Exception:
                    # Skip invalid artifacts
                    continue

        return artifacts

    def load_dvc_yaml(self, repo_root: Path) -> dict[str, dict]:
        """Load existing dvc.yaml or return empty dict if not present."""
        dvc_yaml_path = repo_root / "dvc.yaml"
        if not self.fs.exists(dvc_yaml_path):
            return {}

        try:
            content = self.fs.read_text(dvc_yaml_path)
            data = yaml.safe_load(content)
            if data is None:
                return {}
            return data.get("stages", {})
        except yaml.YAMLError:
            raise ValueError(f"Invalid YAML in {dvc_yaml_path}")

    def build_stage(self, repo_root: Path, artifact: Artifact, derivation: Derivation) -> DVCStage:
        """Build a DVC stage for a single derivation."""
        artifact_dir = artifact.path
        artifact_rel = artifact_dir.relative_to(repo_root).as_posix()

        # Stage name: graft:<artifact-name>:<derivation-id>
        stage_name = f"graft:{artifact.name}:{derivation.id}"

        # Command: graft run <artifact-dir> --id <derivation-id>
        cmd = f"graft run {artifact_rel} --id {derivation.id}"

        # Working directory
        wdir = artifact_rel

        # Dependencies
        deps = []

        # 1. All materials
        for material in artifact.config.inputs.materials:
            # Material path could be absolute or relative to artifact dir
            mat_path = Path(material.path)
            if mat_path.is_absolute():
                # Make relative to repo root
                dep_path = mat_path.relative_to(repo_root).as_posix()
            else:
                # Relative to artifact dir
                dep_path = (artifact_dir / mat_path).relative_to(repo_root).as_posix()
            deps.append(dep_path)

        # 2. graft.yaml
        graft_yaml_path = (artifact_dir / "graft.yaml").relative_to(repo_root).as_posix()
        deps.append(graft_yaml_path)

        # 3. Template file (if template.source == "file")
        if derivation.template and derivation.template.source == "file" and derivation.template.file:
            template_path = (artifact_dir / derivation.template.file).relative_to(repo_root).as_posix()
            deps.append(template_path)

        # 4. Dockerfile (if transformer.build present)
        if derivation.transformer.build:
            dockerfile_path = (artifact_dir / "Dockerfile").relative_to(repo_root).as_posix()
            deps.append(dockerfile_path)

        # Outputs
        outs = []
        for output in derivation.outputs:
            out_path = (artifact_dir / output.path).relative_to(repo_root).as_posix()
            outs.append(out_path)

        return DVCStage(
            name=stage_name,
            wdir=wdir,
            cmd=cmd,
            deps=sorted(set(deps)),  # Remove duplicates and sort
            outs=sorted(set(outs))
        )

    def compute_plan(
        self,
        repo_root: Path,
        artifacts: list[Artifact],
        existing_stages: dict[str, dict],
        managed_prefix: str
    ) -> SyncPlan:
        """Compute the sync plan (create/update/remove)."""
        create: list[StagePlanItem] = []
        update: list[StagePlanItem] = []
        remove: list[StagePlanItem] = []

        # Build expected stages from artifacts
        expected_stages: dict[str, DVCStage] = {}
        for artifact in artifacts:
            for derivation in artifact.config.derivations:
                stage = self.build_stage(repo_root, artifact, derivation)
                expected_stages[stage.name] = stage

        # Check for missing or mismatched stages
        for stage_name, expected_stage in expected_stages.items():
            if stage_name not in existing_stages:
                # Missing stage
                create.append(StagePlanItem(
                    stage_name=stage_name,
                    stage=expected_stage,
                    reason=f"Derivation exists but stage not found in dvc.yaml"
                ))
            else:
                # Check if stage matches
                existing = existing_stages[stage_name]
                if not self._stages_match(expected_stage, existing):
                    update.append(StagePlanItem(
                        stage_name=stage_name,
                        stage=expected_stage,
                        reason=f"Stage spec differs from canonical definition"
                    ))

        # Check for orphaned managed stages
        for stage_name in existing_stages:
            if stage_name.startswith(managed_prefix) and stage_name not in expected_stages:
                remove.append(StagePlanItem(
                    stage_name=stage_name,
                    stage=None,
                    reason=f"Managed stage exists but derivation no longer found"
                ))

        return SyncPlan(create=create, update=update, remove=remove)

    def _stages_match(self, expected: DVCStage, existing: dict) -> bool:
        """Check if an expected stage matches an existing stage dict."""
        expected_dict = expected.to_dict()
        return (
            existing.get("wdir") == expected_dict["wdir"]
            and existing.get("cmd") == expected_dict["cmd"]
            and existing.get("deps") == expected_dict["deps"]
            and existing.get("outs") == expected_dict["outs"]
        )

    def apply_plan(self, repo_root: Path, plan: SyncPlan, existing_stages: dict[str, dict], managed_prefix: str) -> None:
        """Apply the sync plan by writing dvc.yaml atomically."""
        # Start with existing stages
        new_stages = dict(existing_stages)

        # Apply removes
        for item in plan.remove:
            if item.stage_name in new_stages:
                del new_stages[item.stage_name]

        # Apply creates and updates
        for item in plan.create + plan.update:
            if item.stage:
                new_stages[item.stage_name] = item.stage.to_dict()

        # Write atomically using temp file + rename
        dvc_yaml_path = repo_root / "dvc.yaml"
        dvc_content = yaml.dump({"stages": new_stages}, sort_keys=False, default_flow_style=False)

        # Use temp file in same directory to ensure atomic rename works
        with tempfile.NamedTemporaryFile(
            mode='w',
            dir=repo_root,
            prefix='.dvc.yaml.',
            suffix='.tmp',
            delete=False,
            encoding='utf-8'
        ) as tf:
            tf.write(dvc_content)
            temp_path = Path(tf.name)

        try:
            # Atomic rename
            temp_path.rename(dvc_yaml_path)
        except Exception:
            # Clean up temp file on error
            temp_path.unlink(missing_ok=True)
            raise

    def is_dvc_available(self) -> bool:
        """Check if DVC is available (command and .dvc/ directory)."""
        try:
            # Check if dvc command exists
            subprocess.run(["dvc", "version"], capture_output=True, check=True)

            # Check if .dvc/ directory exists in repo root
            repo_root = self.get_repo_root()
            return (repo_root / ".dvc").is_dir()
        except (subprocess.CalledProcessError, FileNotFoundError):
            return False
