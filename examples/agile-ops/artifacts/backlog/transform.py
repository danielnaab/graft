#!/usr/bin/env python3
"""Transform JIRA JSON to backlog YAML format."""
import json
import yaml
import os
import sys

# Read environment variables
artifact_dir = os.getenv("GRAFT_ARTIFACT_DIR", "/workspace")
params = json.loads(os.getenv("GRAFT_PARAMS", "{}"))
outputs = json.loads(os.getenv("GRAFT_OUTPUTS", "[]"))
materials = json.loads(os.getenv("GRAFT_MATERIALS", "[]"))

# Find JIRA material
jira_path = None
for material in materials:
    if "jira" in material.lower() and material.endswith(".json"):
        jira_path = material
        break

if not jira_path:
    print("Error: No JIRA material found", file=sys.stderr)
    sys.exit(1)

# Load JIRA JSON
try:
    with open(jira_path, "r") as f:
        jira_data = json.load(f)
except FileNotFoundError:
    print(f"Error: JIRA file not found: {jira_path}", file=sys.stderr)
    sys.exit(1)
except json.JSONDecodeError as e:
    print(f"Error: Invalid JSON in JIRA file: {e}", file=sys.stderr)
    sys.exit(1)

# Transform to backlog format
backlog = {"items": []}
for issue in jira_data.get("issues", []):
    # Handle both full JIRA format and simplified format
    if "fields" in issue:
        # Full JIRA API format
        backlog["items"].append({
            "id": issue.get("key"),
            "title": issue["fields"].get("summary"),
            "status": issue["fields"].get("status", {}).get("name")
        })
    else:
        # Simplified format
        backlog["items"].append({
            "id": issue.get("key"),
            "title": issue.get("summary"),
            "status": "unknown"
        })

# Write output
if not outputs:
    print("Error: No output paths specified", file=sys.stderr)
    sys.exit(1)

output_path = outputs[0]  # First output
try:
    with open(output_path, "w") as f:
        yaml.dump(backlog, f, default_flow_style=False)
    print(f"Transformed {len(backlog['items'])} items to {output_path}")
except Exception as e:
    print(f"Error writing output: {e}", file=sys.stderr)
    sys.exit(1)
