# Specifications

This directory contains all specifications for the graft ecosystem, organized by tool.

## About Specification Formats

Different tools use different specification formats based on their maturity and purpose:

### Graft Specifications (Formal)

Located in [`graft/`](./graft/), these are detailed, implementer-focused specifications for the graft dependency management system. They follow a formal structure with:

- Complete schemas and field definitions
- Validation pseudocode
- Extensive examples
- Precise implementation guidance

**Format**: Markdown with YAML frontmatter
**Status**: Draft (v3 - flat-only dependency model)
**Best for**: Stable APIs, data formats, protocols that need precise implementation

### Grove Specifications (Living)

Located in [`grove/`](./grove/), these are behavior-focused, evolving specifications for the Grove workspace management tool. They follow the [living-specifications](../../.graft/living-specifications/) methodology with:

- Intent and non-goals sections
- Gherkin-style behavior scenarios
- Open questions and decisions log
- Lightweight, easy to update

**Format**: Living specifications (markdown with status frontmatter)
**Status**: Draft/Working (active development)
**Best for**: User-facing behaviors, workflows, UX that evolves with implementation

## Directory Structure

```
specifications/
├── README.md           # This file
├── graft/              # Graft formal specifications
│   ├── README.md       # Graft specs index
│   └── *.md            # Individual specifications
└── grove/              # Grove living specifications
    ├── README.md       # Grove specs index and reading guide
    └── *.md            # Individual specifications
```

## Why Two Formats?

The different formats reflect different stages of maturity and different needs:

- **Graft** has stable data formats and operations that need precise specification for implementation
- **Grove** is actively being designed through vertical slices and benefits from lightweight, scenario-based specs that can evolve quickly

Both formats are version-controlled in git, reviewable via PRs, and structured for both human reading and AI consumption. The format choice is intentional, not arbitrary.

## Related Documentation

- [Graft Architecture](architecture.md) - High-level graft system design
- [Grove Design Notes](../../notes/) - Exploration notes that inform Grove specs
- [Living Specifications Methodology](../../.graft/living-specifications/) - Details on the living-spec format
