# Project Essence

## What This Tool Does

This is an **LLM-powered documentation pipeline** that treats documentation as code. It enables users to write Markdown prompt files (`.prompt.md`) with YAML frontmatter specifying source dependencies, and automatically generates corresponding output documents using Claude AI via AWS Bedrock.

The tool orchestrates a sophisticated pipeline that:
- Detects what changed (source content vs prompt instructions) using git
- Applies intelligent actions (GENERATE, UPDATE, RESTYLE, REFRESH, MAINTAIN)
- Maintains reproducible documentation pipelines using DVC
- Keeps all artifacts version-controlled in git

## Core Philosophy

The project embodies five key principles:

1. **Git-Based Change Detection** - Understand what changed and respond appropriately, minimizing unnecessary regeneration
2. **Declarative Configuration** - Define dependencies in YAML frontmatter; pipeline configs auto-generate
3. **Intelligent Action Selection** - Different operations for different change types (sources vs prompts)
4. **Multi-Level Documentation DAGs** - Documents can depend on other generated documents, creating hierarchies
5. **Version Control First** - All outputs git-tracked; no hidden caches; full reproducibility

## What Makes This Unique

Unlike traditional static site generators or documentation tools, this project:

- **Synthesizes multiple sources intelligently** - Uses AI to combine and transform content, not just template it
- **Preserves semantic intent during updates** - When sources change, it updates content while maintaining structure
- **Separates content changes from style changes** - Different operations for different types of edits
- **Leverages existing powerful tools** - Built on DVC (data pipelines) + Claude (AI) rather than reinventing
- **Treats documentation as a data flow problem** - Dependencies, stages, reproducibility, version control

## Future Vision

While starting with Markdown knowledge bases, the architecture is designed for:

- **Git-backed, PR-reviewed content workflows** for any content type
- Integration with diverse content sources (code, APIs, databases, query results)
- Multi-team documentation synthesis across repos
- Content governance with validation and approval workflows
- Template systems for common documentation patterns

The vision is a **content pipeline orchestration framework** that brings software engineering rigor to any content workflow.

## Key Metaphors and Mental Models

**Documentation as Data Pipeline**: Just as data pipelines transform inputs through stages, this tool transforms source documents through AI-powered generation stages.

**Git as the Single Source of Truth**: Everything is version-controlled. Changes are detected through git. History is preserved in commits.

**Declarative Dependencies**: Like make, Bazel, or Nix - declare what depends on what, system figures out execution.

**Intelligent Regeneration**: Not just "rebuild everything" but "understand what changed and apply the minimal, correct update."

**Content DAG**: Documentation forms a directed acyclic graph where changes cascade intelligently through layers.

## Technical Architecture Highlights

The system is elegant in its simplicity:

1. **Prompt files** (`.prompt.md`) = declarative specifications
2. **DVC pipeline** = automatically generated from prompts
3. **Change detector** = git-aware, analyzes diffs
4. **Prompt packer** = assembles context with change directives
5. **LLM renderer** = Claude generates outputs
6. **Git** = tracks everything, enables reproducibility

Everything runs in Docker for consistency. Git hooks keep pipeline configs synchronized. DVC handles dependency ordering and caching.

## What This Tool Is NOT

- Not a static site generator (Hugo, Jekyll) - generates intermediate docs, not sites
- Not a documentation hosting platform (GitBook, Read the Docs) - generates content, doesn't host
- Not a simple template engine (Jinja, Mustache) - uses AI for synthesis, not just substitution
- Not a CMS (Contentful, Strapi) - focused on git-backed workflows, not UI editing
- Not a build system (Make, Bazel) - uses DVC for builds, adds AI-powered content generation

## The Name Should Evoke...

- **Orchestration and flow** - Content moving through a pipeline
- **Git and version control** - The foundation of the approach
- **Intelligence and synthesis** - AI transforming content
- **Reproducibility and rigor** - Engineering discipline applied to docs
- **Simplicity and elegance** - The architecture is actually quite simple
- **Trust and reliability** - Version-controlled, auditable, reproducible

Ideally, the name hints at both the "content pipeline" aspect and the "git-backed workflow" aspect without being overly literal or technical.
