# Philosophy of Design

- **Normal editing stays normal** — edit outputs directly; Graft infers and governs.
- **Determinism first** — pipelines are deterministic by default; nondeterminism is explicit and review‑gated.
- **Auditability** — every finalize yields provenance and an attestation stub.
- **Separation of concerns** — CI ingests external data to files; Graft reads/writes files; DVC handles cache/remote.
- **CLI as contract** — `--json` is the API for agents and automations.
