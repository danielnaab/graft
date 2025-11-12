# Composable Intelligence Workflows Across Organizational Boundaries

## The Narrative

A cybersecurity firm has built sophisticated threat intelligence pipelines: ingesting CVE feeds, OSINT sources, vendor advisories, producing normalized threat databases. Other organizations want this intelligence, but with their own extensions—industry-specific threats, their infrastructure context, their risk scoring models.

Traditional approach: download their data dumps, build your own pipelines, maintain them yourself. Or: pay for their hosted SaaS, lose control and customization.

Neither option enables the pattern you need: **composable workflows where you build on others' work without forking**.

## The Pattern Graft Enables

The security firm publishes their normalization grafts as a public git repository. Tagged releases, semantic versioning. Policy: deterministic transformers, auto-merge enabled. This is their data workflow SDK.

Your organization references their grafts as materials:

```yaml
inputs:
  materials:
    # Upstream: ThreatCorp's normalized CVE feed
    - path: "https://github.com/threatcorp/intel/raw/v2.3.0/normalized/cves.json"
      rev: v2.3.0
```

You add your own transformation layer:

```yaml
derivations:
  - id: internal-risk-assessment
    transformer:
      build: { image: "our-risk-model:local" }
    inputs:
      materials:
        - path: "https://github.com/threatcorp/intel/raw/v2.3.0/normalized/cves.json"
          rev: v2.3.0
        - path: "../../internal/infrastructure-inventory.yaml"
    outputs:
      - { path: "./risk-report.md" }
    policy:
      attest: required
```

Your transformer combines: upstream threat data + your infrastructure context + your risk model. Your security team reviews and finalizes with attestation.

When ThreatCorp releases v2.4.0 (improved normalization logic), you can:
- Update the material ref to `v2.3.0` → `v2.4.0`
- Re-run your transformer (it gets the new upstream data)
- Review the changes (what's different in risk assessment?)
- Finalize with approval

Provenance shows: "Our February risk assessment used ThreatCorp intel v2.3.0, our infrastructure snapshot from 2025-02-01, finalized by Jane (Security Analyst) on 2025-02-15."

If you discover an improvement to the risk model, you can contribute back to ThreatCorp. If they publish new data sources, you pull them. Workflow supply chain.

## What's Novel

**Workflow composition via git** — Not just data sharing, but transformation logic as versionable, referenceable units.

**Extend, don't fork** — Build on upstream workflows, track exact versions, upgrade when ready.

**Provenance across boundaries** — Your audit trail includes external dependencies with exact versions.

**Supply chain for intelligence** — Same patterns as code dependencies (semver, pinning, security advisories) applied to data workflows.

## The Wow

"This is how organizations share and compose data workflows like they share code libraries."

You're not just consuming data—you're composing workflows. Upstream provides the foundation, you add your layers, full provenance connects it all. When they improve, you benefit. When you innovate, you can share back.

---

See also:
- [Core Concepts](../concepts.md) for remote material references
- [Workflows](../workflows.md) for composition patterns
