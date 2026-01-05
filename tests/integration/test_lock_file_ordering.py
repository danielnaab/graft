"""Integration tests for lock file ordering convention.

Verifies that lock files are written with correct ordering:
1. Direct dependencies first (direct: true)
2. Transitive dependencies second (direct: false)
3. Alphabetical within each group

Specification: graft-knowledge/docs/specification/lock-file-format.md lines 83-114
"""

import tempfile
from datetime import UTC, datetime
from pathlib import Path

import pytest
import yaml

from graft.adapters.lock_file import YamlLockFile
from graft.domain.lock_entry import LockEntry


class TestLockFileOrdering:
    """Test lock file ordering convention."""

    @pytest.fixture
    def lock_file(self) -> YamlLockFile:
        """Provide lock file adapter."""
        return YamlLockFile()

    @pytest.fixture
    def temp_lock_path(self) -> str:
        """Provide temporary lock file path."""
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".lock", delete=False
        ) as f:
            path = f.name
        yield path
        # Cleanup
        Path(path).unlink(missing_ok=True)

    def test_direct_deps_before_transitive(
        self, lock_file: YamlLockFile, temp_lock_path: str
    ) -> None:
        """Direct dependencies should appear before transitive dependencies."""
        # Create mixed dependencies (intentionally out of order)
        entries = {
            "transitive-dep": LockEntry(
                source="https://example.com/transitive.git",
                ref="v1.0.0",
                commit="a" * 40,
                consumed_at=datetime(2026, 1, 5, 12, 0, 0, tzinfo=UTC),
                direct=False,  # Transitive
                requires=[],
                required_by=["direct-dep"],
            ),
            "direct-dep": LockEntry(
                source="https://example.com/direct.git",
                ref="v2.0.0",
                commit="b" * 40,
                consumed_at=datetime(2026, 1, 5, 12, 0, 0, tzinfo=UTC),
                direct=True,  # Direct
                requires=["transitive-dep"],
                required_by=[],
            ),
        }

        # Write lock file
        lock_file.write_lock_file(temp_lock_path, entries)

        # Read raw YAML to check ordering
        with open(temp_lock_path) as f:
            data = yaml.safe_load(f)

        deps_list = list(data["dependencies"].keys())

        # Verify direct-dep comes before transitive-dep
        assert deps_list.index("direct-dep") < deps_list.index("transitive-dep")

    def test_alphabetical_within_direct_deps(
        self, lock_file: YamlLockFile, temp_lock_path: str
    ) -> None:
        """Direct dependencies should be alphabetically sorted."""
        entries = {
            "zebra-dep": LockEntry(
                source="https://example.com/zebra.git",
                ref="v1.0.0",
                commit="a" * 40,
                consumed_at=datetime(2026, 1, 5, 12, 0, 0, tzinfo=UTC),
                direct=True,
                requires=[],
                required_by=[],
            ),
            "alpha-dep": LockEntry(
                source="https://example.com/alpha.git",
                ref="v1.0.0",
                commit="b" * 40,
                consumed_at=datetime(2026, 1, 5, 12, 0, 0, tzinfo=UTC),
                direct=True,
                requires=[],
                required_by=[],
            ),
            "middle-dep": LockEntry(
                source="https://example.com/middle.git",
                ref="v1.0.0",
                commit="c" * 40,
                consumed_at=datetime(2026, 1, 5, 12, 0, 0, tzinfo=UTC),
                direct=True,
                requires=[],
                required_by=[],
            ),
        }

        lock_file.write_lock_file(temp_lock_path, entries)

        with open(temp_lock_path) as f:
            data = yaml.safe_load(f)

        deps_list = list(data["dependencies"].keys())

        # Verify alphabetical order
        assert deps_list == ["alpha-dep", "middle-dep", "zebra-dep"]

    def test_alphabetical_within_transitive_deps(
        self, lock_file: YamlLockFile, temp_lock_path: str
    ) -> None:
        """Transitive dependencies should be alphabetically sorted."""
        entries = {
            "trans-zebra": LockEntry(
                source="https://example.com/zebra.git",
                ref="v1.0.0",
                commit="a" * 40,
                consumed_at=datetime(2026, 1, 5, 12, 0, 0, tzinfo=UTC),
                direct=False,
                requires=[],
                required_by=["some-dep"],
            ),
            "trans-alpha": LockEntry(
                source="https://example.com/alpha.git",
                ref="v1.0.0",
                commit="b" * 40,
                consumed_at=datetime(2026, 1, 5, 12, 0, 0, tzinfo=UTC),
                direct=False,
                requires=[],
                required_by=["some-dep"],
            ),
            "trans-middle": LockEntry(
                source="https://example.com/middle.git",
                ref="v1.0.0",
                commit="c" * 40,
                consumed_at=datetime(2026, 1, 5, 12, 0, 0, tzinfo=UTC),
                direct=False,
                requires=[],
                required_by=["some-dep"],
            ),
        }

        lock_file.write_lock_file(temp_lock_path, entries)

        with open(temp_lock_path) as f:
            data = yaml.safe_load(f)

        deps_list = list(data["dependencies"].keys())

        # Verify alphabetical order
        assert deps_list == ["trans-alpha", "trans-middle", "trans-zebra"]

    def test_complete_ordering_convention(
        self, lock_file: YamlLockFile, temp_lock_path: str
    ) -> None:
        """Complete test of ordering convention: direct (alpha) then transitive (alpha)."""
        # Create comprehensive set of dependencies
        entries = {
            # Transitive deps (should come second)
            "trans-beta": LockEntry(
                source="https://example.com/trans-beta.git",
                ref="v1.0.0",
                commit="a" * 40,
                consumed_at=datetime(2026, 1, 5, 12, 0, 0, tzinfo=UTC),
                direct=False,
                requires=[],
                required_by=["direct-alpha"],
            ),
            # Direct deps (should come first)
            "direct-delta": LockEntry(
                source="https://example.com/direct-delta.git",
                ref="v1.0.0",
                commit="b" * 40,
                consumed_at=datetime(2026, 1, 5, 12, 0, 0, tzinfo=UTC),
                direct=True,
                requires=[],
                required_by=[],
            ),
            # Transitive
            "trans-alpha": LockEntry(
                source="https://example.com/trans-alpha.git",
                ref="v1.0.0",
                commit="c" * 40,
                consumed_at=datetime(2026, 1, 5, 12, 0, 0, tzinfo=UTC),
                direct=False,
                requires=[],
                required_by=["direct-delta"],
            ),
            # Direct
            "direct-alpha": LockEntry(
                source="https://example.com/direct-alpha.git",
                ref="v1.0.0",
                commit="d" * 40,
                consumed_at=datetime(2026, 1, 5, 12, 0, 0, tzinfo=UTC),
                direct=True,
                requires=["trans-beta"],
                required_by=[],
            ),
            # Direct
            "direct-charlie": LockEntry(
                source="https://example.com/direct-charlie.git",
                ref="v1.0.0",
                commit="e" * 40,
                consumed_at=datetime(2026, 1, 5, 12, 0, 0, tzinfo=UTC),
                direct=True,
                requires=[],
                required_by=[],
            ),
        }

        lock_file.write_lock_file(temp_lock_path, entries)

        with open(temp_lock_path) as f:
            data = yaml.safe_load(f)

        deps_list = list(data["dependencies"].keys())

        # Expected order: direct (alphabetical), then transitive (alphabetical)
        expected = [
            "direct-alpha",      # Direct, alphabetically first
            "direct-charlie",    # Direct, alphabetically second
            "direct-delta",      # Direct, alphabetically third
            "trans-alpha",       # Transitive, alphabetically first
            "trans-beta",        # Transitive, alphabetically second
        ]

        assert deps_list == expected

    def test_robustness_principle_read_any_order(
        self, lock_file: YamlLockFile, temp_lock_path: str
    ) -> None:
        """Parser should accept dependencies in any order (robustness principle)."""
        # Manually write lock file with non-standard ordering
        lock_data = {
            "apiVersion": "graft/v0",
            "dependencies": {
                # Transitive first (non-standard)
                "trans-dep": {
                    "source": "https://example.com/trans.git",
                    "ref": "v1.0.0",
                    "commit": "a" * 40,
                    "consumed_at": "2026-01-05T12:00:00+00:00",
                    "direct": False,
                    "requires": [],
                    "required_by": ["direct-dep"],
                },
                # Direct second (non-standard)
                "direct-dep": {
                    "source": "https://example.com/direct.git",
                    "ref": "v1.0.0",
                    "commit": "b" * 40,
                    "consumed_at": "2026-01-05T12:00:00+00:00",
                    "direct": True,
                    "requires": ["trans-dep"],
                    "required_by": [],
                },
            },
        }

        with open(temp_lock_path, "w") as f:
            yaml.dump(lock_data, f)

        # Should read successfully despite non-standard ordering
        entries = lock_file.read_lock_file(temp_lock_path)

        assert len(entries) == 2
        assert "direct-dep" in entries
        assert "trans-dep" in entries
        assert entries["direct-dep"].direct is True
        assert entries["trans-dep"].direct is False
