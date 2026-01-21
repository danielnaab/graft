"""Unit tests for dependency domain models.

Tests for GitRef, GitUrl, DependencySpec, and DependencyResolution.
"""

import dataclasses

import pytest

from graft.domain.dependency import (
    DependencyResolution,
    DependencySpec,
    DependencyStatus,
    GitRef,
    GitUrl,
)
from graft.domain.exceptions import ValidationError


class TestGitRef:
    """Tests for GitRef value object."""

    def test_create_valid_ref(self) -> None:
        """Should create valid git ref."""
        ref = GitRef("main")
        assert ref.ref == "main"
        assert str(ref) == "main"

    def test_create_ref_with_type(self) -> None:
        """Should create ref with type hint."""
        ref = GitRef("v1.0.0", ref_type="tag")
        assert ref.ref == "v1.0.0"
        assert ref.ref_type == "tag"

    def test_empty_ref_raises_validation_error(self) -> None:
        """Should raise ValidationError for empty ref."""
        with pytest.raises(ValidationError, match="cannot be empty"):
            GitRef("")

    def test_whitespace_ref_raises_validation_error(self) -> None:
        """Should raise ValidationError for whitespace ref."""
        with pytest.raises(ValidationError, match="whitespace"):
            GitRef("   ")

    def test_immutable(self) -> None:
        """Should be immutable (frozen dataclass)."""
        ref = GitRef("main")
        with pytest.raises(dataclasses.FrozenInstanceError):
            ref.ref = "develop"  # type: ignore


class TestGitUrl:
    """Tests for GitUrl value object."""

    def test_create_ssh_url(self) -> None:
        """Should parse SSH URL."""
        url = GitUrl("ssh://git@github.com/user/repo.git")
        assert url.scheme == "ssh"
        assert url.host == "git@github.com"
        assert str(url) == "ssh://git@github.com/user/repo.git"

    def test_create_https_url(self) -> None:
        """Should parse HTTPS URL."""
        url = GitUrl("https://github.com/user/repo.git")
        assert url.scheme == "https"
        assert url.host == "github.com"
        assert "/user/repo.git" in url.path

    def test_create_http_url(self) -> None:
        """Should parse HTTP URL."""
        url = GitUrl("http://example.com/repo.git")
        assert url.scheme == "http"

    def test_create_file_url(self) -> None:
        """Should parse file:// URL."""
        url = GitUrl("file:///path/to/repo.git")
        assert url.scheme == "file"

    def test_empty_url_raises_validation_error(self) -> None:
        """Should raise ValidationError for empty URL."""
        with pytest.raises(ValidationError, match="cannot be empty"):
            GitUrl("")

    def test_invalid_scheme_raises_error(self) -> None:
        """Should raise ValidationError for invalid scheme."""
        with pytest.raises(ValidationError, match="Invalid URL scheme"):
            GitUrl("ftp://example.com/repo.git")

    def test_immutable(self) -> None:
        """Should be immutable (frozen dataclass)."""
        url = GitUrl("https://github.com/user/repo.git")
        with pytest.raises(dataclasses.FrozenInstanceError):
            url.url = "https://other.com"  # type: ignore

    def test_scp_style_url_normalized(self) -> None:
        """Should normalize SCP-style URL to SSH URL."""
        url = GitUrl("git@github.com:user/repo.git")
        assert url.scheme == "ssh"
        assert url.host == "git@github.com"
        assert url.url == "ssh://git@github.com/user/repo.git"

    def test_scp_style_url_with_nested_path(self) -> None:
        """Should handle SCP-style URL with nested path."""
        url = GitUrl("git@github.com:org/suborg/repo.git")
        assert url.url == "ssh://git@github.com/org/suborg/repo.git"

    def test_mixed_format_url_normalized(self) -> None:
        """Should normalize mixed format (ssh:// with colon path separator)."""
        url = GitUrl("ssh://git@github.com:user/repo.git")
        assert url.scheme == "ssh"
        assert url.url == "ssh://git@github.com/user/repo.git"

    def test_git_scheme_mixed_format_normalized(self) -> None:
        """Should normalize git:// scheme with colon path separator."""
        url = GitUrl("git://git@github.com:user/repo.git")
        assert url.url == "git://git@github.com/user/repo.git"

    def test_proper_ssh_url_unchanged(self) -> None:
        """Should not modify already-correct SSH URLs."""
        url = GitUrl("ssh://git@github.com/user/repo.git")
        assert url.url == "ssh://git@github.com/user/repo.git"

    def test_https_url_unchanged(self) -> None:
        """Should not modify HTTPS URLs."""
        url = GitUrl("https://github.com/user/repo.git")
        assert url.url == "https://github.com/user/repo.git"

    def test_ssh_url_with_port_unchanged(self) -> None:
        """Should not modify SSH URLs with explicit port."""
        url = GitUrl("ssh://git@github.com:22/user/repo.git")
        assert url.url == "ssh://git@github.com:22/user/repo.git"

    def test_git_url_with_port_unchanged(self) -> None:
        """Should not modify git:// URLs with explicit port."""
        url = GitUrl("git://git@github.com:9418/user/repo.git")
        assert url.url == "git://git@github.com:9418/user/repo.git"

    def test_relative_path_unchanged(self) -> None:
        """Should not modify relative paths."""
        url = GitUrl("../shared-utils")
        assert url.url == "../shared-utils"

    def test_absolute_path_unchanged(self) -> None:
        """Should not modify absolute paths."""
        url = GitUrl("/home/user/repos/my-repo")
        assert url.url == "/home/user/repos/my-repo"


