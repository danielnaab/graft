---
status: working
---
# Multi-agent coordination

Patterns for multiple specialized agents collaborating on same knowledge base.

## When needed

Consider multi-agent patterns when you have:
- Separate specification and implementation workflows
- Specialized agents for different tasks
- Clear role boundaries in knowledge work

Most projects don't need this. Start with single-agent workflows.

## Define agent roles

Declare in knowledge-base.yaml:

```yaml
agents:
  spec-writer:
    entrypoint: docs/agents-spec.md
    role: "Define what to build"
    write_boundaries:
      allow: ["docs/specification/**"]
      deny: ["src/**"]

  developer:
    entrypoint: docs/agents-impl.md
    role: "Implement specifications"
    write_boundaries:
      allow: ["src/**", "tests/**", "notes/**"]
      deny: ["docs/specification/**"]
```

## Coordination patterns

**Sequential handoff** - clear pipeline, each stage depends on previous:
- Spec agent writes spec, tags commit with `@impl-agent`
- Impl agent implements, tags commit with `@docs-agent`
- Docs agent synchronizes documentation
- Advantages: clear handoffs, no conflicts
- Disadvantages: sequential (slower), potential bottlenecks

**Concurrent with boundaries** - work in parallel without conflicts:
- Non-overlapping write zones per agent
- Each agent owns specific directories
- Git conflicts indicate boundary violations
- Advantages: parallel work, clear ownership
- Disadvantages: requires careful boundary design

**Collaborative with locks** - agents work on same documents:
- Use frontmatter locks: `locked_by: agent-name, locked_until: timestamp`
- Agent checks lock before editing
- Advantages: explicit coordination
- Disadvantages: complexity, potential stale locks

## Communication

Agent-to-agent in commit messages:
```
@agent-name: Message

Details and context
```

Status signaling in frontmatter:
```yaml
status: working
assigned_to: impl-agent
blocked_by: ["waiting for spec completion"]
```

## Best practices

- Start simple: one agent first
- Clear role definitions
- Minimize coordination overhead
- Prefer boundaries over locks
- Human mediates conflicts

## Anti-patterns

Avoid:
- Too many roles (start with 1-2)
- Unclear boundaries (overlapping responsibilities)
- No handoff protocol
- Real-time coordination (async is simpler)
- Complex locking

Related:
- [Agent workflow](agent-workflow.md)
- [Write boundaries policy](../policies/writes.md)
