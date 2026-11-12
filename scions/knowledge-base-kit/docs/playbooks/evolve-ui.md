---
status: stable
---
# Playbook: evolve a UI

This playbook describes how to introduce and evolve user interfaces
for a knowledge base **safely and incrementally**.

It assumes familiarity with the
[meta knowledge base](../../README.md).

---

## Step 1: confirm the pain

Before building any UI, write down:

- who is struggling
- what task is slow or error-prone
- how often the problem occurs

If the pain is occasional or speculative, improve markdown navigation instead.

---

## Step 2: identify the smallest viable view

Choose the lowest-impact option that addresses the pain:

- navigation pain → index page or generated table
- query pain → filtered static view
- synthesis pain → generated summary
- contributor pain → clearer entrypoints and write guidance

Avoid jumping directly to interactive UIs.

---

## Step 3: define inputs and outputs

Every UI must clearly declare:

- **inputs**
  - which files it reads
  - which metadata it depends on

- **outputs**
  - which artifacts it generates or edits
  - whether outputs are committed to git

If inputs or outputs are unclear, pause and refine.

---

## Step 4: enforce invariants

Before deployment, verify:

- no hidden state exists outside git
- canonical sources remain authoritative
- lifecycle markers are preserved
- provenance is not obscured

If enforcement is difficult, reduce UI scope.

---

## Step 5: integrate with review workflows

All UI-driven changes should be:

- visible as diffs
- reviewable by humans
- reversible

Automated or agent-driven edits should follow
the same Plan → Patch → Verify discipline used elsewhere.

---

## Step 6: observe and iterate

After introduction:

- watch for new kinds of errors
- note whether contributors rely on the UI correctly
- confirm the UI actually reduces pain

If it does not, remove or simplify it.

---

## Anti-patterns to avoid

- building a UI before understanding the domain
- embedding business logic exclusively in the UI
- bypassing lifecycle or provenance rules
- treating the UI as the source of truth

---

## Related guidance

- [UI views as derived artifacts](../strategies/ui-views.md)
- [Playbook: evolve a knowledge base](evolve-kb.md)
- [Linking policy](../policies/linking.md)
- [Meta knowledge base](../../README.md)
