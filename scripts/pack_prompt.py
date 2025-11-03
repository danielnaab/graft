#!/usr/bin/env python3
import argparse, pathlib, subprocess, re, hashlib, os, sys, json
try:
    import yaml
except ImportError:
    print("Missing PyYAML. Run: pip install pyyaml", file=sys.stderr); sys.exit(1)

SYSTEM = """You are maintaining a strategic Markdown document. You will receive a CHANGE ANALYSIS that tells you exactly what changed and what action to take.

Follow the action directive precisely:

- GENERATE: Create a new document from scratch using source content and style prompt
- RESTYLE: The style/tone has changed but sources haven't - rewrite the ENTIRE document to match the new style, preserving the same semantic content but changing tone, structure, or presentation as directed
- UPDATE: Sources changed but style hasn't - apply ONLY the semantic changes from the source diff, keeping all other sections byte-identical
- REFRESH: Both sources and style changed - apply source updates AND rewrite in the new style
- MAINTAIN: Nothing changed - output should be identical to previous draft

When RESTYLE is requested, you MUST rewrite the full document. Compare PREVIOUS STYLE PROMPT vs CURRENT STYLE PROMPT to understand what changed."""

def read(p): return pathlib.Path(p).read_text(encoding="utf-8")
def exists(p): return pathlib.Path(p).exists()

FM_RE = re.compile(r"^---\n(.*?)\n---\n(.*)$", re.S)

DEFAULTS = {
  "model": "bedrock-claude-v4.5-sonnet-us",
}

def parse_front_matter(text):
    m = FM_RE.match(text)
    if not m:
        return {}, text
    meta = yaml.safe_load(m.group(1)) or {}
    body = m.group(2)
    return meta, body

def redact(text):
    # mask obvious env-looking secrets
    return re.sub(r"(AWS_[A-Z_]+=)[^\s]+", r"\1***", text)

def get_prev_commit_content(path):
    """Get file content from previous commit"""
    try:
        base = subprocess.check_output(["git", "rev-parse", "HEAD~1"], text=True).strip()
        content = subprocess.check_output(["git", "show", f"{base}:{path}"], text=True)
        return content
    except Exception:
        return None

def git_unified_diff(paths):
    if not paths:
        return ""
    try:
        # Show diff vs last commit; fallback to working tree if needed
        base = subprocess.check_output(["git", "rev-parse", "HEAD~1"], text=True).strip()
        diff = subprocess.check_output(["git", "diff", "--unified=3", base, "HEAD", "--"] + paths, text=True)
        # If no diff (files unchanged), show full content instead
        if not diff.strip():
            chunks = []
            for p in paths:
                if exists(p):
                    chunks.append(f"--- a/{p}\n+++ b/{p}\n@@ CURRENT CONTENT @@\n" + read(p))
            return "\n\n".join(chunks)
        return diff
    except Exception:
        # Fallback: no git history yet
        chunks = []
        for p in paths:
            if exists(p):
                chunks.append(f"--- a/{p}\n+++ b/{p}\n@@ NEW FILE @@\n" + read(p))
        return "\n\n".join(chunks)

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--prompt", required=True)
    ap.add_argument("--prev", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--params-out", required=False, help="Path to write effective params JSON")
    ap.add_argument("--name", required=False, help="Logical doc name for build files")
    args = ap.parse_args()

    prompt_raw = read(args.prompt)
    meta, prompt_body = parse_front_matter(prompt_raw)

    # Validate deps (optional but recommended)
    deps = meta.get("deps") or []
    if not isinstance(deps, list):
        print(f"Error: 'deps' must be a list in {args.prompt}", file=sys.stderr)
        sys.exit(1)
    for d in deps:
        if not exists(d):
            print(f"Error: dep does not exist: {d}", file=sys.stderr)
            sys.exit(1)

    # Effective params with defaults
    eff = dict(DEFAULTS)
    if "model" in meta and meta["model"] is not None:
        eff["model"] = meta["model"]

    prev = pathlib.Path(args.prev).read_text("utf-8") if exists(args.prev) else ""
    diff = git_unified_diff(deps)

    # Detect what changed
    sources_changed = bool(diff.strip() and "@@ CURRENT CONTENT @@" not in diff)
    output_exists = bool(prev.strip())

    # Check if prompt changed
    prev_prompt_content = get_prev_commit_content(args.prompt)
    if prev_prompt_content:
        prev_meta, prev_prompt_body = parse_front_matter(prev_prompt_content)
        prompt_changed = prev_prompt_body.strip() != prompt_body.strip()
    else:
        prev_prompt_body = None
        prompt_changed = False

    # Determine action
    if not output_exists:
        action = "GENERATE (no previous draft exists)"
    elif prompt_changed and not sources_changed:
        action = "RESTYLE (prompt changed, sources unchanged - rewrite entire document with new style)"
    elif sources_changed and not prompt_changed:
        action = "UPDATE (sources changed - apply semantic changes only)"
    elif sources_changed and prompt_changed:
        action = "REFRESH (both changed - apply source updates AND new style)"
    else:
        action = "MAINTAIN (no changes detected - keep document unchanged)"

    # Build change analysis section
    change_analysis = f"""CHANGE ANALYSIS:
- Source files: {'CHANGED' if sources_changed else 'NO CHANGES'}
- Prompt instructions: {'CHANGED' if prompt_changed else 'NO CHANGES'}
- Previous draft: {'EXISTS' if output_exists else 'NONE'}
- Action required: {action}
"""

    # Show prompt diff if it changed
    prompt_section = ""
    if prompt_changed and prev_prompt_body:
        prompt_section = (
            f"---BEGIN PREVIOUS STYLE PROMPT---\n{prev_prompt_body}\n---END PREVIOUS STYLE PROMPT---\n\n"
            f"---BEGIN CURRENT STYLE PROMPT---\n{prompt_body}\n---END CURRENT STYLE PROMPT---\n\n"
        )
    else:
        prompt_section = f"---BEGIN STYLE PROMPT---\n{prompt_body}\n---END STYLE PROMPT---\n\n"

    packed = (
        f"SYSTEM:\n{SYSTEM}\n\n"
        f"{change_analysis}\n"
        f"USER:\n---BEGIN PREVIOUS DRAFT---\n{prev}\n---END PREVIOUS DRAFT---\n\n"
        f"---BEGIN SOURCE DIFF---\n{redact(diff)}\n---END SOURCE DIFF---\n\n"
        f"{prompt_section}"
    )

    outp = pathlib.Path(args.out)
    outp.parent.mkdir(parents=True, exist_ok=True)
    outp.write_text(packed, "utf-8")

    if args.params_out:
        po = pathlib.Path(args.params_out)
        po.parent.mkdir(parents=True, exist_ok=True)
        po.write_text(json.dumps(eff, indent=2), "utf-8")

    # Also write a small summary for humans/agents
    name = args.name or outp.stem
    summ = {
      "name": name,
      "deps": deps,
      "effective": eff
    }
    (outp.parent / f"{name}.context.json").write_text(json.dumps(summ, indent=2), "utf-8")

if __name__ == "__main__":
    main()
