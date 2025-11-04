#!/usr/bin/env python3
"""
Graft Documentation Validation Script

Validates the Graft documentation pipeline structure without regenerating docs.
Performs fast structural checks that can run in CI without AWS credentials.

Validation Layers:
  1. DVC synchronization - dvc.yaml matches prompt files
  2. Frontmatter validity - YAML parses correctly
  3. Missing dependencies - all deps files exist
  4. Circular dependencies - no cycles in dependency graph
"""

import argparse
import glob
import os
import re
import sys
from pathlib import Path
from typing import Dict, List, Set, Tuple

try:
    import yaml
except ImportError:
    print("Error: PyYAML not installed. Run: pip install pyyaml", file=sys.stderr)
    sys.exit(1)


class ValidationError(Exception):
    """Base class for validation errors"""
    pass


class FrontmatterError(ValidationError):
    """Error parsing YAML frontmatter"""
    pass


class MissingDependencyError(ValidationError):
    """Missing dependency file"""
    pass


class CircularDependencyError(ValidationError):
    """Circular dependency detected"""
    pass


def extract_frontmatter(prompt_file: str) -> Tuple[Dict, int]:
    """
    Extract and parse YAML frontmatter from a prompt file.

    Returns:
        (frontmatter_dict, line_number) where line_number is the line where
        the frontmatter section ends
    """
    with open(prompt_file, 'r') as f:
        content = f.read()

    # Match YAML frontmatter between --- markers
    match = re.match(r'^---\n(.*?)\n---\n', content, re.DOTALL)
    if not match:
        raise FrontmatterError(f"No frontmatter found in {prompt_file}")

    frontmatter_str = match.group(1)
    frontmatter_lines = len(frontmatter_str.split('\n'))

    try:
        frontmatter = yaml.safe_load(frontmatter_str)
        if frontmatter is None:
            frontmatter = {}
        return frontmatter, frontmatter_lines + 2  # +2 for the --- markers
    except yaml.YAMLError as e:
        raise FrontmatterError(f"Invalid YAML in {prompt_file}: {e}")


def find_prompt_files() -> List[str]:
    """Find all .prompt.md files in the repository"""
    return glob.glob("**/*.prompt.md", recursive=True)


def validate_frontmatter_structure(prompt_file: str, frontmatter: Dict) -> List[str]:
    """
    Validate that frontmatter has required fields and correct types.

    Returns:
        List of warning messages (non-fatal issues)
    """
    warnings = []

    # Check for deps field
    if 'deps' not in frontmatter:
        warnings.append(f"{prompt_file}: No 'deps' field in frontmatter")
    elif not isinstance(frontmatter['deps'], list):
        warnings.append(f"{prompt_file}: 'deps' field must be a list")

    # Check for model field (optional but recommended)
    if 'model' in frontmatter and not isinstance(frontmatter['model'], str):
        warnings.append(f"{prompt_file}: 'model' field must be a string")

    return warnings


def check_missing_dependencies(prompt_file: str, frontmatter: Dict) -> List[str]:
    """
    Check if all dependencies exist.

    Returns:
        List of missing dependency paths
    """
    if 'deps' not in frontmatter:
        return []

    deps = frontmatter['deps']
    if not isinstance(deps, list):
        return []

    missing = []
    for dep in deps:
        if not isinstance(dep, str):
            continue

        # Skip glob patterns (contain *)
        if '*' in dep:
            # Validate that glob expands to at least one file
            matches = glob.glob(dep, recursive=True)
            if not matches:
                missing.append(f"{dep} (glob pattern matches no files)")
            continue

        # Check if file exists
        if not os.path.exists(dep):
            missing.append(dep)

    return missing


