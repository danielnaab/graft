"""Docker adapter for container operations."""
from __future__ import annotations
import subprocess
from pathlib import Path
from typing import Protocol


class BuildError(Exception):
    """Raised when Docker build fails."""
    pass


class TransformerExecutionError(Exception):
    """Raised when container execution fails."""
    pass


class ContainerPort(Protocol):
    """Port for executing containers."""

    def build_image(
        self,
        dockerfile_path: Path,
        image_tag: str,
        context_path: Path
    ) -> None:
        """Build Docker image from Dockerfile.

        Args:
            dockerfile_path: Path to the Dockerfile
            image_tag: Tag for the built image
            context_path: Build context directory

        Raises:
            BuildError: If Docker build fails
        """
        ...

    def run_container(
        self,
        image_tag: str,
        working_dir: Path,
        env_vars: dict[str, str]
    ) -> tuple[int, str, str]:
        """Run container and return (exit_code, stdout, stderr).

        Args:
            image_tag: Tag of the image to run
            working_dir: Directory to mount as /workspace
            env_vars: Environment variables to pass to container

        Returns:
            Tuple of (exit_code, stdout, stderr)

        Raises:
            TransformerExecutionError: If container execution fails
        """
        ...


class DockerAdapter:
    """Docker container execution adapter."""

    def build_image(
        self,
        dockerfile_path: Path,
        image_tag: str,
        context_path: Path
    ) -> None:
        """Build Docker image using docker build.

        Args:
            dockerfile_path: Path to the Dockerfile
            image_tag: Tag for the built image
            context_path: Build context directory

        Raises:
            BuildError: If Docker build fails or Docker is not available
        """
        try:
            result = subprocess.run(
                [
                    "docker", "build",
                    "-t", image_tag,
                    "-f", str(dockerfile_path),
                    str(context_path)
                ],
                capture_output=True,
                text=True,
                timeout=300  # 5 minute timeout for builds
            )

            if result.returncode != 0:
                raise BuildError(f"Docker build failed: {result.stderr}")

        except FileNotFoundError:
            raise BuildError(
                "Docker is required for transformer derivations. "
                "Please install Docker and ensure it is running."
            )
        except subprocess.TimeoutExpired:
            raise BuildError("Docker build timed out after 5 minutes")

    def run_container(
        self,
        image_tag: str,
        working_dir: Path,
        env_vars: dict[str, str]
    ) -> tuple[int, str, str]:
        """Run container with artifact directory mounted.

        Args:
            image_tag: Tag of the image to run
            working_dir: Directory to mount as /workspace
            env_vars: Environment variables to pass to container

        Returns:
            Tuple of (exit_code, stdout, stderr)

        Raises:
            TransformerExecutionError: If Docker is not available
        """
        try:
            # Build docker run command
            cmd = [
                "docker", "run", "--rm",
                "-v", f"{working_dir.absolute()}:/workspace",
                "-w", "/workspace"
            ]

            # Add environment variables
            for key, value in env_vars.items():
                cmd.extend(["-e", f"{key}={value}"])

            cmd.append(image_tag)

            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=600  # 10 minute timeout for execution
            )

            return result.returncode, result.stdout, result.stderr

        except FileNotFoundError:
            raise TransformerExecutionError(
                "Docker is required for transformer derivations. "
                "Please install Docker and ensure it is running."
            )
        except subprocess.TimeoutExpired:
            raise TransformerExecutionError("Container execution timed out after 10 minutes")
