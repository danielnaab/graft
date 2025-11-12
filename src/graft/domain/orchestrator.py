"""Domain entities for orchestrator integration."""
from __future__ import annotations
from dataclasses import dataclass, field
from enum import Enum
from typing import Any


class SyncPolicy(str, Enum):
    """Orchestrator sync policy."""
    OFF = "off"           # Never write; show plan if drift exists
    WARN = "warn"         # Never write; show plan; exit 0
    APPLY = "apply"       # Write the plan automatically
    ENFORCE = "enforce"   # Fail if drift exists; no write


class DriftStatus(str, Enum):
    """Drift detection status."""
    NONE = "none"                  # No drift detected
    MISSING_STAGES = "missing_stages"  # Derivations exist without stages
    EXTRA_STAGES = "extra_stages"      # Managed stages without derivations
    MIXED = "mixed"                    # Both missing and extra stages


@dataclass(frozen=True)
class OrchestratorConfig:
    """Orchestrator configuration from graft.config.yaml."""
    type: str = "dvc"
    managed_stage_prefix: str = "graft:"
    sync_policy: SyncPolicy = SyncPolicy.APPLY
    roots: list[str] = field(default_factory=lambda: ["."])


@dataclass(frozen=True)
class DVCStage:
    """A DVC stage specification."""
    name: str          # e.g., "graft:sprint-brief:default"
    wdir: str          # Working directory (relative to repo root)
    cmd: str           # Command to execute
    deps: list[str]    # Dependencies (relative to repo root)
    outs: list[str]    # Outputs (relative to repo root)

    def to_dict(self) -> dict[str, Any]:
        """Convert to dict for YAML serialization."""
        return {
            "wdir": self.wdir,
            "cmd": self.cmd,
            "deps": self.deps,
            "outs": self.outs,
        }


@dataclass(frozen=True)
class StagePlanItem:
    """A single stage in a sync plan."""
    stage_name: str
    stage: DVCStage | None  # None for removals
    reason: str  # Human-readable reason for this action

    def to_dict(self) -> dict[str, Any]:
        """Convert to dict for JSON serialization."""
        result: dict[str, Any] = {
            "stage_name": self.stage_name,
            "reason": self.reason,
        }
        if self.stage:
            result["stage"] = self.stage.to_dict()
        return result


@dataclass(frozen=True)
class SyncPlan:
    """Plan for syncing dvc.yaml with derivations."""
    create: list[StagePlanItem] = field(default_factory=list)
    update: list[StagePlanItem] = field(default_factory=list)
    remove: list[StagePlanItem] = field(default_factory=list)

    @property
    def has_drift(self) -> bool:
        """Check if there is any drift."""
        return bool(self.create or self.update or self.remove)

    @property
    def drift_status(self) -> DriftStatus:
        """Determine drift status."""
        has_missing = bool(self.create or self.update)
        has_extra = bool(self.remove)

        if has_missing and has_extra:
            return DriftStatus.MIXED
        elif has_missing:
            return DriftStatus.MISSING_STAGES
        elif has_extra:
            return DriftStatus.EXTRA_STAGES
        else:
            return DriftStatus.NONE

    def to_dict(self) -> dict[str, Any]:
        """Convert to dict for JSON serialization."""
        return {
            "create": [item.to_dict() for item in self.create],
            "update": [item.to_dict() for item in self.update],
            "remove": [item.to_dict() for item in self.remove],
        }


@dataclass(frozen=True)
class OrchestratorStatus:
    """Result of orchestrator status check."""
    type: str
    sync_policy: SyncPolicy
    drift: DriftStatus
    plan: SyncPlan
    applied: bool

    def to_dict(self) -> dict[str, Any]:
        """Convert to dict for JSON serialization."""
        return {
            "type": self.type,
            "sync_policy": self.sync_policy.value,
            "drift": self.drift.value,
            "plan": self.plan.to_dict(),
            "applied": self.applied,
        }
