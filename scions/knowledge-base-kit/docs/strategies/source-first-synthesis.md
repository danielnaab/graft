---
status: stable
---
# Strategy: Source-first synthesis

Canonical sources are primary. Syntheses reference sources at a useful granularity.

## When it works well
- correctness and auditability matter
- the repo has canonical artifacts (code, data files, contracts)

## Minimal implementation
- declare canonical sources in your domain `knowledge-base.yaml`
- require Sources sections for important claims (see [Provenance policy](../policies/provenance.md))
- keep syntheses short and focused

## Risks
- manual upkeep (if not automated later)

## Recommended safeguards
- [Authority policy](../policies/authority.md)
- [Provenance policy](../policies/provenance.md)
- [Linking policy](../policies/linking.md)

Next:
- [Metadata + views](metadata-and-views.md)
