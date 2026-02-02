"""Lock file adapter implementation.

YAML-based lock file operations.
"""

from pathlib import Path

import yaml

from graft.domain.lock_entry import LockEntry


class YamlLockFile:
    """YAML-based lock file implementation.

    Implements lock file operations using YAML format as specified.
    Lock file format (v3 - flat-only):
        apiVersion: graft/v0
        dependencies:
          dep-name:
            source: "..."
            ref: "..."
            commit: "..."
            consumed_at: "..."
    """

    API_VERSION = "graft/v0"
    # Legacy version field for backward compatibility
    LOCK_FILE_VERSION = 1

    def read_lock_file(self, path: str) -> dict[str, LockEntry]:
        """Read lock file and return dependency entries.

        Args:
            path: Path to graft.lock file

        Returns:
            Dictionary mapping dependency name to LockEntry

        Raises:
            FileNotFoundError: If lock file doesn't exist
            ValueError: If lock file is malformed
        """
        path_obj = Path(path)
        if not path_obj.exists():
            raise FileNotFoundError(f"Lock file not found: {path}")

        try:
            with open(path) as f:
                data = yaml.safe_load(f)
        except yaml.YAMLError as e:
            raise ValueError(f"Invalid YAML in lock file: {e}") from e

        if not isinstance(data, dict):
            raise ValueError("Lock file must be a YAML mapping")

        # Validate version (support both v2 apiVersion and v1 version)
        api_version = data.get("apiVersion")
        legacy_version = data.get("version")

        if api_version:
            # V2 format
            if api_version != self.API_VERSION:
                raise ValueError(
                    f"Unsupported API version: {api_version}. "
                    f"Expected {self.API_VERSION}"
                )
        elif legacy_version:
            # V1 format (backward compatibility)
            if legacy_version != self.LOCK_FILE_VERSION:
                raise ValueError(
                    f"Unsupported lock file version: {legacy_version}. "
                    f"Expected version {self.LOCK_FILE_VERSION}"
                )
        else:
            raise ValueError(
                "Lock file missing version field ('apiVersion' or 'version')"
            )

        # Parse dependencies
        entries: dict[str, LockEntry] = {}
        dependencies = data.get("dependencies", {})

        if not isinstance(dependencies, dict):
            raise ValueError("Lock file 'dependencies' must be a mapping")

        for dep_name, dep_data in dependencies.items():
            if not isinstance(dep_data, dict):
                raise ValueError(
                    f"Dependency '{dep_name}' data must be a mapping"
                )

            try:
                entry = LockEntry.from_dict(dep_data)
                entries[dep_name] = entry
            except Exception as e:
                raise ValueError(
                    f"Invalid lock entry for '{dep_name}': {e}"
                ) from e

        return entries

    def write_lock_file(self, path: str, entries: dict[str, LockEntry]) -> None:
        """Write lock file with dependency entries.

        Uses v3 format with apiVersion field (flat-only model).

        Args:
            path: Path to graft.lock file
            entries: Dictionary mapping dependency name to LockEntry

        Raises:
            IOError: If unable to write file
        """
        # Build lock file structure (v3 format - flat-only)
        # Simple alphabetical ordering
        lock_data = {
            "apiVersion": self.API_VERSION,
            "dependencies": {
                name: entries[name].to_dict()
                for name in sorted(entries.keys())
            },
        }

        # Write to file
        try:
            # Ensure parent directory exists
            path_obj = Path(path)
            path_obj.parent.mkdir(parents=True, exist_ok=True)

            with open(path, "w") as f:
                yaml.dump(
                    lock_data,
                    f,
                    default_flow_style=False,
                    sort_keys=False,
                    allow_unicode=True,
                )
        except Exception as e:
            raise OSError(f"Failed to write lock file: {e}") from e

    def update_lock_entry(
        self, path: str, dep_name: str, entry: LockEntry
    ) -> None:
        """Update a single dependency entry in lock file.

        Atomic operation that reads, updates, and writes.

        Args:
            path: Path to graft.lock file
            dep_name: Name of dependency to update
            entry: New LockEntry for the dependency

        Raises:
            FileNotFoundError: If lock file doesn't exist
            IOError: If unable to write file
        """
        # Read existing entries
        entries = self.read_lock_file(path)

        # Update the entry
        entries[dep_name] = entry

        # Write back
        self.write_lock_file(path, entries)

    def lock_file_exists(self, path: str) -> bool:
        """Check if lock file exists.

        Args:
            path: Path to check

        Returns:
            True if lock file exists
        """
        return Path(path).exists()
