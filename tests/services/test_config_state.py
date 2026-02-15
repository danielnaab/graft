"""Tests for state query parsing in config service."""

import pytest

from graft.domain.exceptions import ConfigValidationError
from graft.services.config_service import parse_graft_yaml
from graft.services.dependency_context import DependencyContext


class TestStateQueryParsing:
    """Tests for parsing state queries from graft.yaml."""

    def test_parse_state_queries_minimal(self, dependency_context: DependencyContext):
        """Test parsing minimal state query."""
        dependency_context.filesystem.create_file(
            "graft.yaml",
            """
apiVersion: graft/v0

state:
  coverage:
    run: "pytest --cov --cov-report=json | jq '.totals.percent_covered'"
""",
        )

        config = parse_graft_yaml(dependency_context, "graft.yaml")

        assert config.has_state_query("coverage")
        query = config.get_state_query("coverage")
        assert query.name == "coverage"
        assert query.run == "pytest --cov --cov-report=json | jq '.totals.percent_covered'"
        assert query.cache.deterministic is True  # default

    def test_parse_state_queries_with_cache(self, dependency_context: DependencyContext):
        """Test parsing state query with cache configuration."""
        dependency_context.filesystem.create_file(
            "graft.yaml",
            """
apiVersion: graft/v0

state:
  test-results:
    run: "cat test-results.json"
    cache:
      deterministic: true
""",
        )

        config = parse_graft_yaml(dependency_context, "graft.yaml")

        query = config.get_state_query("test-results")
        assert query.name == "test-results"
        assert query.run == "cat test-results.json"
        assert query.cache.deterministic is True

    def test_parse_multiple_state_queries(self, dependency_context: DependencyContext):
        """Test parsing multiple state queries."""
        dependency_context.filesystem.create_file(
            "graft.yaml",
            """
apiVersion: graft/v0

state:
  coverage:
    run: "pytest --cov"
    cache:
      deterministic: true

  test-results:
    run: "cat test-results.json"
    cache:
      deterministic: true

  lint-report:
    run: "ruff check --output-format=json"
    cache:
      deterministic: true
""",
        )

        config = parse_graft_yaml(dependency_context, "graft.yaml")

        assert len(config.state) == 3
        assert config.has_state_query("coverage")
        assert config.has_state_query("test-results")
        assert config.has_state_query("lint-report")

    def test_parse_config_without_state(self, dependency_context: DependencyContext):
        """Test parsing config without state section."""
        dependency_context.filesystem.create_file(
            "graft.yaml",
            """
apiVersion: graft/v0

commands:
  test:
    run: "pytest"
""",
        )

        config = parse_graft_yaml(dependency_context, "graft.yaml")

        assert len(config.state) == 0
        assert not config.has_state_query("coverage")

    def test_parse_state_missing_run(self, dependency_context: DependencyContext):
        """Test state query must have 'run' field."""
        dependency_context.filesystem.create_file(
            "graft.yaml",
            """
apiVersion: graft/v0

state:
  coverage:
    cache:
      deterministic: true
""",
        )

        with pytest.raises(
            ConfigValidationError,
            match="State query 'coverage' missing required field 'run'",
        ):
            parse_graft_yaml(dependency_context, "graft.yaml")

    def test_parse_state_not_dict(self, dependency_context: DependencyContext):
        """Test state section must be a dict."""
        dependency_context.filesystem.create_file(
            "graft.yaml",
            """
apiVersion: graft/v0

state:
  - coverage
  - test-results
""",
        )

        with pytest.raises(
            ConfigValidationError,
            match="state.*Must be a mapping/dict",
        ):
            parse_graft_yaml(dependency_context, "graft.yaml")

    def test_parse_state_query_not_dict(self, dependency_context: DependencyContext):
        """Test individual state query must be a dict."""
        dependency_context.filesystem.create_file(
            "graft.yaml",
            """
apiVersion: graft/v0

state:
  coverage: "pytest --cov"
""",
        )

        with pytest.raises(
            ConfigValidationError,
            match="state.coverage.*must be a mapping/dict",
        ):
            parse_graft_yaml(dependency_context, "graft.yaml")

    def test_get_state_query_not_found(self, dependency_context: DependencyContext):
        """Test getting non-existent state query raises KeyError."""
        dependency_context.filesystem.create_file(
            "graft.yaml",
            """
apiVersion: graft/v0

state:
  coverage:
    run: "pytest --cov"
""",
        )

        config = parse_graft_yaml(dependency_context, "graft.yaml")

        with pytest.raises(KeyError):
            config.get_state_query("nonexistent")

    def test_has_state_query(self, dependency_context: DependencyContext):
        """Test checking if state query exists."""
        dependency_context.filesystem.create_file(
            "graft.yaml",
            """
apiVersion: graft/v0

state:
  coverage:
    run: "pytest --cov"
""",
        )

        config = parse_graft_yaml(dependency_context, "graft.yaml")

        assert config.has_state_query("coverage") is True
        assert config.has_state_query("nonexistent") is False
