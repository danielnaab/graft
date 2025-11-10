---
description: Start work on a new vertical slice
argument-hint: <slice-number>
allowed-tools: Read(docs/**), Read(tests/**), Read(src/**), TodoWrite, Edit(agent-records/work-log/**), Write(agent-records/work-log/**)
---
Begin implementing Slice $ARGUMENTS following the Graft development process:

1. Read the requirements:
   - Check `docs/roadmap/vertical-slices.md` for Slice $ARGUMENTS acceptance criteria
   - Review `docs/implementation-strategy.md` for the slice overview
   - Understand the CLI contract from `docs/cli-spec.md`

2. Create a comprehensive todo list using TodoWrite with items for:
   - Writing failing tests in `tests/test_*.py` (subprocess/black-box style)
   - Implementing domain entities in `src/graft/domain/` if needed
   - Creating adapters in `src/graft/adapters/` if needed
   - Building services in `src/graft/services/`
   - Adding CLI command in `src/graft/cli.py`
   - Updating schemas in `schemas/`
   - Updating documentation

3. Prepare work log:
   - Create or update `agent-records/work-log/$(date +%Y-%m-%d).md`
   - Add a new session section for this slice

4. Review architectural patterns:
   - Read `docs/adr/0002-layered-architecture-with-separation-of-concerns.md`
   - Look at existing implementations in `src/graft/services/explain.py` for patterns

Remember:
- Deliver narrow slices, no scope expansion beyond Slice $ARGUMENTS
- Follow outside-in TDD: tests first, then minimal implementation
- Use layered architecture: Domain → Adapters → Services → CLI
