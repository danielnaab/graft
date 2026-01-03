"""Unit tests for configuration service.

Tests for parse_graft_yaml and find_graft_yaml service functions.

Rationale:
    Configuration parsing is critical - errors here prevent the entire tool from working.
    These tests ensure we provide clear, actionable error messages for common mistakes:
    - Missing files
    - Malformed YAML
    - Invalid configuration structure
    - Invalid dependency format
"""

import pytest

from graft.domain.exceptions import (
    ConfigFileNotFoundError,
    ConfigParseError,
    ConfigValidationError,
)
from graft.services import config_service
from graft.services.dependency_context import DependencyContext
from tests.fakes.fake_filesystem import FakeFileSystem


class TestParseGraftYaml:
    """Tests for parse_graft_yaml service function."""

    def test_parse_valid_config(
        self,
        dependency_context: DependencyContext,
        fake_filesystem: FakeFileSystem,
    ) -> None:
        """Should parse valid graft.yaml."""
        # Setup: Create valid config file
        fake_filesystem.create_file(
            "/fake/cwd/graft.yaml",
            """apiVersion: graft/v0
deps:
  graft-knowledge: "ssh://git@example.com/repo.git#main"
""",
        )

        # Exercise
        config = config_service.parse_graft_yaml(
            dependency_context,
            "/fake/cwd/graft.yaml",
        )

        # Verify
        assert config.api_version == "graft/v0"
        assert len(config.dependencies) == 1
        assert config.has_dependency("graft-knowledge")

        dep = config.get_dependency("graft-knowledge")
        assert dep.name == "graft-knowledge"
        assert dep.git_ref.ref == "main"
        assert "git@example.com" in dep.git_url.url

    def test_parse_multiple_dependencies(
        self,
        dependency_context: DependencyContext,
        fake_filesystem: FakeFileSystem,
    ) -> None:
        """Should parse config with multiple dependencies."""
        fake_filesystem.create_file(
            "/fake/cwd/graft.yaml",
            """apiVersion: graft/v0
deps:
  dep1: "https://github.com/user/repo1.git#main"
  dep2: "https://github.com/user/repo2.git#develop"
  dep3: "ssh://git@example.com/repo3.git#v1.0.0"
""",
        )

        config = config_service.parse_graft_yaml(
            dependency_context,
            "/fake/cwd/graft.yaml",
        )

        assert len(config.dependencies) == 3
        assert config.has_dependency("dep1")
        assert config.has_dependency("dep2")
        assert config.has_dependency("dep3")

    def test_missing_file_raises_error(
        self,
        dependency_context: DependencyContext,
    ) -> None:
        """Should raise ConfigFileNotFoundError if file doesn't exist.

        Rationale: Users need clear guidance when graft.yaml is missing,
        including where it was expected and how to create it.
        """
        with pytest.raises(ConfigFileNotFoundError) as exc_info:
            config_service.parse_graft_yaml(
                dependency_context,
                "/fake/cwd/missing.yaml",
            )

        # Verify error includes helpful context
        assert exc_info.value.path == "/fake/cwd/missing.yaml"
        assert "Create graft.yaml" in exc_info.value.suggestion

    def test_invalid_yaml_raises_error(
        self,
        dependency_context: DependencyContext,
        fake_filesystem: FakeFileSystem,
    ) -> None:
        """Should raise ConfigParseError for invalid YAML.

        Rationale: Syntax errors in YAML are common. Error should clearly
        indicate the file and provide the YAML parser's error message.
        """
        fake_filesystem.create_file(
            "/fake/cwd/graft.yaml",
            "invalid: yaml: content: [[[",
        )

        with pytest.raises(ConfigParseError) as exc_info:
            config_service.parse_graft_yaml(
                dependency_context,
                "/fake/cwd/graft.yaml",
            )

        assert exc_info.value.path == "/fake/cwd/graft.yaml"
        assert "syntax" in exc_info.value.reason.lower()

    def test_missing_api_version_raises_error(
        self,
        dependency_context: DependencyContext,
        fake_filesystem: FakeFileSystem,
    ) -> None:
        """Should raise ConfigValidationError if apiVersion missing.

        Rationale: apiVersion is required for forward compatibility.
        Error should specify which field is missing.
        """
        fake_filesystem.create_file(
            "/fake/cwd/graft.yaml",
            """deps:
  test: "https://example.com/repo.git#main"
""",
        )

        with pytest.raises(ConfigValidationError) as exc_info:
            config_service.parse_graft_yaml(
                dependency_context,
                "/fake/cwd/graft.yaml",
            )

        assert exc_info.value.field == "apiVersion"
        assert "Missing required field" in exc_info.value.reason

    def test_missing_deps_is_now_optional(
        self,
        dependency_context: DependencyContext,
        fake_filesystem: FakeFileSystem,
    ) -> None:
        """Should allow missing deps field.

        Rationale: deps field is now optional to support new 'dependencies' format.
        A graft.yaml can define just changes/commands without dependencies.
        """
        fake_filesystem.create_file(
            "/fake/cwd/graft.yaml",
            "apiVersion: graft/v0\n",
        )

        # Should not raise - deps is optional now
        config = config_service.parse_graft_yaml(
            dependency_context,
            "/fake/cwd/graft.yaml",
        )

        # Should have empty dependencies
        assert config.dependencies == {}

    def test_deps_not_dict_raises_error(
        self,
        dependency_context: DependencyContext,
        fake_filesystem: FakeFileSystem,
    ) -> None:
        """Should raise ConfigValidationError if deps is not a dict.

        Rationale: deps must be a mapping of name to URL.
        Common mistake is using a list instead.
        """
        fake_filesystem.create_file(
            "/fake/cwd/graft.yaml",
            """apiVersion: graft/v0
deps:
  - "https://example.com/repo.git#main"
""",
        )

        with pytest.raises(ConfigValidationError) as exc_info:
            config_service.parse_graft_yaml(
                dependency_context,
                "/fake/cwd/graft.yaml",
            )

        assert exc_info.value.field == "deps"
        assert "mapping" in exc_info.value.reason.lower()

    def test_dependency_without_hash_raises_error(
        self,
        dependency_context: DependencyContext,
        fake_filesystem: FakeFileSystem,
    ) -> None:
        """Should raise ConfigValidationError if dependency missing #ref.

        Rationale: url#ref format is required for version pinning.
        Error should show the expected format and what was provided.
        """
        fake_filesystem.create_file(
            "/fake/cwd/graft.yaml",
            """apiVersion: graft/v0
deps:
  test: "https://example.com/repo.git"
""",
        )

        with pytest.raises(ConfigValidationError) as exc_info:
            config_service.parse_graft_yaml(
                dependency_context,
                "/fake/cwd/graft.yaml",
            )

        assert exc_info.value.field == "deps.test"
        assert "url#ref" in exc_info.value.reason

    def test_not_yaml_dict_raises_error(
        self,
        dependency_context: DependencyContext,
        fake_filesystem: FakeFileSystem,
    ) -> None:
        """Should raise ConfigValidationError if YAML is not a dict.

        Rationale: graft.yaml must be a mapping at root level.
        Common mistake is using a YAML list.
        """
        fake_filesystem.create_file(
            "/fake/cwd/graft.yaml",
            "- item1\n- item2\n",
        )

        with pytest.raises(ConfigValidationError) as exc_info:
            config_service.parse_graft_yaml(
                dependency_context,
                "/fake/cwd/graft.yaml",
            )

        assert exc_info.value.field == "root"

    def test_parse_metadata_section(
        self,
        dependency_context: DependencyContext,
        fake_filesystem: FakeFileSystem,
    ) -> None:
        """Should parse optional metadata section."""
        fake_filesystem.create_file(
            "/fake/cwd/graft.yaml",
            """apiVersion: graft/v0
metadata:
  name: "example-library"
  description: "An example library"
  version: "1.0.0"
""",
        )

        config = config_service.parse_graft_yaml(
            dependency_context,
            "/fake/cwd/graft.yaml",
        )

        assert config.metadata == {
            "name": "example-library",
            "description": "An example library",
            "version": "1.0.0",
        }

    def test_parse_commands_section(
        self,
        dependency_context: DependencyContext,
        fake_filesystem: FakeFileSystem,
    ) -> None:
        """Should parse commands section."""
        fake_filesystem.create_file(
            "/fake/cwd/graft.yaml",
            """apiVersion: graft/v0
commands:
  test:
    run: "npm test"
    description: "Run tests"
  build:
    run: "npm build"
    working_dir: "src"
    env:
      NODE_ENV: "production"
""",
        )

        config = config_service.parse_graft_yaml(
            dependency_context,
            "/fake/cwd/graft.yaml",
        )

        assert "test" in config.commands
        assert "build" in config.commands
        assert config.commands["test"].run == "npm test"
        assert config.commands["test"].description == "Run tests"
        assert config.commands["build"].working_dir == "src"
        assert config.commands["build"].env == {"NODE_ENV": "production"}

    def test_parse_changes_section(
        self,
        dependency_context: DependencyContext,
        fake_filesystem: FakeFileSystem,
    ) -> None:
        """Should parse changes section."""
        fake_filesystem.create_file(
            "/fake/cwd/graft.yaml",
            """apiVersion: graft/v0
changes:
  v1.0.0:
    type: feature
    description: "Initial release"
  v2.0.0:
    type: breaking
    description: "Major refactor"
    migration: migrate-v2
    verify: verify-v2
commands:
  migrate-v2:
    run: "./migrate.sh"
  verify-v2:
    run: "npm test"
""",
        )

        config = config_service.parse_graft_yaml(
            dependency_context,
            "/fake/cwd/graft.yaml",
        )

        assert "v1.0.0" in config.changes
        assert "v2.0.0" in config.changes
        assert config.changes["v1.0.0"].type == "feature"
        assert config.changes["v2.0.0"].type == "breaking"
        assert config.changes["v2.0.0"].migration == "migrate-v2"
        assert config.changes["v2.0.0"].verify == "verify-v2"

    def test_parse_new_dependencies_format_with_source_and_ref(
        self,
        dependency_context: DependencyContext,
        fake_filesystem: FakeFileSystem,
    ) -> None:
        """Should parse new dependencies format with source and ref."""
        fake_filesystem.create_file(
            "/fake/cwd/graft.yaml",
            """apiVersion: graft/v0
dependencies:
  meta-kb:
    source: "git@github.com:org/meta-kb.git"
    ref: "v1.5.0"
  shared-utils:
    source: "../shared-utils"
""",
        )

        config = config_service.parse_graft_yaml(
            dependency_context,
            "/fake/cwd/graft.yaml",
        )

        assert "meta-kb" in config.dependencies
        assert "shared-utils" in config.dependencies
        assert str(config.dependencies["meta-kb"].git_url) == "git@github.com:org/meta-kb.git"
        assert str(config.dependencies["meta-kb"].git_ref) == "v1.5.0"
        # Should default to "main" when ref not specified
        assert str(config.dependencies["shared-utils"].git_ref) == "main"

    def test_change_references_nonexistent_migration_command(
        self,
        dependency_context: DependencyContext,
        fake_filesystem: FakeFileSystem,
    ) -> None:
        """Should raise ValidationError when change references missing migration command."""
        fake_filesystem.create_file(
            "/fake/cwd/graft.yaml",
            """apiVersion: graft/v0
changes:
  v2.0.0:
    migration: nonexistent-command
""",
        )

        # ValidationError raised during GraftConfig validation
        from graft.domain.exceptions import ValidationError

        with pytest.raises(ValidationError) as exc_info:
            config_service.parse_graft_yaml(
                dependency_context,
                "/fake/cwd/graft.yaml",
            )

        error_msg = str(exc_info.value)
        assert "migration command 'nonexistent-command'" in error_msg
        assert "not found in commands section" in error_msg

    def test_change_references_nonexistent_verify_command(
        self,
        dependency_context: DependencyContext,
        fake_filesystem: FakeFileSystem,
    ) -> None:
        """Should raise ValidationError when change references missing verify command."""
        fake_filesystem.create_file(
            "/fake/cwd/graft.yaml",
            """apiVersion: graft/v0
changes:
  v2.0.0:
    verify: nonexistent-verify
""",
        )

        # ValidationError raised during GraftConfig validation
        from graft.domain.exceptions import ValidationError

        with pytest.raises(ValidationError) as exc_info:
            config_service.parse_graft_yaml(
                dependency_context,
                "/fake/cwd/graft.yaml",
            )

        error_msg = str(exc_info.value)
        assert "verify command 'nonexistent-verify'" in error_msg
        assert "not found in commands section" in error_msg

    def test_command_without_run_field_raises_error(
        self,
        dependency_context: DependencyContext,
        fake_filesystem: FakeFileSystem,
    ) -> None:
        """Should raise ConfigValidationError if command missing 'run' field."""
        fake_filesystem.create_file(
            "/fake/cwd/graft.yaml",
            """apiVersion: graft/v0
commands:
  test:
    description: "Missing run field"
""",
        )

        with pytest.raises(ConfigValidationError) as exc_info:
            config_service.parse_graft_yaml(
                dependency_context,
                "/fake/cwd/graft.yaml",
            )

        assert "commands.test" in exc_info.value.field
        assert "run" in exc_info.value.reason.lower()

    def test_parse_changes_with_custom_metadata(
        self,
        dependency_context: DependencyContext,
        fake_filesystem: FakeFileSystem,
    ) -> None:
        """Should extract custom metadata from changes."""
        fake_filesystem.create_file(
            "/fake/cwd/graft.yaml",
            """apiVersion: graft/v0
changes:
  v1.0.0:
    type: feature
    description: "Test"
    author: "jane@example.com"
    jira_ticket: "PROJ-123"
""",
        )

        config = config_service.parse_graft_yaml(
            dependency_context,
            "/fake/cwd/graft.yaml",
        )

        change = config.changes["v1.0.0"]
        assert change.metadata["author"] == "jane@example.com"
        assert change.metadata["jira_ticket"] == "PROJ-123"

    def test_parse_complete_graft_yaml_with_all_sections(
        self,
        dependency_context: DependencyContext,
        fake_filesystem: FakeFileSystem,
    ) -> None:
        """Should parse complete graft.yaml with all sections."""
        fake_filesystem.create_file(
            "/fake/cwd/graft.yaml",
            """apiVersion: graft/v0
metadata:
  name: "example-lib"
  description: "Complete example"

changes:
  v1.0.0:
    type: feature
    description: "Initial release"
    migration: migrate-v1
    verify: verify-v1

commands:
  migrate-v1:
    run: "./migrate.sh"
    description: "Run migration"
  verify-v1:
    run: "npm test"
    description: "Verify migration"

dependencies:
  meta-kb:
    source: "git@github.com:org/meta-kb.git"
    ref: "v1.5.0"

deps:
  old-dep: "git@github.com:org/old.git#main"
""",
        )

        config = config_service.parse_graft_yaml(
            dependency_context,
            "/fake/cwd/graft.yaml",
        )

        # All sections should be parsed
        assert config.metadata["name"] == "example-lib"
        assert "v1.0.0" in config.changes
        assert "migrate-v1" in config.commands
        assert "meta-kb" in config.dependencies
        assert "old-dep" in config.dependencies  # Both formats should coexist


