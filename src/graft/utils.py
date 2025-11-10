from __future__ import annotations
import json, sys, hashlib, pathlib, typing as t, yaml

def load_yaml(path: pathlib.Path) -> dict:
    return yaml.safe_load(path.read_text(encoding="utf-8"))

def print_json(data: dict) -> None:
    json.dump(data, sys.stdout, indent=2, sort_keys=True)
    sys.stdout.write("\n")
