"""Unit tests for validation service.

Tests validation logic for graft.yaml and graft.lock files.
"""

from datetime import datetime, timezone

import pytest

from graft.domain.change import Change
from graft.domain.command import Command
from graft.domain.config import GraftConfig
from graft.domain.dependency import DependencySpec, GitRef, GitUrl
from graft.domain.lock_entry import LockEntry
from graft.services import validation_service


class TestValidateConfigSchema:
    """Tests for validate_config_schema function.

    Note: GraftConfig already validates in __post_init__, so these tests
    verify that the validation service provides additional/consistent validation.
    """

    def test_valid_config_returns_no_errors(self):
        """Should return empty list for valid configuration."""
        config = GraftConfig(
            api_version="graft/v0",
            dependencies={
                "test-dep": DependencySpec(
                    name="test-dep",
                    git_url=GitUrl("https://github.com/test/repo.git"),
                    git_ref=GitRef("main"),
                )
            },
            changes={},
            commands={},
            metadata={},
        )

        errors = validation_service.validate_config_schema(config)

        assert len(errors) == 0

    def test_no_dependencies(self):
        """Should error when no dependencies are defined."""
        config = GraftConfig(
            api_version="graft/v0",
            dependencies={},
            changes={},
            commands={},
            metadata={},
        )

        errors = validation_service.validate_config_schema(config)

        assert len(errors) == 1
        assert "No dependencies defined" in errors[0].message


# Note: Command reference validation tests removed because this validation
# happens in GraftConfig.__post_init__ during domain model construction.
# Tests for domain validation belong in tests/unit/test_config.py instead.


class TestGetValidationSummary:
    """Tests for get_validation_summary function."""

    def test_separate_errors_and_warnings(self):
        """Should separate errors and warnings correctly."""
        validation_errors = [
            validation_service.ValidationError("Error 1", severity="error"),
            validation_service.ValidationError("Warning 1", severity="warning"),
            validation_service.ValidationError("Error 2", severity="error"),
            validation_service.ValidationError("Warning 2", severity="warning"),
        ]

        errors, warnings = validation_service.get_validation_summary(validation_errors)

        assert len(errors) == 2
        assert "Error 1" in errors
        assert "Error 2" in errors

        assert len(warnings) == 2
        assert "Warning 1" in warnings
        assert "Warning 2" in warnings

    def test_empty_list(self):
        """Should return empty lists for empty input."""
        errors, warnings = validation_service.get_validation_summary([])

        assert errors == []
        assert warnings == []

    def test_only_errors(self):
        """Should return only errors when no warnings."""
        validation_errors = [
            validation_service.ValidationError("Error 1", severity="error"),
            validation_service.ValidationError("Error 2", severity="error"),
        ]

        errors, warnings = validation_service.get_validation_summary(validation_errors)

        assert len(errors) == 2
        assert len(warnings) == 0

    def test_only_warnings(self):
        """Should return only warnings when no errors."""
        validation_errors = [
            validation_service.ValidationError("Warning 1", severity="warning"),
            validation_service.ValidationError("Warning 2", severity="warning"),
        ]

        errors, warnings = validation_service.get_validation_summary(validation_errors)

        assert len(errors) == 0
        assert len(warnings) == 2
