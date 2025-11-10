"""Domain entities for Graft."""
from __future__ import annotations
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


@dataclass(frozen=True)
class Material:
    """A source material input."""
    path: str
    rev: str = "HEAD"


@dataclass(frozen=True)
class Output:
    """A derivation output specification."""
    path: str
    schema: str | None = None


@dataclass(frozen=True)
class Template:
    """Template specification for a derivation."""
    source: str
    engine: str
    content_type: str
    file: str | None = None
    persist: str | None = None
    persist_path: str | None = None


@dataclass(frozen=True)
class Policy:
    """Policy configuration."""
    deterministic: bool = True
    network: str = "off"
    attest: str = "required"
    direct_edit: bool = False


@dataclass(frozen=True)
class Derivation:
    """A single derivation within an artifact."""
    id: str
    transformer: dict[str, Any]
    outputs: list[Output]
    template: Template | None = None
    policy: Policy | None = None


@dataclass(frozen=True)
class Inputs:
    """Inputs specification for an artifact."""
    materials: list[Material] = field(default_factory=list)


@dataclass(frozen=True)
class GraftConfig:
    """Complete graft.yaml configuration."""
    graft: str
    derivations: list[Derivation]
    inputs: Inputs = field(default_factory=lambda: Inputs())
    policy: Policy = field(default_factory=lambda: Policy())


@dataclass(frozen=True)
class Artifact:
    """A Graft artifact - the core domain entity."""
    path: Path
    config: GraftConfig

    @property
    def name(self) -> str:
        """Return the artifact name."""
        return self.config.graft
