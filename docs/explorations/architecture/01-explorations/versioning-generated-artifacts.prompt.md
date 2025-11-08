---
deps:
  - docs/explorations/architecture/00-sources/current-implementation.md
  - docs/explorations/architecture/00-sources/design-goals.md
  - docs/explorations/architecture/00-sources/open-questions.md
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---

# Deep Exploration: When to Version Control Generated Artifacts

You are a systems architect analyzing the fundamental question: when does it make sense to version control generated artifacts, and what makes graft different from traditional build systems?

## Your Task

Think critically about graft's core assumption: that generated documentation should be committed to git. This is unusual—most build systems put generated files in `.gitignore`.

Be rigorous and honest: when is versioning generated artifacts the right choice vs the wrong choice?

### The Fundamental Question

**Traditional build systems**: Generated files are ephemeral
```
src/main.c → (build) → bin/program
# bin/ is in .gitignore
# Never commit generated artifacts
```

**Traditional data pipelines**: Intermediate outputs are cached, not versioned
```
raw-data.csv → (process) → clean-data.csv → (analyze) → report.pdf
# DVC tracks these, but doesn't commit to git
# dvc.lock is committed, outputs are cached
```

**Graft's approach**: Generated documentation IS versioned
```
sources.md → (graft) → docs/report.md
# docs/report.md is committed to git
# It evolves with the repository
```

**Why is graft different? When is this the right approach?**

### Established Practices: What Do We Commit?

#### Practice 1: Source Code Only

**Philosophy**: Only commit human-authored source, not machine-generated outputs

**Examples**:
- C/C++: Commit `.c`, not compiled binaries
- TypeScript: Commit `.ts`, not transpiled `.js`
- Sass: Commit `.scss`, not compiled `.css`
- Protocol buffers: Commit `.proto`, not generated code

**Rationale**:
- Generated files are deterministic from source
- Adds noise to diffs and merge conflicts
- Increases repo size unnecessarily
- Build step is fast enough to regenerate on demand

**When this makes sense**:
- ✓ Generation is fast (< seconds)
- ✓ Generation is deterministic
- ✓ Generated files are for machines (binaries, compiled code)
- ✓ Everyone has the build tools

#### Practice 2: Commit Some Generated Files

**Philosophy**: Commit generated files that users need without build tools

**Examples**:
- npm packages: Commit transpiled JS alongside TS (for non-build consumers)
- Generated API clients: Commit generated SDKs (convenience for users)
- Vendored dependencies: Commit copies of libraries (for offline builds)
- Database migrations: Commit generated SQL from schema definitions

**Rationale**:
- Not everyone has the build toolchain
- Convenience for downstream consumers
- Generated files ARE the interface (API clients, libraries)
- Deterministic generation, but toolchain is heavy

**When this makes sense**:
- ✓ Generated files are consumed by users without build tools
- ✓ Build toolchain is heavy/complex
- ✓ Generated files are relatively stable
- ✗ Generation is non-deterministic

#### Practice 3: Documentation as Code

**Philosophy**: Commit generated documentation for human consumption

**Examples**:
- OpenAPI → generated HTML docs (sometimes committed)
- JSDoc → HTML documentation (usually not committed)
- Sphinx/MkDocs → Built sites (usually not committed, deployed separately)
- README badges: Dynamic SVGs (not committed, served by shields.io)

**Mixed practice**:
- GitHub Pages: Often commit to `gh-pages` branch
- README.md: Often includes generated sections (TOC, API lists)
- CHANGELOG.md: Sometimes generated from git history, but committed

**When docs are committed**:
- ✓ Documentation lives alongside code (in-repo browsing)
- ✓ Documentation is part of the "source of truth"
- ✓ Want version history of how docs evolved
- ✓ Offline access is important

**When docs are NOT committed**:
- ✓ Documentation is deployed separately (docs site)
- ✓ Generation is deterministic and fast
- ✓ Build step is part of CI/CD
- ✓ Reduces repo noise

#### Practice 4: Data Pipelines (DVC Model)

**Philosophy**: Track outputs, don't version them

**DVC approach**:
```
data/raw.csv → (process) → data/clean.csv → (analyze) → results/report.md
```

**What's committed**:
- ✓ Pipeline definitions (`dvc.yaml`)
- ✓ Dependency hashes (`dvc.lock`)
- ✓ Metrics and plots (small files)

