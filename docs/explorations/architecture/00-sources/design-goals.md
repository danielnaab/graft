# Design Goals for Graft

## Core Philosophy

Graft should embody the principles of:

1. **Composability**: Small, focused tools that work together
2. **Transparency**: Everything should be inspectable and understandable
3. **Reproducibility**: Same inputs should produce same outputs
4. **Git-native**: Embrace git as the source of truth
5. **Flexibility**: Support both LLM and non-LLM transformations
6. **Incrementality**: Only regenerate what changed

## Metaphor Consistency

"Graft" works as both noun and verb:
- **Noun**: A graft on a git tree - a living document that grows from sources
- **Verb**: Graft sources onto the tree - the act of integrating content

Operations should align with horticultural metaphors:
- **grow**: Update/regenerate a graft
- **prune**: Remove outdated grafts
- **feed**: Update source dependencies

## User Experience Goals

### For Document Authors
- Simple, declarative specification of what to generate
- Clear dependency tracking
- Ability to lock expensive explorations
- Support for multi-file outputs

### For System Integrators
- Pipeline output through external processes
- Support non-LLM transformations (scripts, formatters)
- Flexible enough for diverse use cases
- Minimal magic, maximum clarity

## Non-Goals

- Being a general-purpose build system (use DVC/Make for that)
- Supporting every possible transformation (focus on synthesis)
- Hiding git (embrace it as the version control layer)
