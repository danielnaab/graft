# Slice 6 — Container-Based Transformers (Future)

**Status**: Deferred
**Depends on**: Slice 2 (Python transformers)
**Note**: This is the ChatGPT-designed specification for production-grade container support.

## Intent

Enable each graft (artifact directory) to run its derivations via a container built locally from a co-located Dockerfile. The container acts as the transformer and follows a simple stdin→outputs contract. Policies keep runs deterministic and safe; all activity stays file-first (no side effects beyond the repo).

- One runtime for this slice (agent chooses which).
- Local builds only (from a Dockerfile in the graft).
- Remote images are out of scope and will be introduced in Slice 7.

## Why Deferred to Slice 6?

After implementing Slice 2 (Python transformers), we will have:
- ✅ Both examples working
- ✅ Transformer architecture validated
- ✅ Material loading proven
- ✅ Clear understanding of real transformer requirements

Container support adds significant complexity:
- Container runtime dependency (Docker/Podman/Nerdctl)
- Build management and caching
- Network isolation at container level
- Image digest tracking
- Cross-platform considerations
- ~2000+ lines of additional code

This complexity is valuable for production use but premature before validating the core architecture works with Python transformers.

## Terms

- **Graft (artifact)**: directory with graft.yaml, templates, outputs, and now (by convention) a Dockerfile.
- **Derivation**: one transformation step declared in graft.yaml.
- **Transformer**: the container the runtime builds from the local Dockerfile and then runs to produce outputs.
- **Runner**: the mechanism that builds and runs the transformer.

## Contracts

### 1) graft.yaml (derivation block)

A derivation that uses a container transformer MUST declare a `transformer.build` object. Remote refs are not allowed in this slice.

```yaml
derivations:
  - id: brief
    transformer:
      build:
        image: "graft/sprint-brief:local"  # required local tag/name
        context: "."                       # optional; default is artifact directory
        target: "optional-stage"           # optional multi-stage build target
        args:                              # optional build args
          TITLE: "Sprint Brief"
      params:                              # arbitrary JSON; passed via env
        title: "Sprint Brief"
    template:
      source: file | inline
      engine: jinja2 | none
      content_type: text/markdown | application/json
      file: "./template.md"                # when source: file
      persist: text | never
      persist_path: "./.graft/evaluated/brief-input.md"
    outputs:
      - { path: "./brief.md", schema: sprint_brief }
    policy:
      deterministic: true
      attest: required
      direct_edit: true
```

**Rules**:
- `transformer.build.image` is required and names the local image tag to produce.
- The Dockerfile is assumed to be at `<context>/Dockerfile` (default `./Dockerfile` in the artifact).
- `transformer.ref` is not permitted in this slice (use Slice 7 for remote refs).
- Supplying both `build` and `ref` is an error.

### 2) CLI contract

```bash
graft run <artifact-dir/> [--id <derivation-id>]
```

If a derivation declares `transformer.build`, Graft MUST:
1. Build the local image using the provided `image`, `context`, optional `target`, and `args`, under the current policy
2. Run the container following the runner IO contract (below)

If a derivation has no transformer block, graft run behaves as in Slice 1 (template rendering), preserving backward compatibility.

**Exit codes**:
- `0` = success and all declared outputs exist relative to the artifact directory
- `!=0` = failure (human-readable message); a log MUST be written to `.graft/logs/`

### 3) Runner IO & environment contract (for the container)

**stdin**: the evaluated template bytes

**Environment variables** (strings unless noted):
- `GRAFT_ARTIFACT_DIR` — absolute path of the artifact as seen by the process (e.g., `/workspace`)
- `GRAFT_PARAMS` — JSON string of `transformer.params` (object or `{}`)
- `GRAFT_CONTENT_TYPE` — MIME of stdin
- `GRAFT_OUTPUTS` — JSON array of absolute file paths (as seen by the process) that must be written
- `GRAFT_NETWORK` — "off" when policy forbids networking; otherwise "inherit"
- If `policy.deterministic: true`: reproducibility env MUST be set (e.g., `SOURCE_DATE_EPOCH`, `TZ=UTC`, stable locale)

**Output expectation**:
- The process MUST create/write every path in `GRAFT_OUTPUTS`
- Missing outputs → `E_OUTPUT_MISSING`
- Process exit code 0 indicates success; non-zero indicates run failure

(Contract is backend-agnostic and does not prescribe exact mount flags or user options.)

### 4) Determinism & network policy

**If `policy.deterministic: true`**:
- Network-off MUST apply to both build and run phases
- If the selected runtime cannot honor a no-network build, error with `E_BUILD_NETWORK_FORBIDDEN`
- Run record MUST capture:
  - Hash of the Dockerfile and a manifest hash of the build context (ordered file list + hashes)
  - Serialized build args (key/value strings)
  - Resulting image digest/ID as reported by the runtime after build

**If `policy.deterministic: false`**:
- Builds are allowed
- The run record MUST include the resulting image identifier/digest

