# Claude Skills in Graft

## What Are Claude Skills?

[Claude Skills](https://docs.claude.com/en/docs/claude-code/skills) are model-invoked capabilities that Claude can use automatically when relevant to your conversation. Unlike slash commands you explicitly call, Skills activate based on context.

## The Steward Skill

Graft includes a steward skill that gives Claude knowledge of Graft's structure, commands, and dependency system. When you describe documentation goals, Claude can use this skill to help design and implement solutions.

## Working with Design Problems

The examples below show how to start with qualitative goals and work toward concrete solutions. The process often involves exploring the problem space, creating structure, and refining based on what you learn.

### Example: From Vague Goal to Information Architecture

**Your initial request:**
```
"We're building a new feature that lets users export their data. I need to
document this, but I'm not sure how to organize it. There's the user-facing
part, the API changes, and some security considerations."
```

**Design conversation:**

You start by describing what you know. Claude asks clarifying questions:
- What formats do you support? (CSV, JSON, PDF)
- Who's the audience for each piece? (End users, API consumers, security reviewers)
- Does this relate to existing docs? (Privacy policy, API reference)

Through this conversation, you realize you need several connected pieces:

1. **User guide** - How end users export their data
2. **API documentation** - Technical details for developers
3. **Security analysis** - Privacy implications and safeguards
4. **Architecture overview** - How the pieces fit together

**Exploring the structure:**

Claude can help you think through the information architecture:

```
docs/00-sources/
  export-feature/
    user-requirements.md     # What users need to know
    api-spec.md             # Technical API details
    security-notes.md       # Privacy & security concerns

docs/01-explorations/
  export-user-guide.prompt.md       # Depends on: user-requirements.md
  export-api-reference.prompt.md    # Depends on: api-spec.md
  export-security.prompt.md         # Depends on: security-notes.md

docs/02-frameworks/
  export-overview.prompt.md         # Synthesizes all three explorations
```

This structure lets you:
- Work on each piece independently
- Have the overview regenerate when any piece changes
- Maintain clear dependencies

**Building iteratively:**

You don't need to create everything at once. Start with the user guide:

```
"Let's start with the user guide. I'll paste in our requirements doc."
```

Claude creates the source file and prompt, generates the guide. You review it, notice it's missing information about rate limits. You update the source, rebuild, see the changes.

Later, you add the API reference. When you're ready, you create the overview that synthesizes both. Each step builds on what came before.

### Example: Refining Through Iteration

**Your situation:**
```
"I have technical documentation for our authentication system, but it's too
detailed for our sales team to use. They need something that explains what
we do and why it's secure, without all the implementation details."
```

**The design process:**

Instead of duplicating content, you can create a new view:

```
docs/01-explorations/
  auth-technical.prompt.md          # Existing detailed docs

docs/02-frameworks/
  auth-executive-summary.prompt.md  # New: depends on auth-technical.prompt.md
                                    # Instructions: "Create a 2-page executive
                                    # summary focusing on security benefits and
                                    # competitive advantages. Avoid implementation
                                    # details. Target audience: non-technical
                                    # stakeholders."
```

The executive summary automatically incorporates the source material but transforms it based on the prompt instructions. When the technical docs change, the summary regenerates to stay in sync.

You review the first version: "This is better, but it still has too much jargon."

You update the prompt instructions: "Use plain language. Define any technical terms that can't be avoided. Use examples instead of abstract concepts."

Rebuild. Better, but the structure feels off. You add: "Start with the business value, then explain how it works at a high level, then address common concerns."

After a few iterations, you have a document that serves its audience while staying connected to the authoritative technical source.

### Example: Understanding Impact

**Your question:**
```
"I want to update our architecture diagrams in the infrastructure source file.
What will this affect?"
```

Claude uses `bin/graft uses` to show you:

```
docs/00-sources/infrastructure.md is used by:
  - docs/01-explorations/deployment-guide.prompt.md
  - docs/02-frameworks/system-architecture.prompt.md
  - docs/03-integration/technical-overview.prompt.md
```

You see the cascade: changing the diagrams will regenerate three documents. The deployment guide will get new diagrams. The system architecture will incorporate them into a bigger picture. The technical overview will get refreshed to stay consistent.

This helps you decide: "Let me update the diagrams and check the deployment guide carefully. The others can regenerate automatically."

### Example: Discovering Structure Through Exploration

**Your situation:**
```
"We're documenting a complex workflow that involves multiple teams. I have
interviews with each team about their part, but I don't know how to organize
this into documentation."
```

**The exploration process:**

Start by capturing what you have:

```
docs/00-sources/workflow-project/
  team-a-interview.md
  team-b-interview.md
  team-c-interview.md
  current-process-diagram.png
```

Create an exploration prompt that asks Claude to analyze the material and identify:
- Common themes across teams
- Handoff points and dependencies
- Pain points mentioned by multiple teams
- The end-to-end flow

Review the exploration output. You notice three distinct phases emerge from the interviews. This suggests a structure:

```
docs/01-explorations/
  workflow-phase1.prompt.md    # Depends on: team-a, team-b interviews
  workflow-phase2.prompt.md    # Depends on: team-b, team-c interviews
  workflow-phase3.prompt.md    # Depends on: team-c interview

docs/02-frameworks/
  workflow-complete.prompt.md  # Synthesizes all three phases
```

The structure emerged from the content, not from a predetermined template. Each phase document focuses on the relevant interviews. The synthesis creates the complete picture.

## Working with the Steward

When you describe documentation goals, Claude may use the steward skill to:

- Suggest information architecture based on your content and goals
- Check which prompts will regenerate before making changes
- Create source files and prompts with appropriate dependencies
- Validate structure before running builds
- Explain why something regenerated when you didn't expect it

You can ask questions about structure, dependencies, and impact. The steward helps you understand the system as you work with it.

## Getting Started

In Claude Code with your Graft repository open, describe what you're trying to document. Be specific about your goals and constraints. If you're not sure how to organize something, say so—figuring out the structure is part of the design process.

## Learn More

- [project-overview.md](project-overview.md) - Core concepts and quick start
- [how-it-works.md](how-it-works.md) - Change detection and actions
- [command-reference.md](command-reference.md) - `bin/graft` commands
- `.claude/skills/steward/SKILL.md` - Steward skill source
