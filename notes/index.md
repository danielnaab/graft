---
title: "Notes Index"
status: working
---

# Notes

Working notes, brainstorming sessions, design analysis, and explorations for the Graft project.

Insights should graduate to [docs/](../docs/) when they stabilize.

## Architecture & Repository

- [Implementation Language and Repo Structure (2026-02-08)](./2026-02-08-implementation-language-and-repo-structure.md) - Decision to rewrite in Rust with monorepo structure
- [Binary Architecture and Composition (2026-02-08)](./2026-02-08-binary-architecture-and-composition.md) - Library-first architecture with separate binaries

## Grove (Workspace Tool)

- [Grove Slice 1 Review Phase 2 (2026-02-10)](./2026-02-10-grove-slice-1-review-phase-2.md) - Comprehensive second-phase review of Grove Slice 1
- [Grove Slice 1 Improvement Plan (2026-02-10)](./2026-02-10-grove-slice-1-improvement-plan.md) - Improvement plan for Grove Slice 1
- [Grove Slice 1 Implementation (2026-02-10)](./2026-02-10-grove-slice-1-implementation.md) - Implementation notes for Grove Slice 1
- [Grove Slice 1 Review (2026-02-10)](./2026-02-10-grove-slice-1-review.md) - Initial review of Grove Slice 1
- [Status Check Syntax Exploration (2026-02-08)](./2026-02-08-status-check-syntax-exploration.md) - Deep exploration of status check syntax alternatives; concludes that simple scripts are clearest
- [Grove as Workflow Hub: Design Primitives (2026-02-07)](./2026-02-07-grove-workflow-hub-primitives.md) - Six simple primitives for Grove as workflow hub
- [Grove Vertical Slices (2026-02-06)](./2026-02-06-grove-vertical-slices.md) - Seven narrow end-to-end slices for building Grove incrementally
- [Workspace UI Exploration (2026-02-06)](./2026-02-06-workspace-ui-exploration.md) - "Grove" workspace tool: TUI-first multi-repo navigation and graft awareness

## Design Analysis

- [Flat-Only Dependency Analysis (2026-01-31)](./2026-01-31-flat-only-dependency-analysis.md) - Comprehensive exploration of flat-only dependency model → **Graduated to [Decision 0007](../docs/specifications/decisions/decision-0007-flat-only-dependencies.md)**
- [One-Level Dependency Exploration (2026-01-31)](./2026-01-31-one-level-dependency-exploration.md) - Initial analysis of simplified dependency model
- [Dependency Management Exploration (2026-01-12)](./2026-01-12-dependency-management-exploration.md) - Evaluated git submodules and artifact-based composition
- [Design Improvements Analysis (2026-01-05)](./2026-01-05-design-improvements-analysis.md) - Comprehensive analysis of design recommendations; created ADRs and updated specifications

## Brainstorming

- [UI Architecture Brainstorming (2026-01-07)](./2026-01-07-ui-architecture-brainstorming.md) - Browser-based UI concepts: key qualities, architectural patterns
- [Evolution Brainstorming (2026-01-05)](./2026-01-05-evolution-brainstorming.md) - Future directions: transactions, web UI, upgrade affordances, agent philosophy

## Design Sessions

- [Upgrade Mechanisms (2026-01-01)](./2026-01-01-upgrade-mechanisms.md) - Design of change tracking, migrations, and atomic upgrades

## Implementation

- [Rust Starter Exploration (2026-02-09)](./2026-02-09-rust-starter-exploration.md) - Exploration of rust-starter template patterns
- [Documentation Improvements Summary (2026-01-05)](./2026-01-05-documentation-improvements-summary.md) - Summary of documentation improvements
- [CI/CD Design (2026-01-05)](./2026-01-05-ci-cd-design.md) - CI/CD pipeline design
- [Meta Knowledge Base Compliance Analysis (2026-01-05)](./2026-01-05-meta-knowledge-base-compliance-analysis.md) - Meta KB compliance analysis
- [Information Architecture Analysis (2026-01-04)](./2026-01-04-information-architecture-analysis.md) - Information architecture analysis
- [Meta Knowledge Base Evaluation (2026-01-04)](./2026-01-04-meta-knowledge-base-evaluation.md) - Meta KB evaluation
- [Specification Sync (2026-01-03)](./2026-01-03-specification-sync.md) - Specification sync session
- [Python Implementation Plan (2026-01-03)](./2026-01-03-python-implementation-plan.md) - Implementation roadmap for the Python Graft tool

## Setup & Historical

- [Error Handling Improvements (2025-12-27)](./2025-12-27-error-handling-improvements.md) - Error handling improvements
- [Template as Dependency (2025-12-26)](./2025-12-26-template-as-dependency.md) - Template as dependency exploration
- [Initial Setup (2025-12-26)](./2025-12-26-initial-setup.md) - Initial project setup
- [Initialization (graft) (2025-12-24)](./2025-12-24-initialization.md) - Initial graft project setup
- [Initialization (specs) (2025-12-23)](./2025-12-23-initialization.md) - Initial specification KB creation

---

## Related

- [Docs Index](../docs/README.md) - Implementation documentation
- [Specifications](../docs/specifications/README.md) - Canonical specifications
- [Decisions](../docs/specifications/decisions/) - Specification-level ADRs
- [Implementation Decisions](../docs/decisions/) - Implementation-specific ADRs