**What's NOT committed** (cached instead):
- ✗ Large data files
- ✗ Intermediate outputs
- ✗ Final reports (often)

**Rationale**:
- Data files are too large for git
- Outputs are reproducible from `dvc.lock`
- Remote cache provides sharing
- Git stays fast and small

**When this makes sense**:
- ✓ Outputs are large (> MBs)
- ✓ Expensive to regenerate, but deterministic
- ✓ Team shares via DVC remote cache
- ✓ Reproducibility via `dvc repro`, not git history

### Graft's Position: Where Does It Fit?

Graft commits generated documentation to git. Why?

**Current rationale** (implicit):
1. Documentation is human-readable text (not binaries)
2. Documentation should be browsable in-repo
3. Documentation evolves with code
4. Want git history of how docs changed
5. Documentation IS the product (not intermediate artifact)

**But this conflicts with**:
- Traditional build systems (don't commit generated files)
- Data pipelines (cache outputs, don't version)
- Many documentation tools (build separately, deploy to docs site)

**What makes graft special?**

### Use Case Analysis: When Versioning Makes Sense

#### Use Case 1: In-Repo Documentation (README, CONTRIBUTING, etc.)

**Scenario**:
```
docs/explorations/architecture/
  00-sources/design-goals.md (human-written)
  01-explorations/external-process.prompt.md (template)
  01-explorations/external-process.md (generated)
  02-final/recommendations.md (generated from explorations)
```

**Question**: Should `external-process.md` and `recommendations.md` be committed?

**Argument FOR versioning**:
- ✓ These are documentation meant to be read in-repo
- ✓ GitHub can render them directly
- ✓ They're part of the project's knowledge base
- ✓ Want history: "How did our thinking evolve?"
- ✓ Diffs show what changed in our understanding
- ✓ Non-LLM users can read without regenerating

**Argument AGAINST versioning**:
- ✗ They're generated, not source of truth
- ✗ Adds noise to PRs (large diffs for generated content)
- ✗ Could be regenerated on-demand
- ✗ Non-deterministic (LLM outputs change)

**Critical question**: Is the history of generated docs valuable, or just noise?

#### Use Case 2: API Documentation

**Scenario**:
```
api-spec.yaml → (graft) → docs/api-reference.md
```

**Traditional approach**:
```
api-spec.yaml → (build) → docs-site/
# Deployed to docs.example.com
# Not committed to git
```

**Graft approach**:
```
api-spec.yaml → (graft) → docs/api-reference.md
# Committed to git
# Browsable in repo
```

**When graft makes sense**:
- ✓ API docs should live in repo (developer convenience)
- ✓ Docs evolve with API changes (same PR)
- ✓ Want docs for every git commit/tag
- ✓ No separate docs site infrastructure

**When traditional makes sense**:
- ✓ Large documentation site (many pages, search, navigation)
- ✓ Deterministic generation (always produces same output)
- ✓ Deployed to dedicated docs site
- ✓ Fast rebuild (can regenerate on every commit via CI)

#### Use Case 3: Release Notes

**Scenario**:
```
git log → (graft) → CHANGELOG.md
# Generated from git history and merged PRs
```

**Traditional approaches**:
- Hand-written CHANGELOG.md (committed)
- Generated via `git cliff` or similar (committed or not)
- GitHub Releases (not in repo, generated via API)

**Graft approach**: Synthesize from git history + human guidance

**Should CHANGELOG.md be committed?**

**Argument FOR**:
- ✓ Changelog is part of the package (npm, PyPI expect it)
- ✓ Users want to see history without git tools
- ✓ Curated changelog is different from raw git log
- ✓ LLM synthesis adds value (summarization, categorization)

**Argument AGAINST**:
- ✗ Can be regenerated from git history
- ✗ Adds large diffs to commits
- ✗ Non-deterministic (LLM-generated)

**Nuance**: Changelog is often hand-edited after generation, so it's semi-generated.

#### Use Case 4: Living Documentation (Architecture Decisions, RFCs)

**Scenario**:
```
docs/explorations/architecture/
  explorations/ (generated from prompts)
  final-recommendation.md (generated synthesis)
```

**These documents**:
- Capture thinking at a point in time
- Evolve as project matures
- Are meant to be read and referenced
- Have historical value (see past decisions)

**Should they be versioned?**

**Argument FOR**:
- ✓ Architecture decisions have historical value
- ✓ Want to see "what were we thinking in v1.0?"
- ✓ These become part of project knowledge
- ✓ Future contributors need context

**Argument AGAINST**:
- ✗ Could store as locked/frozen artifacts (DVC cache)
- ✗ Non-deterministic generation
- ✗ Large diffs clutter git history
- ✗ Could use git tags to mark specific versions

**Alternative**: Commit only hand-curated final docs, not all explorations?

### The Graft Paradox

**Graft's value proposition**: "Living documentation that evolves with your sources"

**But**:
- If docs evolve (regenerate frequently) → git diffs are noisy
- If docs are locked (frozen) → they're not "living"
- If docs are non-deterministic → diffs don't represent source changes

**Question**: Is "living documentation in git" actually coherent?

**Possible resolutions**:

**Resolution 1: Deterministic docs only**
- Only use graft for deterministic transformations (temp=0)
- Non-deterministic explorations go in DVC cache, not git
- Git diffs represent actual content changes, not LLM variability

**Resolution 2: Two-tier system**
- Source-level docs (exploratory, prompt-based) → Not committed
- Final deliverable docs (curated, stable) → Committed
- Graft generates both, user chooses what to commit

**Resolution 3: Commit strategically**
- Commit docs only when "publishing" (releases, milestones)
- Lock published docs (frozen in DVC)
- Working docs stay in cache, not git

**Resolution 4: Embrace the noise**
- Accept that generated docs create diffs
- Use git attributes to hide generated diffs
- Focus on "snapshot at each commit" value

### Comparative Analysis: Build Systems

#### Make / Grunt / Webpack / etc.

**Philosophy**: Inputs are versioned, outputs are ephemeral

```makefile
# Makefile
docs/api.html: api-spec.yaml
    openapi-generator api-spec.yaml > docs/api.html
```

**What's committed**: `Makefile`, `api-spec.yaml`
**What's NOT committed**: `docs/api.html` (in `.gitignore`)

**Why**: Generation is deterministic and fast, no need to version output

**Graft comparison**: Graft versions the output, treating docs as first-class artifacts

#### Jupyter Notebooks

**Philosophy**: Commit notebooks WITH outputs (controversial!)

```json
{
  "cells": [
    {
      "cell_type": "code",
      "execution_count": 1,
      "outputs": [{"data": "..."}]
    }
  ]
}
```

**What's committed**: Notebooks including cell outputs

**Why**: Outputs are valuable for reviewing analysis without re-running

**Controversy**: Outputs can be large, non-deterministic, and create noise

**Common practice**:
- Some teams commit outputs (convenience)
- Some teams strip outputs before commit (`nbstripout`)
- Some teams use nbdime for better diffs

**Graft parallel**: Like Jupyter, graft commits "executed" outputs. Same trade-offs apply.

#### Quarto / R Markdown

**Philosophy**: Commit source, optionally commit rendered output

```yaml
# _quarto.yml
project:
  output-dir: _output  # Often in .gitignore
```

**Options**:
- Commit `.qmd` only, render on CI/deploy
- Commit `.qmd` + rendered HTML (for GitHub Pages)
- Commit `.qmd` + rendered PDF (for distribution)

**Graft parallel**: Similar choice—commit prompts only, or prompts + generated docs?

#### Pandoc / AsciiDoc / Sphinx

**Philosophy**: Source is versioned, output is deployed

```
docs/
  source/
    index.rst (committed)
  build/
    html/ (not committed)
```

**Why**: HTML is deterministic from source, rebuild is fast, deployed separately

**When output IS committed**: GitHub Pages workflows often commit to `gh-pages` branch

**Graft difference**: Graft commits output in SAME branch, alongside source

### When Should Graft Commit Generated Docs?

Let me propose a decision framework:

#### Commit generated docs when:

1. **Docs are the interface** (README, API reference users consume in-repo)
2. **Non-deterministic but curated** (LLM generates, human reviews and commits)
3. **Historical value** (architecture decisions, design explorations)
4. **Slow/expensive generation** (don't want to regenerate on every checkout)
5. **Offline access important** (users clone repo, read docs without build)
6. **Small-to-medium size** (< 100KB per doc, not massive HTML sites)

#### DON'T commit generated docs when:

1. **Fast deterministic generation** (can rebuild in CI, no need to version)
2. **Large outputs** (> MBs, better in DVC cache or deployed separately)
3. **High churn** (regenerate constantly, creates noisy diffs)
4. **Intermediate artifacts** (build steps, not final deliverables)
5. **Deployed separately** (docs site, not in-repo consumption)
6. **Machine-generated noise** (obfuscates human changes in PRs)

### Proposed Graft Modes

**Mode 1: "Source + Output" (current default)**
```
Commit:
  ✓ .prompt.md (source)
  ✓ .md (generated output)

Use when:
  - Docs are meant to be read in-repo
  - Generation is expensive
  - Historical evolution has value
```

**Mode 2: "Source Only"**
```
Commit:
  ✓ .prompt.md (source)
  ✗ .md (in .gitignore)

Build:
  - CI regenerates docs on deploy
  - Or users run `graft rebuild` locally

Use when:
  - Deterministic generation
  - Docs deployed separately (docs site)
  - Want clean git history
```

**Mode 3: "Curated Snapshots"**
```
Commit:
  ✓ .prompt.md (source)
  ✓ .md (only on releases/milestones)

Workflow:
  - Working docs in DVC cache (not committed)
  - On release, commit final docs and lock
  - Tagged versions have committed docs
  - Working branches don't

Use when:
  - Non-deterministic generation
  - Want snapshots at milestones
  - Clean history between releases
```

**Mode 4: "Two-Tier Documentation"**
```
exploratory/
  *.prompt.md (committed)
  *.md (NOT committed, cached in DVC)

docs/
  *.prompt.md (committed)
  *.md (committed, curated final docs)

Use when:
  - Explorations are messy/iterative
  - Final docs are polished deliverables
  - Want both working and published docs
```

### Trade-off Analysis

| Approach | Git History | Repo Size | Offline Access | Determinism | Noise in PRs |
|----------|-------------|-----------|----------------|-------------|--------------|
| Commit all outputs | ✓✓✓ Full | ✗ Large | ✓✓✓ Yes | ✗ Not required | ✗ High |
| Commit none (build only) | ✗ None | ✓✓✓ Small | ✗ No | ✓✓✓ Required | ✓✓✓ None |
| Curated snapshots | ✓✓ Milestones | ✓✓ Medium | ✓✓ Tagged | ✓ Helpful | ✓✓ Low |
| Two-tier | ✓✓ Final docs | ✓ Medium-Small | ✓ Final docs | ✓ Mixed | ✓✓ Medium |

### Research Questions

1. **Do users actually read generated docs in-repo, or do they prefer docs sites?**
   - Value of GitHub rendering vs deployed site

2. **Is git history of generated docs valuable, or just noise?**
   - Do teams review how docs evolved, or just current state?

3. **How do teams handle merge conflicts in generated docs?**
   - Regenerate? Manually resolve? Pain point?

4. **For non-deterministic docs, how often do they actually change?**
   - If temp=0.3, is output stable enough for commits?

5. **What's the size distribution of generated docs?**
   - Are we talking 10KB READMEs or 1MB reference docs?

## Output Requirements

Produce a comprehensive analysis with:

1. **Executive Summary**: When should generated artifacts be version-controlled?
2. **Comparative Analysis**: How do established practices (Make, DVC, Jupyter, Sphinx) handle this?
3. **Use Case Framework**: Decision tree for when to commit vs cache vs deploy
4. **Graft's Unique Position**: What makes graft different from traditional build systems?
5. **The Living Docs Paradox**: Is "evolving docs in git" coherent with non-deterministic generation?
6. **Proposed Modes**: Different graft workflows for different needs
7. **Git Hygiene**: How to manage noise from generated diffs
8. **Team Workflows**: How does committing generated docs affect PRs, reviews, merges?
9. **DVC Integration**: How does graft's versioning interact with DVC caching?
10. **Size and Performance**: What's the impact on repo size and git performance?
11. **Recommendations**: Clear guidance on when to use each approach
12. **Open Questions**: What needs user research or prototyping?

Think like a pragmatic engineer who's worked with many build systems and documentation tools. Be honest about trade-offs. Challenge graft's assumptions if they don't hold up. The goal is to understand when versioning generated docs is genuinely valuable vs when it's just cargo-culting.
