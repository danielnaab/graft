---
model: bedrock-claude-v4.5-sonnet-us
deps:
  - .claude/skills/steward/SKILL.md
  - scripts/entrypoint.sh
  - docs/project-overview.md
---

# Claude Skills in Graft

Create engaging documentation that shows users how to work with Claude using the steward skill for documentation design and management.

## Required Sections

1. **What Are Claude Skills?**
   - Brief explanation of Claude Skills as model-invoked capabilities
   - Contrast with slash commands (context-aware vs explicit)
   - Link to Claude docs

2. **The Steward Skill**
   - Explain what the steward skill provides
   - Describe when it activates (describing documentation goals)
   - Set expectations for how it helps

3. **Working with Design Problems**
   - This is the heart of the document - show how to move from vague goals to concrete solutions
   - Create 4-5 realistic, detailed examples showing:
     - **From Vague Goal to Information Architecture**: User has unclear requirements, works through them conversationally, arrives at a structured solution
     - **Refining Through Iteration**: User has existing docs, wants to adapt for different audience, iterates on prompt instructions
     - **Understanding Impact**: User wants to make changes, asks what will regenerate, makes informed decision
     - **Discovering Structure Through Exploration**: User has raw materials but no organization, uses prompts to analyze and discover structure
   - Each example should show:
     - Initial user request (realistic, slightly unclear)
     - Design conversation (questions, considerations)
     - Exploration of options
     - Concrete solution
     - Iterative refinement
     - Final outcome
   - Use realistic dialogue and scenarios from actual documentation projects

4. **Working with the Steward**
   - Explain what the steward can help with:
     - Information architecture suggestions
     - Impact analysis before changes
     - Creating structured prompts
     - Validation and checking
     - Understanding regeneration behavior
   - Emphasize conversational, exploratory approach

5. **Getting Started**
   - How to begin (open Claude Code with Graft repo)
   - What to say and how to describe goals
   - Embrace ambiguity and exploration

6. **Learn More**
   - Link to related documentation

## Style Guidelines

- Write in a warm, encouraging tone
- Show, don't just tell - use extensive examples
- Examples should feel like real work scenarios
- Use dialogue and conversation in examples
- Emphasize the exploratory, iterative nature
- Avoid prescriptive rules - focus on principles and patterns
- Make it feel approachable and collaborative
- Show both successful outcomes AND iterative refinement
- Demonstrate thinking process, not just final solutions

## Example Quality

Each example should:
- Start with realistic ambiguity
- Show the human thinking process
- Include actual commands and file structures
- Demonstrate incremental progress
- Show course corrections and refinements
- Feel like something that actually happened

## Technical Accuracy

- Reference actual steward skill capabilities
- Show real bin/graft commands
- Use accurate file paths and directory structures
- Demonstrate actual system behavior
- Cross-reference project-overview.md for consistency

Generate documentation that makes users excited to explore documentation design with Claude, showing them what's possible beyond simple command execution.
