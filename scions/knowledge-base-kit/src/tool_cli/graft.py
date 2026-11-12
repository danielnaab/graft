"""Recognize directories graft wrote, so the tools leave them alone."""
# spec: specs/tool-cli.md
# spec-section: Graft directories

from pathlib import Path

# The first line graft >= 2.1 writes in its directory's README.md, and the sentence 2.0 wrote.
# Matches steward's GRAFT_MARKERS (duties/check-kb-integrity/check.ts).
GRAFT_MARKERS = (
    "<!-- managed by graft: https://github.com/danielnaab/graft -->",
    "Managed by [graft]",
)

# How far into the README the markers are looked for: 2.1+ writes its marker on line 1, 2.0
# its sentence near the top.
_HEAD_BYTES = 2048


def is_graft_dir(path: Path) -> bool:
    """True if graft wrote this directory: its README.md carries one of graft's markers.

    A grafted directory is another repo's export, checked by that repo, and graft's own
    `nix flake check` fails if it is edited here, so findings inside it aren't actionable.
    Links into it are still checked by the tools that check links.
    """
    readme = path / "README.md"
    try:
        with readme.open("rb") as f:
            head = f.read(_HEAD_BYTES).decode("utf-8", errors="replace")
    except OSError:
        return False
    return any(marker in head for marker in GRAFT_MARKERS)
