"""Integration tests for dependency resolution.

Tests the full flow using graft's own graft.yaml with real adapters.
"""

import os
from pathlib import Path

import pytest

from graft.adapters.filesystem import RealFileSystem
from graft.adapters.git import SubprocessGitOperations
from graft.domain.dependency import DependencyStatus
from graft.services import config_service, resolution_service
from graft.services.dependency_context import DependencyContext


@pytest.mark.integration
class TestResolveRealGraftYaml:
    """Integration tests using real graft.yaml."""

    def test_parse_real_graft_yaml(self) -> None:
        """Should successfully parse graft's own graft.yaml."""
        # Use real context
        ctx = DependencyContext(
            filesystem=RealFileSystem(),
            git=SubprocessGitOperations(),
            deps_directory="..",
        )

        # Find real graft.yaml
        repo_root = Path(__file__).parent.parent.parent
        config_path = str(repo_root / "graft.yaml")

        # Parse
        config = config_service.parse_graft_yaml(ctx, config_path)

        # Verify structure
        assert config.api_version == "graft/v0"
        assert config.has_dependency("python-starter")

        # Verify python-starter dependency
        dep = config.get_dependency("python-starter")
        assert dep.name == "python-starter"
        assert "python-starter.git" in dep.git_url.url
        assert dep.git_ref.ref == "main"

    @pytest.mark.skipif(
        os.getenv("CI") == "true",
        reason="Skip actual git operations in CI",
    )
    def test_resolve_real_dependencies(self) -> None:
        """Should resolve graft's real dependencies.

        Note: This test actually clones repositories.
        Skipped in CI to avoid network dependencies.
        """
        # Use real context
        ctx = DependencyContext(
            filesystem=RealFileSystem(),
            git=SubprocessGitOperations(),
            deps_directory="../",
        )

        # Parse config
        repo_root = Path(__file__).parent.parent.parent
        config_path = str(repo_root / "graft.yaml")
        config = config_service.parse_graft_yaml(ctx, config_path)

        # Resolve
        resolutions = resolution_service.resolve_all_dependencies(ctx, config)

        # Verify all resolved
        for resolution in resolutions:
            assert resolution.status == DependencyStatus.RESOLVED
            assert resolution.local_path
            assert Path(resolution.local_path).exists()
