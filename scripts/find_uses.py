#!/usr/bin/env python3
"""Find which prompts depend on a given file (reverse dependency lookup)"""
import sys
import pathlib
import re

try:
    import yaml
except ImportError:
    print("Missing PyYAML. Run: pip install pyyaml", file=sys.stderr)
    sys.exit(1)

ROOT = pathlib.Path(".").resolve()


def parse_frontmatter(prompt_path: pathlib.Path):
    """Extract deps from frontmatter"""
    txt = prompt_path.read_text(encoding="utf-8")
    m = re.match(r"^---\n(.*?)\n---\n", txt, re.S)
    if not m:
        return []
    meta = yaml.safe_load(m.group(1)) or {}
    deps = meta.get("deps") or []
    if not isinstance(deps, list):
        return []
    return deps


def find_uses(target_file: str):
    """Find all prompts that depend on target_file"""
    # Normalize target path
    target_path = pathlib.Path(target_file)
    if not target_path.is_absolute():
        target_path = ROOT / target_file

    # Get relative path for comparison
    try:
        target_rel = target_path.relative_to(ROOT).as_posix()
    except ValueError:
        print(f"Error: {target_file} is not within project root", file=sys.stderr)
        return []

    # Check if file exists
    if not target_path.exists():
        print(f"Warning: {target_file} does not exist", file=sys.stderr)

    # Find all prompt files
    prompt_files = sorted(ROOT.glob("**/*.prompt.md"))

    uses = []
    for prompt_path in prompt_files:
        deps = parse_frontmatter(prompt_path)

        # Check if target is in deps
        if target_rel in deps:
            rel_prompt = prompt_path.relative_to(ROOT).as_posix()
            # Output path
            out_path = prompt_path.parent / f"{prompt_path.stem.replace('.prompt', '')}.md"
            rel_out = out_path.relative_to(ROOT).as_posix()

            uses.append({
                "prompt": rel_prompt,
                "output": rel_out,
                "all_deps": deps
            })

    return uses


def main():
    if len(sys.argv) < 2:
        print("Usage: python3 scripts/find_uses.py <file>", file=sys.stderr)
        print("", file=sys.stderr)
        print("Find which prompts depend on a given file.", file=sys.stderr)
        print("", file=sys.stderr)
        print("Example:", file=sys.stderr)
        print("  python3 scripts/find_uses.py docs/strategy/messaging-framework.md", file=sys.stderr)
        return 1

    target_file = sys.argv[1]
    uses = find_uses(target_file)

    if not uses:
        print(f"No prompts depend on {target_file}")
        return 0

    print(f"Prompts that depend on {target_file}:")
    print()
    for use in uses:
        print(f"  {use['prompt']}")
        print(f"    → generates {use['output']}")
        print(f"    deps: {', '.join(use['all_deps'])}")
        print()

    print(f"Total: {len(uses)} prompt(s)")

    return 0


if __name__ == "__main__":
    sys.exit(main())