class TestDependencySpec:
    """Tests for DependencySpec value object."""

    def test_create_valid_spec(self) -> None:
        """Should create valid dependency spec."""
        spec = DependencySpec(
            name="test-dep",
            git_url=GitUrl("https://github.com/user/repo.git"),
            git_ref=GitRef("main"),
        )
        assert spec.name == "test-dep"
        assert spec.git_url.scheme == "https"
        assert spec.git_ref.ref == "main"

    def test_empty_name_raises_error(self) -> None:
        """Should raise ValidationError for empty name."""
        with pytest.raises(ValidationError, match="cannot be empty"):
            DependencySpec(
                name="",
                git_url=GitUrl("https://github.com/user/repo.git"),
                git_ref=GitRef("main"),
            )

    def test_long_name_raises_error(self) -> None:
        """Should raise ValidationError for name > 100 chars."""
        with pytest.raises(ValidationError, match="too long"):
            DependencySpec(
                name="a" * 101,
                git_url=GitUrl("https://github.com/user/repo.git"),
                git_ref=GitRef("main"),
            )

    def test_name_with_slash_raises_error(self) -> None:
        """Should reject names with forward slash."""
        with pytest.raises(ValidationError, match="path separators"):
            DependencySpec(
                name="invalid/name",
                git_url=GitUrl("https://github.com/user/repo.git"),
                git_ref=GitRef("main"),
            )

    def test_name_with_backslash_raises_error(self) -> None:
        """Should reject names with backslash."""
        with pytest.raises(ValidationError, match="path separators"):
            DependencySpec(
                name="invalid\\name",
                git_url=GitUrl("https://github.com/user/repo.git"),
                git_ref=GitRef("main"),
            )

    def test_immutable(self) -> None:
        """Should be immutable (frozen dataclass)."""
        spec = DependencySpec(
            name="test-dep",
            git_url=GitUrl("https://github.com/user/repo.git"),
            git_ref=GitRef("main"),
        )
        with pytest.raises(dataclasses.FrozenInstanceError):
            spec.name = "other"  # type: ignore


