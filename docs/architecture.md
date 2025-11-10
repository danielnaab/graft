# Architecture

- **Artifacts (grafts)**: directories with `graft.yaml`, templates, and outputs.
- **Materials**: on-disk files under version control (or snapshots written by CI).
- **Derivations**: transformer + evaluated template → output(s).
- **Policy**: determinism, editable regions, attestation.
- **Provenance**: per-output record with inputs/template hashes, transformer ref, policy flags, and attestation.

## DVC & Git
- **Git** holds code, small configs, and most outputs.
- **DVC** is used when outputs/snapshots are large. Graft writes a **`dvc.yaml` scaffold** where each stage calls `graft run <artifact/>` with declared `deps` and `outs`. You initialize/configure DVC (e.g., `dvc init`, `dvc remote add`) outside Graft.
- This lets you use `dvc repro` to recompute derivations and take advantage of DVC’s caching while preserving Graft’s provenance model.
