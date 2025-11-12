"""Orchestrator service for managing DVC integration."""
from __future__ import annotations
from dataclasses import dataclass
from pathlib import Path

from ..domain.orchestrator import (
    OrchestratorConfig,
    OrchestratorStatus,
    SyncPolicy,
    SyncPlan,
)
from ..adapters.orchestrator import OrchestratorPort


class OrchestratorError(Exception):
    """Base exception for orchestrator errors."""
    pass


class DriftEnforcedError(OrchestratorError):
    """Raised when drift is detected in enforce mode."""
    def __init__(self, plan: SyncPlan):
        self.plan = plan
        super().__init__("Drift detected in enforce mode")


class InvalidDVCYamlError(OrchestratorError):
    """Raised when dvc.yaml is invalid or cannot be parsed."""
    pass


@dataclass
class AutosyncResult:
    """Result of an autosync operation."""
    status: OrchestratorStatus
    summary: str  # Human-readable summary for display

    def to_dict(self) -> dict:
        """Convert to dict for JSON serialization."""
        return {
            "orchestrator": self.status.to_dict(),
            "summary": self.summary
        }


class OrchestratorService:
    """Service for orchestrator operations."""

    def __init__(self, adapter: OrchestratorPort, config: OrchestratorConfig):
        """Initialize with adapter and configuration."""
        self.adapter = adapter
        self.config = config

    def autosync(self, sync_policy: SyncPolicy | None = None) -> AutosyncResult:
        """
        Perform autosync with the specified policy.

        Args:
            sync_policy: Override the default sync policy from config

        Returns:
            AutosyncResult with status and summary

        Raises:
            DriftEnforcedError: If enforce mode and drift exists
            InvalidDVCYamlError: If dvc.yaml cannot be parsed
        """
        effective_policy = sync_policy or self.config.sync_policy

        # Check if DVC is available
        if not self.adapter.is_dvc_available():
            # Graceful degradation: degrade to warn
            plan = SyncPlan()  # Empty plan
            status = OrchestratorStatus(
                type=self.config.type,
                sync_policy=SyncPolicy.WARN,
                drift=plan.drift_status,
                plan=plan,
                applied=False
            )
            summary = "DVC not available (degrade to warn mode). Run 'dvc init' to enable orchestration."
            return AutosyncResult(status=status, summary=summary)

        # Get repo root
        repo_root = self.adapter.get_repo_root()

        # Discover all artifacts
        artifacts = self.adapter.discover_artifacts(repo_root, self.config.roots)

        # Load existing dvc.yaml
        try:
            existing_stages = self.adapter.load_dvc_yaml(repo_root)
        except ValueError as e:
            raise InvalidDVCYamlError(str(e))

        # Compute plan
        plan = self.adapter.compute_plan(
            repo_root,
            artifacts,
            existing_stages,
            self.config.managed_stage_prefix
        )

        # Apply based on policy
        applied = False
        summary = ""

        if plan.has_drift:
            if effective_policy == SyncPolicy.OFF:
                summary = f"Drift detected (off mode): {self._format_plan_counts(plan)}"
            elif effective_policy == SyncPolicy.WARN:
                summary = f"Drift detected (warn mode): {self._format_plan_counts(plan)}"
            elif effective_policy == SyncPolicy.APPLY:
                self.adapter.apply_plan(repo_root, plan, existing_stages, self.config.managed_stage_prefix)
                applied = True
                summary = f"Autosync: {self._format_plan_counts(plan)}"
            elif effective_policy == SyncPolicy.ENFORCE:
                raise DriftEnforcedError(plan)
        else:
            summary = "No drift detected"

        status = OrchestratorStatus(
            type=self.config.type,
            sync_policy=effective_policy,
            drift=plan.drift_status,
            plan=plan,
            applied=applied
        )

        return AutosyncResult(status=status, summary=summary)

    def scaffold(self, check_only: bool = False) -> AutosyncResult:
        """
        Scaffold dvc.yaml (explicit write/check entrypoint).

        This method bypasses DVC availability checks since the whole point
        of scaffolding is to create dvc.yaml before DVC is initialized.

        Args:
            check_only: If True, only check for drift without writing

        Returns:
            AutosyncResult with status and summary

        Raises:
            InvalidDVCYamlError: If dvc.yaml cannot be parsed
        """
        effective_policy = SyncPolicy.WARN if check_only else SyncPolicy.APPLY

        # Get repo root
        repo_root = self.adapter.get_repo_root()

        # Discover all artifacts
        artifacts = self.adapter.discover_artifacts(repo_root, self.config.roots)

        # Load existing dvc.yaml
        try:
            existing_stages = self.adapter.load_dvc_yaml(repo_root)
        except ValueError as e:
            raise InvalidDVCYamlError(str(e))

        # Compute plan
        plan = self.adapter.compute_plan(
            repo_root,
            artifacts,
            existing_stages,
            self.config.managed_stage_prefix
        )

        # Apply based on policy
        applied = False
        summary = ""

        if plan.has_drift:
            if effective_policy == SyncPolicy.WARN:
                summary = f"Drift detected: {self._format_plan_counts(plan)}"
            elif effective_policy == SyncPolicy.APPLY:
                self.adapter.apply_plan(repo_root, plan, existing_stages, self.config.managed_stage_prefix)
                applied = True
                summary = f"Scaffolded: {self._format_plan_counts(plan)}"
        else:
            summary = "No drift detected"

        status = OrchestratorStatus(
            type=self.config.type,
            sync_policy=effective_policy,
            drift=plan.drift_status,
            plan=plan,
            applied=applied
        )

        return AutosyncResult(status=status, summary=summary)

    def _format_plan_counts(self, plan: SyncPlan) -> str:
        """Format plan counts for human-readable output."""
        parts = []
        if plan.create:
            parts.append(f"create={len(plan.create)}")
        if plan.update:
            parts.append(f"update={len(plan.update)}")
        if plan.remove:
            parts.append(f"remove={len(plan.remove)}")
        return ", ".join(parts) if parts else "no changes"