class TestDependencyResolution:
    """Tests for DependencyResolution entity."""

    def test_create_pending_resolution(self) -> None:
        """Should create resolution with pending status."""
        spec = DependencySpec(
            name="test-dep",
            git_url=GitUrl("https://github.com/user/repo.git"),
            git_ref=GitRef("main"),
        )
        resolution = DependencyResolution(spec=spec, status=DependencyStatus.PENDING)

        assert resolution.name == "test-dep"
        assert resolution.status == DependencyStatus.PENDING
        assert resolution.local_path is None
        assert resolution.error_message is None

    def test_mark_cloning(self) -> None:
        """Should mark resolution as cloning."""
        spec = DependencySpec(
            name="test-dep",
            git_url=GitUrl("https://github.com/user/repo.git"),
            git_ref=GitRef("main"),
        )
        resolution = DependencyResolution(spec=spec, status=DependencyStatus.PENDING)

        resolution.mark_cloning()

        assert resolution.status == DependencyStatus.CLONING

    def test_mark_resolved(self) -> None:
        """Should mark resolution as successful."""
        spec = DependencySpec(
            name="test-dep",
            git_url=GitUrl("https://github.com/user/repo.git"),
            git_ref=GitRef("main"),
        )
        resolution = DependencyResolution(spec=spec, status=DependencyStatus.PENDING)

        resolution.mark_resolved("/path/to/repo")

        assert resolution.status == DependencyStatus.RESOLVED
        assert resolution.local_path == "/path/to/repo"
        assert resolution.error_message is None

    def test_mark_failed(self) -> None:
        """Should mark resolution as failed."""
        spec = DependencySpec(
            name="test-dep",
            git_url=GitUrl("https://github.com/user/repo.git"),
            git_ref=GitRef("main"),
        )
        resolution = DependencyResolution(spec=spec, status=DependencyStatus.PENDING)

        resolution.mark_failed("Clone failed")

        assert resolution.status == DependencyStatus.FAILED
        assert resolution.error_message == "Clone failed"

    def test_mark_resolved_clears_error(self) -> None:
        """Should clear error when marking as resolved."""
        spec = DependencySpec(
            name="test-dep",
            git_url=GitUrl("https://github.com/user/repo.git"),
            git_ref=GitRef("main"),
        )
        resolution = DependencyResolution(spec=spec, status=DependencyStatus.FAILED)
        resolution.error_message = "Previous error"

        resolution.mark_resolved("/path/to/repo")

        assert resolution.status == DependencyStatus.RESOLVED
        assert resolution.error_message is None

    def test_equality_based_on_name(self) -> None:
        """Should be equal if same dependency name."""
        spec1 = DependencySpec(
            name="test-dep",
            git_url=GitUrl("https://github.com/user/repo.git"),
            git_ref=GitRef("main"),
        )
        spec2 = DependencySpec(
            name="test-dep",
            git_url=GitUrl("https://github.com/other/repo.git"),
            git_ref=GitRef("develop"),
        )

        res1 = DependencyResolution(spec=spec1, status=DependencyStatus.PENDING)
        res2 = DependencyResolution(spec=spec2, status=DependencyStatus.RESOLVED)

        assert res1 == res2  # Same name

    def test_inequality_different_names(self) -> None:
        """Should not be equal if different dependency names."""
        spec1 = DependencySpec(
            name="dep1",
            git_url=GitUrl("https://github.com/user/repo.git"),
            git_ref=GitRef("main"),
        )
        spec2 = DependencySpec(
            name="dep2",
            git_url=GitUrl("https://github.com/user/repo.git"),
            git_ref=GitRef("main"),
        )

        res1 = DependencyResolution(spec=spec1, status=DependencyStatus.PENDING)
        res2 = DependencyResolution(spec=spec2, status=DependencyStatus.PENDING)

        assert res1 != res2

    def test_hashable(self) -> None:
        """Should be hashable (can use in sets/dicts)."""
        spec = DependencySpec(
            name="test-dep",
            git_url=GitUrl("https://github.com/user/repo.git"),
            git_ref=GitRef("main"),
        )
        resolution = DependencyResolution(spec=spec, status=DependencyStatus.PENDING)

        # Should be able to add to set
        resolutions = {resolution}
        assert resolution in resolutions