**Quality guidance** (non-binding but required intent):
- The tool SHOULD skip rebuilding when Dockerfile, context, and build args are unchanged
- The tool SHOULD strive for hermeticity (no hidden host deps)
- If not achievable, the run record MUST reflect determinism limits

### 5) Logging & run records

**On any failure**, capture output and write a log:
```
.graft/logs/<derivation-id>-<timestamp>.log
```

**On success or failure**, write a run record:
```
.graft/runs/<derivation-id>/<timestamp>.json
```

**Minimum fields**:
- `artifact_path`, `derivation_id`
- `transformer: { build: { image, context, args?, target? } }`
- `policy: { deterministic: bool, network: "off"|"inherit" }`
- `template_hash`, `input_hashes[]`, `output_paths[]`
- `image: { digest: "<id-or-digest>" }`
- `status: success | failure`, and `log_path` if failure

(Exact hashing algorithms may be defined later; they must be stable.)

### 6) Root configuration

A single runtime backend is configured at the project root:

```yaml
# graft.config.yaml
version: 1
runtime:
  oci_runner: <one-selected-runtime>   # e.g., "docker", "podman", "nerdctl"
```

Multiple backends and auto-detection are out of scope for this slice.

### 7) Schema guidance

**schemas/transformer.schema.json** (new/updated):

```json
{
  "type": "object",
  "required": ["build"],
  "properties": {
    "build": {
      "type": "object",
      "required": ["image"],
      "properties": {
        "image": { "type": "string" },
        "context": { "type": "string" },
        "target": { "type": "string" },
        "args": {
          "type": "object",
          "additionalProperties": { "type": "string" }
        }
      },
      "additionalProperties": false
    },
    "params": {
      "type": "object",
      "additionalProperties": true
    }
  },
  "additionalProperties": false
}
```

### 8) DVC interaction

Unchanged: DVC stages shell out to `graft run <artifact/>`. Determinism and network enforcement apply to build and run phases inside Graft.

## Errors

- `E_IMAGE_NAME_REQUIRED` — `transformer.build.image` missing
- `E_TRANSFORMER_REF_NOT_ALLOWED` — `transformer.ref` provided; remote images not supported in this slice
- `E_TRANSFORMER_CONFIG_CONFLICT` — both build and ref specified (disallowed here)
- `E_BUILD_FAILED` — build step failed; see `.graft/logs/...`
- `E_BUILD_NETWORK_FORBIDDEN` — build attempted with network when policy requires it off
- `E_OUTPUT_MISSING` — at least one declared output not produced
- `E_RUNNER_NOT_AVAILABLE` — configured runtime cannot be used on this system
- `E_EXEC_FAILED` — container run failed; see `.graft/logs/...`

## Backward Compatibility

- Derivations without a transformer block behave as in Slice 1 (template rendering)
- Derivations with Python transformers (Slice 2) continue to work
- Derivations with a `transformer.build` block follow this slice's contract

## Repo Conventions

- Place one Dockerfile per graft next to graft.yaml:
  ```
  examples/.../artifacts/<graft>/Dockerfile
  ```
- Use a simple local image tag per graft (e.g., `graft/<name>:local`)
- Keep examples offline (no network) to reflect the deterministic baseline

## Acceptance Criteria

1. **Image name is required**
   - Given a derivation with `transformer.build` but no `image`
   - When `graft run <artifact/>`
   - Then it fails with `E_IMAGE_NAME_REQUIRED`

2. **Build + run produces all outputs**
   - Given a derivation with a Dockerfile at `<artifact>/Dockerfile` and `transformer.build.image` set
   - When `graft run <artifact/>`
   - Then exit code is 0 and every file in `outputs[].path` exists

3. **Missing output fails**
   - If any declared output is not written by the transformer
   - Then `graft run` fails with `E_OUTPUT_MISSING`

4. **Network disabled under determinism**
   - Given `policy.deterministic: true` (or an explicit network-off policy)
   - When `graft run <artifact/>`
   - Then both build and run occur with networking disabled
   - `GRAFT_NETWORK=off` is present during execution

5. **Run records & logs**
   - On failure, a log file exists under `.graft/logs/...` referenced by the CLI error
   - On success or failure, a run record exists under `.graft/runs/<derivation-id>/...` containing at least the fields listed above

6. **Remote refs disallowed**
   - Given a derivation with `transformer.ref`
   - When `graft run`
   - Then it fails with `E_TRANSFORMER_REF_NOT_ALLOWED`

## Rationale

- Per-graft Dockerfiles keep derivations self-describing and portable
- Recording build inputs and the resulting image digest yields strong, reviewable provenance
- Keeping the contract backend-agnostic and single-runtime now lets us add multi-backend and remote image support later without changing artifacts or CLI shape

## Implementation Notes

This will require:
- `ContainerTransformerRegistry` implementing `TransformerPort`
- Docker/Podman client integration
- Build caching logic
- Network isolation configuration
- Run record persistence
- Comprehensive error handling

Estimated: ~2000+ lines of code, 2-3 weeks of development.

## Credit

Original specification designed by ChatGPT, adapted for Graft's vertical slice approach.
