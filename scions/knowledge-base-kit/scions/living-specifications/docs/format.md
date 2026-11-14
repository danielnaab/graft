---
status: working
---

# Specification Format

The minimum viable structure for a living specification.

## Template

```markdown
---
status: draft | working | stable | deprecated
last-verified: YYYY-MM-DD
owners: [team-members]
---

# [Feature/Component Name]

## Intent

Why does this exist? What problem does it solve?

## Non-goals

What is explicitly out of scope?

## Behavior

What does the system do?

### [Scenario Name]

\`\`\`gherkin
Given [precondition]
When [action]
Then [expected outcome]
\`\`\`

### Edge Cases

- [Case]: [Expected behavior]

## Constraints

- Performance: [requirements]
- Security: [requirements]
- Compatibility: [requirements]

## Open Questions

- [ ] [Unresolved question needing team input]

## Decisions

- [Date]: [Decision and brief rationale]

## Sources

- [Reference](URL) - Context
- Personal experience: [Project] - Learning
```

## Section Purposes

| Section | Audience | Purpose |
|---------|----------|---------|
| Intent | Everyone | Why this matters; context for newcomers |
| Non-goals | Reviewers, implementers | Prevent scope creep; clarify boundaries |
| Behavior | Developers, AI, testers | Precise enough to implement and verify |
| Constraints | Developers, ops | Non-functional requirements that often get lost |
| Open Questions | Team | Explicit "not decided yet"—invites collaboration |
| Decisions | Future maintainers | Inline rationale prevents "why is this like this?" archaeology |
| Sources | Everyone | Provenance for trust |

## Design Choices

- **Status is non-negotiable** — without it, nobody knows if the spec is aspirational or authoritative
- **Examples over abstractions** — concrete Given/When/Then scenarios beat general rules
- **No implementation details** — that's what code is for
- **No version history** — git handles this
- **No approval signatures** — PR reviews handle this
- **Structured enough for tools** — behavior statements could be mechanically verified

## Increment Tagging

Behavior statements can indicate which increment introduced them:

```markdown
## Behavior

### Authentication [v0.1]

\`\`\`gherkin
Given valid credentials
When user submits login
Then user receives a session token
\`\`\`

### Password Reset [v0.2]

\`\`\`gherkin
Given expired password
When user attempts login
Then user is prompted to reset
\`\`\`

### Rate Limiting [v0.3]

- 5 failed attempts per 10 minutes triggers lockout
```

This allows tickets to reference specific increments: "Implement behaviors marked `[v0.2]`."

## Related

- [Principles](principles.md)
- [Writing specs playbook](guides/writing-specs.md)
