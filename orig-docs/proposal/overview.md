# Proposal: Graft — Auditable, File-First Derivations

**Thesis**: A *graft* is like a branch on a git tree, growing as it is fed new source material. It is both a recipe and a directory of files that evolves naturally with its sources. **Graft** (the tool) coordinates people, agents, CI, and deterministic transformations to evolve those files with **provenance** and **safety**.

## What Graft does (high level)
- Runs **deterministic derivations** from versioned materials to outputs.
- Allows **direct edits** to outputs for human/agent-authored context; classifies those edits and enforces **editable regions**.
- Records **provenance + attestation** when you finalize.
- Computes downstream **impact** and can **simulate** cascades without touching the repo.
- Integrates with **DVC** for large files and cacheable artifacts by generating `dvc.yaml` stages around `graft run` (no side effects).