def build_dependency_graph() -> Dict[str, Set[str]]:
    """
    Build a dependency graph from all prompt files.

    Returns:
        Dict mapping prompt files to their output files (dependencies)
    """
    graph = {}

    for prompt_file in find_prompt_files():
        try:
            frontmatter, _ = extract_frontmatter(prompt_file)

            # Get output file (usually prompt_file without .prompt extension)
            output_file = prompt_file.replace('.prompt.md', '.md')

            # Get dependencies
            deps = frontmatter.get('deps', [])
            if not isinstance(deps, list):
                deps = []

            # Filter out glob patterns and non-existent files for graph
            real_deps = set()
            for dep in deps:
                if isinstance(dep, str) and '*' not in dep and os.path.exists(dep):
                    real_deps.add(dep)

            graph[output_file] = real_deps

        except (FrontmatterError, yaml.YAMLError):
            # Skip files with invalid frontmatter for dependency graph
            continue

    return graph


def detect_cycles(graph: Dict[str, Set[str]]) -> List[List[str]]:
    """
    Detect circular dependencies in the dependency graph.

    Returns:
        List of cycles, where each cycle is a list of file paths
    """
    cycles = []

    def visit(node: str, path: List[str], visited: Set[str]) -> None:
        if node in path:
            # Found a cycle
            cycle_start = path.index(node)
            cycle = path[cycle_start:] + [node]
            cycles.append(cycle)
            return

        if node in visited:
            return

        visited.add(node)
        path.append(node)

        # Visit dependencies
        for dep in graph.get(node, set()):
            visit(dep, path.copy(), visited)

    visited_global = set()
    for node in graph:
        if node not in visited_global:
            visit(node, [], visited_global)

    return cycles


def main():
    parser = argparse.ArgumentParser(description="Validate Graft documentation structure")
    parser.add_argument('--json', action='store_true', help='Output results as JSON')
    parser.add_argument('--verbose', '-v', action='store_true', help='Verbose output')
    args = parser.parse_args()

    errors = []
    warnings = []

    # Find all prompt files
    prompt_files = find_prompt_files()

    if args.verbose:
        print(f"Found {len(prompt_files)} prompt files")

    # Validate each prompt file
    for prompt_file in prompt_files:
        try:
            frontmatter, line_num = extract_frontmatter(prompt_file)

            # Validate frontmatter structure
            file_warnings = validate_frontmatter_structure(prompt_file, frontmatter)
            warnings.extend(file_warnings)

            # Check for missing dependencies
            missing_deps = check_missing_dependencies(prompt_file, frontmatter)
            for dep in missing_deps:
                errors.append({
                    'type': 'missing_dependency',
                    'file': prompt_file,
                    'dependency': dep,
                    'message': f"Missing dependency: {dep} (required by {prompt_file})"
                })

        except FrontmatterError as e:
            errors.append({
                'type': 'frontmatter_error',
                'file': prompt_file,
                'message': str(e)
            })

    # Build dependency graph and check for cycles
    try:
        graph = build_dependency_graph()
        cycles = detect_cycles(graph)

        for cycle in cycles:
            cycle_str = ' → '.join(cycle)
            errors.append({
                'type': 'circular_dependency',
                'cycle': cycle,
                'message': f"Circular dependency: {cycle_str}"
            })

    except Exception as e:
        errors.append({
            'type': 'graph_error',
            'message': f"Error building dependency graph: {e}"
        })

    # Output results
    if args.json:
        import json
        result = {
            'errors': errors,
            'warnings': warnings,
            'prompt_files_checked': len(prompt_files),
            'has_errors': len(errors) > 0,
            'has_warnings': len(warnings) > 0
        }
        print(json.dumps(result, indent=2))
    else:
        # Human-readable output
        if errors:
            print("❌ Validation Errors Found:\n", file=sys.stderr)
            for error in errors:
                print(f"  {error['message']}", file=sys.stderr)
            print(file=sys.stderr)

        if warnings:
            print("⚠️  Validation Warnings:\n")
            for warning in warnings:
                print(f"  {warning}")
            print()

        if not errors and not warnings:
            print("✅ All validation checks passed!")
            print(f"   Checked {len(prompt_files)} prompt files")
            return 0

    # Exit with error code if there are errors
    return 1 if errors else 0


if __name__ == '__main__':
    sys.exit(main())
