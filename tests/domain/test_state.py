"""Tests for state domain models."""

from datetime import datetime

import pytest

from graft.domain.state import StateCache, StateQuery, StateResult


class TestStateCache:
    """Tests for StateCache."""

    def test_create_deterministic(self):
        """Test creating deterministic cache."""
        cache = StateCache(deterministic=True)
        assert cache.deterministic is True

    def test_create_non_deterministic(self):
        """Test creating non-deterministic cache."""
        cache = StateCache(deterministic=False)
        assert cache.deterministic is False

    def test_from_dict_with_deterministic(self):
        """Test creating StateCache from dict."""
        cache = StateCache.from_dict({"deterministic": True})
        assert cache.deterministic is True

    def test_from_dict_defaults_to_true(self):
        """Test StateCache defaults to deterministic=True."""
        cache = StateCache.from_dict({})
        assert cache.deterministic is True


class TestStateQuery:
    """Tests for StateQuery."""

    def test_create_query(self):
        """Test creating a state query."""
        cache = StateCache(deterministic=True)
        query = StateQuery(name="coverage", run="pytest --cov", cache=cache)

        assert query.name == "coverage"
        assert query.run == "pytest --cov"
        assert query.cache.deterministic is True

    def test_from_dict_minimal(self):
        """Test creating StateQuery from minimal dict."""
        query = StateQuery.from_dict(
            "coverage",
            {"run": "pytest --cov"},
        )

        assert query.name == "coverage"
        assert query.run == "pytest --cov"
        assert query.cache.deterministic is True  # default

    def test_from_dict_with_cache(self):
        """Test creating StateQuery with cache config."""
        query = StateQuery.from_dict(
            "test-results",
            {
                "run": "cat test-results.json",
                "cache": {"deterministic": True},
            },
        )

        assert query.name == "test-results"
        assert query.run == "cat test-results.json"
        assert query.cache.deterministic is True

    def test_from_dict_missing_run(self):
        """Test StateQuery requires 'run' field."""
        with pytest.raises(ValueError, match="missing required field 'run'"):
            StateQuery.from_dict("coverage", {})


class TestStateResult:
    """Tests for StateResult."""

    def test_create_result(self):
        """Test creating a state result."""
        now = datetime.now()
        result = StateResult(
            query_name="coverage",
            commit_hash="abc123",
            data={"percent_covered": 85.0},
            timestamp=now,
            command="pytest --cov",
            deterministic=True,
            cached=False,
        )

        assert result.query_name == "coverage"
        assert result.commit_hash == "abc123"
        assert result.data == {"percent_covered": 85.0}
        assert result.timestamp == now
        assert result.command == "pytest --cov"
        assert result.deterministic is True
        assert result.cached is False

    def test_to_cache_file(self):
        """Test serializing result to cache file format."""
        now = datetime.now()
        result = StateResult(
            query_name="coverage",
            commit_hash="abc123",
            data={"percent_covered": 85.0},
            timestamp=now,
            command="pytest --cov",
            deterministic=True,
        )

        cache_data = result.to_cache_file()

        assert cache_data == {
            "metadata": {
                "query_name": "coverage",
                "commit_hash": "abc123",
                "timestamp": now.isoformat(),
                "command": "pytest --cov",
                "deterministic": True,
            },
            "data": {"percent_covered": 85.0},
        }

    def test_from_cache_file(self):
        """Test loading result from cache file format."""
        now = datetime.now()
        cache_data = {
            "metadata": {
                "query_name": "coverage",
                "commit_hash": "abc123",
                "timestamp": now.isoformat(),
                "command": "pytest --cov",
                "deterministic": True,
            },
            "data": {"percent_covered": 85.0},
        }

        result = StateResult.from_cache_file(cache_data)

        assert result.query_name == "coverage"
        assert result.commit_hash == "abc123"
        assert result.data == {"percent_covered": 85.0}
        assert result.timestamp == now
        assert result.command == "pytest --cov"
        assert result.deterministic is True
        assert result.cached is True  # from_cache_file sets cached=True

    def test_roundtrip_cache_file(self):
        """Test result can roundtrip through cache file format."""
        now = datetime.now()
        original = StateResult(
            query_name="test",
            commit_hash="def456",
            data={"tests_passed": 42},
            timestamp=now,
            command="pytest",
            deterministic=True,
        )

        # Serialize and deserialize
        cache_data = original.to_cache_file()
        restored = StateResult.from_cache_file(cache_data)

        assert restored.query_name == original.query_name
        assert restored.commit_hash == original.commit_hash
        assert restored.data == original.data
        assert restored.timestamp == original.timestamp
        assert restored.command == original.command
        assert restored.deterministic == original.deterministic
