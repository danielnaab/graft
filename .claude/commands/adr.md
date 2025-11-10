---
description: Create a new Architecture Decision Record
argument-hint: <title>
allowed-tools: Read(docs/adr/**), Write(docs/adr/**), Bash(ls:docs/adr), Edit(agent-records/work-log/**)
---
Create a new Architecture Decision Record for: $ARGUMENTS

1. Find the next ADR number:
   - List existing ADRs in `docs/adr/`
   - Determine the next sequential number (e.g., if 0002 exists, create 0003)

2. Create slug from title:
   - Convert "$ARGUMENTS" to lowercase-with-hyphens
   - Example: "Use Protocol Types" → "use-protocol-types"

3. Create `docs/adr/NNNN-<slug>.md` following the template:

```markdown
# ADR NNNN: [Title]

## Status
Proposed / Accepted / Deprecated / Superseded

## Context
What is the issue that we're seeing that is motivating this decision or change?

## Decision
What is the change that we're proposing and/or doing?

## Consequences
What becomes easier or more difficult to do because of this change?

### Positive
- [Benefits of this decision]

### Negative
- [Drawbacks or trade-offs]

## Implementation Notes
How should this decision be applied in practice?
```

4. Fill in the template based on the architectural decision being made

5. Update the work log with the new ADR

Remember: ADRs document significant architectural decisions and their rationale for future reference.
