---
deps:
  - architecture-exploration/00-sources/current-implementation.md
  - architecture-exploration/00-sources/design-goals.md
  - architecture-exploration/00-sources/open-questions.md
  - architecture-exploration/01-explorations/unique-value-proposition.md
  - architecture-exploration/01-explorations/versioning-generated-artifacts.md
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---

# Deep Exploration: Graft's Core Use Cases and Design Validation

You are a product architect validating graft's design against real-world use cases and user needs.

## Your Task

Move from abstract architectural possibilities to concrete use cases. Which problems does graft actually solve? For which users? How should these use cases drive architectural decisions?

Be specific and grounded. Use real examples, not hypotheticals.

### The Core Question

**What documentation problems do real teams face that graft could solve?**

Not: "What COULD graft do?"
But: "What SHOULD graft do to solve real pain?"

### Real-World Documentation Pain Points

#### Pain Point 1: Stale Documentation

**Problem**: Docs fall out of sync with code

**Examples**:
- API reference says endpoint exists, but it was removed
- Architecture diagram shows old system design
- README lists features that no longer exist
- Onboarding guide references deprecated tools

**Why it happens**:
- Writing docs is boring
- Updating docs is forgotten
- No forcing function (code breaks, docs don't)
- Docs are separate from code changes

**How graft could help**:
```yaml
# docs/api-reference.prompt.md
---
deps:
  - src/api/routes.ts
  - api-spec.yaml
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---

Generate API reference from current code and spec.
When routes.ts changes, this regenerates automatically.
```

**Validation questions**:
- Does automatic regeneration solve staleness?
- Or do docs need human judgment to stay accurate?
- Is "regenerate when code changes" the right forcing function?

#### Pain Point 2: Multi-Source Documentation

**Problem**: Documentation requires synthesizing many sources

**Examples**:
- Release notes need: git commits + issue tracker + breaking changes doc
- Onboarding guide needs: architecture + setup scripts + team practices
- API docs need: OpenAPI spec + code comments + usage examples
- Troubleshooting guide needs: logs + known issues + runbooks

**Current approaches**:
- Manual copy-paste (tedious, error-prone)
- Template-based generation (rigid, low-quality prose)
- Hand-written (time-consuming, difficult to maintain)

**How graft could help**:
```yaml
# docs/onboarding.prompt.md
---
deps:
  - architecture/system-overview.md
  - SETUP.md
  - team/practices.md
  - infrastructure/tooling.md
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---

Create comprehensive onboarding guide that:
- Explains system architecture (from overview)
- Provides setup instructions (from SETUP.md)
- Describes team practices (from practices.md)
- Lists required tooling (from tooling.md)
```

**Validation questions**:
- Is LLM synthesis better than manual writing?
- Does regeneration on source changes maintain quality?
- Is this more maintainable than hand-written docs?

#### Pain Point 3: Documentation Hierarchy

**Problem**: Need docs at multiple levels of detail

**Examples**:
- Executive summary → Technical deep dive → Implementation details
- Overview → Component guides → API reference
- Getting started → Advanced topics → Troubleshooting
- Quarterly review → Weekly updates → Daily logs

**Current approaches**:
- Write each level separately (duplication, inconsistency)
- Link between docs (manual, breaks easily)
- Extract summaries (manual, labor-intensive)

**How graft could help**:
```yaml
# Detailed explorations
explorations/option-a.prompt.md → option-a.md
explorations/option-b.prompt.md → option-b.md

# Mid-level synthesis
analysis/comparison.prompt.md
  deps: [explorations/*.md]
  → comparison.md

# Executive summary
final/recommendation.prompt.md
  deps: [analysis/comparison.md]
  → recommendation.md
```

**Validation questions**:
- Does DAG structure match how teams actually think?
- Is cascading regeneration (detailed → summary) desirable?
- Or do different levels need different authoring?

#### Pain Point 4: Consistency Across Docs

**Problem**: Documentation has inconsistent style, terminology, structure

**Examples**:
- Some docs use "user", others "client"
- Inconsistent formatting and structure
- Different levels of detail for similar topics
- Tone varies by author

**Current approaches**:
- Style guides (often ignored)
- Editorial review (slow, bottleneck)
- Templates (rigid, not always followed)

**How graft could help**:
- LLM applies consistent style across all docs
- Shared system prompt defines tone and terminology
- Regeneration ensures consistency maintained

**Validation questions**:
- Is LLM-enforced consistency valuable?
- Or does it homogenize docs in a bad way?
- Can LLM maintain nuance while being consistent?

#### Pain Point 5: Documentation for Multiple Audiences

**Problem**: Need different docs for different readers

**Examples**:
- Developers vs product managers vs executives
- Internal vs external (customers, partners)
- Beginner vs advanced users
- Different departments (eng, sales, support)

**Current approaches**:
- Write separate docs (duplication, drift)
- Single doc with sections (too long, unfocused)
- Link between docs (fragmentation, hard to navigate)

**How graft could help**:
```yaml
# Same sources, different prompts
docs/developer-guide.prompt.md
  deps: [api-spec.yaml, examples/]
  audience: developers
  focus: implementation details

docs/integration-guide.prompt.md
  deps: [api-spec.yaml, examples/]
  audience: partners
  focus: business value, getting started
```

**Validation questions**:
- Is audience-specific synthesis valuable?
- Or do audiences need fundamentally different information?
- Can one prompt set + different audiences really work?

### User Personas and Their Needs

#### Persona 1: Solo Developer / Small Team

**Context**:
- 1-5 person team
- Need documentation but hate writing it
- Want "good enough" docs quickly
- Documentation is secondary to shipping code

**Pain points**:
- README is outdated
- No architecture docs (all in my head)
- New team members struggle to onboard
- Investors/users ask for docs, we scramble

**How graft could help**:
- Generate README from code + brief notes
- Create architecture docs from code structure + design notes
- Keep docs updated automatically as code changes

**Requirements**:
- ✓ Easy setup (< 30 min)
- ✓ Minimal configuration
- ✓ Works with existing tools (git, no new infra)
- ✓ Cheap (don't burn LLM credits on every change)

**Anti-requirements**:
- ✗ Complex pipeline configuration
- ✗ Need to learn DVC
- ✗ Requires dedicated tooling/infrastructure

#### Persona 2: Documentation Team in Medium Company

**Context**:
- Dedicated technical writers
- 50-200 person engineering org
- Docs are important (product requirement)
- Multiple products, shared architecture

**Pain points**:
- Developers don't update docs when code changes
- Writers aren't in the code review loop
- Docs are always catching up
- Hard to maintain consistency across products

**How graft could help**:
- Automatic drafts when code changes (writers review/approve)
- Dependency tracking ensures completeness
- Consistent style across all products
- Writers focus on high-value content, not updates

**Requirements**:
- ✓ Review workflow (gen → review → approve → commit)
- ✓ Version control integration (PRs, reviews)
- ✓ Quality control (metrics, validation)
- ✓ Team collaboration (shared prompts, caching)

**Anti-requirements**:
- ✗ Fully automated (need human oversight)
- ✗ Replace writers (augment, don't replace)

#### Persona 3: Research Team / Knowledge Workers

**Context**:
- Researchers, analysts, strategists
- Generate insights from many sources
- Documentation is the output (reports, syntheses)
- Knowledge work, not software development

**Pain points**:
- Reading many papers/sources manually
- Synthesizing insights is time-consuming
- Hard to track how understanding evolved
- Difficult to update analysis when new data arrives

**How graft could help**:
- Synthesize literature review from PDFs
- Track research evolution (git history)
- Update analysis when new papers added
- Multi-level synthesis (papers → themes → recommendations)

**Requirements**:
- ✓ PDF/image support (read research papers)
- ✓ Long context (research papers are long)
- ✓ Quality synthesis (not just summarization)
- ✓ Provenance tracking (cite sources)

**Anti-requirements**:
- ✗ Code integration (not writing software)
- ✗ API docs (not documenting APIs)

#### Persona 4: Open Source Maintainer

**Context**:
- Maintain popular OSS project
- Users demand good docs
- Limited time (volunteer or part-time)
- Docs affect adoption and support burden

**Pain points**:
- README and docs are never complete
- Users ask questions answered in docs (but outdated)
- Release notes are tedious
- Contributing guide is stale

**How graft could help**:
- Generate API reference from code
- Create changelogs from git history + PR descriptions
- Update examples when API changes
- Keep contributing guide in sync with actual process

**Requirements**:
- ✓ Free or cheap (OSS budget)
- ✓ GitHub integration (PRs, Actions)
- ✓ Easy for contributors (simple `.prompt.md`)
- ✓ Good defaults (works without heavy config)

**Anti-requirements**:
- ✗ Paid tools required
- ✗ Complex setup (barriers to contribution)

### Use Case Validation

Let me validate graft's architecture against these use cases:

#### Validation 1: Automatic Dependency Tracking (DVC)

**Use case need**: When API spec changes, update API docs

**Current graft**: Frontmatter deps + DVC pipeline
```yaml
---
deps: [api-spec.yaml]
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---
```

**Assessment**: ✓ This works well
- DVC handles dependency tracking
- Automatic regeneration when deps change
- Supports DAG (multiple docs depending on same source)

**Architectural decision validated**: Using DVC for dependencies is correct

#### Validation 2: Multi-Source Synthesis (Prompt Packing)

**Use case need**: Combine code + spec + examples into docs

**Current graft**: pack_prompt.py includes all deps
```yaml
---
deps:
  - api-spec.yaml
  - src/api/*.ts
  - examples/*.ts
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---
```

**Assessment**: ✓ This works, but could be better
- Prompt packing combines sources
- LLM sees full context
- But: context window limits (can't include huge codebases)

**Potential improvement**: Smart source selection (relevance filtering)

#### Validation 3: Change Detection (GENERATE/UPDATE/REFINE)

**Use case need**: When spec changes, patch docs; when prompt changes, regenerate

**Current graft**: Git-based change detection

**Assessment**: ? Unclear if valuable in practice
- Theoretically nice: patch vs regenerate
- But: Does it matter? Regeneration might be fine
- And: Non-deterministic LLMs make patching unreliable

**Validation question**: Do users actually value UPDATE vs REFINE distinction?

#### Validation 4: Versioning Outputs (Commit to Git)

**Use case need**: In-repo docs, version history, offline access

**Current graft**: Generated `.md` files are committed

**Assessment**: ⚠️  Works for some use cases, not others
- ✓ Good for: in-repo READMEs, browsable docs
- ✗ Bad for: large doc sites, non-deterministic outputs, frequent regen

**Validation question**: Should this be configurable? (commit vs cache)

#### Validation 5: Team Collaboration (Shared Prompts)

**Use case need**: Multiple people write prompts, share generated docs

**Current graft**: Prompts in git, outputs in git (or DVC cache)

**Assessment**: ✓ Git works well for collaboration
- Prompts are code-reviewed
- Outputs can be reviewed in PRs
- DVC remote cache for sharing expensive outputs

**Architectural decision validated**: Git-native approach is good

#### Validation 6: Cost Control (LLM Credits)

**Use case need**: Don't regenerate unnecessarily

**Current graft**: DVC only runs changed stages

**Assessment**: ✓ Good, but could be better
- DVC skips unchanged stages
- But: No cost awareness (doesn't know stage is expensive)
- And: Non-deterministic jobs might benefit from cache_ttl

**Potential improvement**: Cost-aware regeneration, lock expensive stages

### Architectural Decisions from Use Cases

Based on use case analysis, what should graft prioritize?

#### Priority 1: Simple Setup (Solo Developer Needs)

**Decision**: Graft should work with minimal configuration
- `graft init` sets up project
- Sensible defaults (model, temperature, etc.)
- Work without DVC remote (local-only is fine)

**Implications**:
- Don't require complex DVC setup
- Don't require AWS credentials (support local models?)
- Docs should get users productive in < 30 min

#### Priority 2: Review Workflow (Documentation Team Needs)

**Decision**: Generated docs should be reviewable before commit

**Implications**:
- Maybe: Don't auto-commit generated docs
- Or: Generate in branch, open PR automatically
- Or: CI generates docs, humans review diff

**Architecture question**: Is graft a "regenerate locally" or "CI generates" tool?

#### Priority 3: Quality Control (All Personas)

**Decision**: Users need visibility into generation quality

**Implications**:
- Metrics: How much did the doc change?
- Validation: Are there broken links, missing sections?
- Diffing: Show what changed and why
- Provenance: Which sources contributed to this output?

**Architecture addition**: Build artifacts should include quality metrics

#### Priority 4: Selective Regeneration (Cost Control)

**Decision**: Users need fine-grained control over what regenerates

**Implications**:
- Lock mechanism (expensive stages)
- Selective regen: `graft rebuild docs/api-reference.md`
- Force flags: `graft rebuild --force`
- TTL for time-based regen

**Architecture validation**: Lock mechanism is needed

#### Priority 5: Multi-Level DAGs (Research Team Needs)

**Decision**: Support cascading synthesis (sources → analysis → summary)

**Implications**:
- DAG visualization: show dependency graph
- Partial regeneration: only affected downstream docs
- Multi-stage prompts: intermediate outputs as deps

**Architecture validation**: DVC DAG + multi-file outputs are important

### Use Cases Graft Should NOT Target

Be honest about what graft isn't for:

#### NOT: Large-Scale Doc Sites

**Why not**:
- Doc sites need themes, search, navigation
- MkDocs, Docusaurus do this well
- Graft is synthesis, not presentation

**If users want this**: Graft could generate Markdown, then MkDocs renders it

#### NOT: Real-Time Interactive Docs

**Why not**:
- Graft is batch-oriented (regenerate on change)
- RAG systems handle query-time generation better
- Graft doesn't do interactive Q&A

**If users want this**: Use LlamaIndex or custom RAG, not graft

#### NOT: Code Generation

**Why not**:
- Code generation is different from documentation
- Different validation (code must execute)
- Different workflows (testing, compilation)

**If users want this**: Different tool (copilot, cursor, aider)

#### NOT: Business Intelligence / Data Analysis

**Why not**:
- BI tools (Tableau, Looker) have visualizations
- Graft generates text, not dashboards
- Different user base (analysts, not developers)

**If users want this**: Use BI tools or Jupyter

### Prioritized Feature List from Use Cases

Based on use case validation:

**Tier 1: Must-Have** (solves core use cases)
1. ✓ Dependency tracking (DVC)
2. ✓ Multi-source prompt packing
3. ✓ LLM synthesis
4. ✓ Git-native workflow
5. ✓ Simple setup

**Tier 2: High-Value** (makes graft great)
6. Lock mechanism (cost control)
7. Multi-file outputs (DAG synthesis)
8. DVC remote cache (team sharing)
9. Quality metrics (validation)
10. Selective regeneration

**Tier 3: Nice-to-Have** (polish, convenience)
11. Review workflow (CI integration)
12. TTL-based regeneration
13. Determinism tracking
14. Experiments (multi-sampling)
15. Cost estimation

**Tier 4: Maybe** (unclear value)
16. UPDATE vs REFINE distinction
17. External process pipeline
18. Versioning all outputs (might be anti-pattern)
19. Per-output locking
20. Complex naming conventions

### Use Case-Driven Recommendations

#### Recommendation 1: Focus on "Multi-Source README" Use Case

**Why**: Highest pain, clearest value, simplest to understand

**Example**:
```yaml
# README.prompt.md
---
deps:
  - package.json (metadata)
  - src/ (code structure)
  - CONTRIBUTING.md (how to contribute)
  - CHANGELOG.md (recent changes)
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---

Generate comprehensive README with:
- Project description
- Installation instructions
- Usage examples (from code)
- Contributing guide
- Recent changes
```

**This use case validates**:
- Multi-source synthesis ✓
- Dependency tracking ✓
- In-repo docs (versioned) ✓
- Automatic updates ✓

#### Recommendation 2: Support "Research Synthesis" Workflow

**Why**: Unique value, graft's DAG structure shines here

**Example**:
```
papers/*.pdf → themes/security.md
              → themes/performance.md
              → themes/usability.md

themes/*.md → synthesis/recommendations.md
```

**This use case validates**:
- Multi-level DAG ✓
- Multi-file outputs ✓
- PDF support ✓
- Evolution tracking (git history) ✓

#### Recommendation 3: De-Prioritize "Full Doc Sites"

**Why**: Not graft's strength, other tools do this well

**Instead**: Graft generates Markdown, other tools render
```
graft regenerate
mkdocs build  # or docusaurus, etc.
```

**This keeps graft focused**: Synthesis, not presentation

### Open Questions for User Research

1. **Do teams want automatic regeneration, or on-demand?**
   - CI auto-generates and opens PRs?
   - Or developers run `graft rebuild` locally?

2. **Is UPDATE vs REFINE actually useful?**
   - Or do people just regenerate everything?

3. **Should outputs be committed to git by default?**
   - Or cached in DVC by default?

4. **What's the ideal prompt format?**
   - Current `.prompt.md` frontmatter?
   - Or more structured (YAML files)?

5. **Do users want metrics and validation?**
   - "This doc changed 50%, review carefully"
   - Or is that noise?

6. **Is lock mechanism the right abstraction?**
   - Or do users just want `--skip-expensive` flag?

## Output Requirements

Produce a comprehensive analysis with:

1. **Executive Summary**: What are graft's core use cases?
2. **Pain Points**: What real problems does graft solve?
3. **User Personas**: Who uses graft and why?
4. **Use Case Validation**: Do proposed features match real needs?
5. **Architectural Validation**: Which design decisions are validated by use cases?
6. **Priority Features**: Tier 1 (must-have) through Tier 4 (maybe)
7. **Anti-Use Cases**: What should graft NOT try to do?
8. **Focus Recommendations**: Which use cases to prioritize?
9. **User Research Questions**: What needs validation with real users?
10. **Success Metrics**: How to measure if graft solves these problems?

Think like a product manager validating hypotheses with real user needs. Features are only valuable if they solve real problems. Be ruthless about cutting features that don't have clear use case validation.
