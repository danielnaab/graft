# Testing Strategy (black-box, pytest)

- **Subprocess-only** — tests execute the `graft` CLI via Python’s `-m` entry point.
- **Temp repos** — copy fixture artifacts into `tmp_path` and run commands.
- **Structured asserts** — prefer JSON (`--json`) and file existence/content checks over internal imports.
- **No network** — use only local files; external systems are out of scope here.