class TestFindGraftYaml:
    """Tests for find_graft_yaml service function.

    Rationale: Users often run graft in the wrong directory.
    These tests ensure clear error messages about where graft.yaml was expected.
    """

    def test_find_existing_config(
        self,
        dependency_context: DependencyContext,
        fake_filesystem: FakeFileSystem,
    ) -> None:
        """Should find graft.yaml in current directory.

        Rationale: Normal case - graft.yaml exists in cwd.
        """
        # Setup
        fake_filesystem.set_cwd("/fake/project")
        fake_filesystem.create_file("/fake/project/graft.yaml", "content")

        # Exercise
        config_path = config_service.find_graft_yaml(dependency_context)

        # Verify
        assert config_path == "/fake/project/graft.yaml"

    def test_missing_config_raises_error(
        self,
        dependency_context: DependencyContext,
        fake_filesystem: FakeFileSystem,
    ) -> None:
        """Should raise ConfigFileNotFoundError if graft.yaml not found.

        Rationale: User needs to know where graft.yaml was expected
        and how to create it in that location.
        """
        # Setup
        fake_filesystem.set_cwd("/fake/empty")

        # Verify
        with pytest.raises(ConfigFileNotFoundError) as exc_info:
            config_service.find_graft_yaml(dependency_context)

        assert "/fake/empty/graft.yaml" in exc_info.value.path
        assert "/fake/empty" in exc_info.value.suggestion
