#!/usr/bin/env python3
import os, subprocess, sys

def has(cmd):
    try:
        subprocess.check_output([cmd, "--version"], stderr=subprocess.STDOUT)
        return True
    except Exception:
        return False

if not has("llm"):
    print("ERROR: `llm` CLI not found in PATH.", file=sys.stderr)
    sys.exit(1)

# Minimal Bedrock env check
aws_vars = ["AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY", "AWS_REGION"]
missing = [v for v in aws_vars if not os.environ.get(v)]
if missing:
    print("WARNING: Missing AWS env vars: " + ", ".join(missing), file=sys.stderr)
    # non-fatal: they might use a profile/role
print("llm CLI and environment check passed.")
