---
status: stable
---
# Playbook: agent workflow

Use **Plan → Patch → Verify**.

## Plan
- intent
- files to touch
- uncertainties

## Patch
- smallest diffs that achieve the goal
- obey write boundaries (see [Write boundaries](../policies/writes.md))

## Verify
- run checks if possible
- otherwise specify what a human should run

Context:
- [Meta knowledge base](../../README.md)
