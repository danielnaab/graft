---
deps:
  - docs/project-overview.md
  - docs/how-it-works.md
  - docs/getting-started.md
  - docs/configuration.md
  - docs/command-reference.md
  - docs/claude-skills.md
---

Generate a comprehensive guide to Graft use cases that reveals its unique position as an organizational knowledge management system.

## Core Thesis

Graft is not just a documentation generator—it's a git-native knowledge management system that enables organizations to maintain living, interconnected knowledge where:

- **Documents reference other documents as sources** - Define dependency relationships that match how knowledge flows in your organization
- **Changes propagate through the dependency graph** with surgical edits that teams can review and discuss in PRs
- **First principles cascade through abstraction layers** - but also: meeting notes can inform policies, incidents can inform runbooks, technical realities can inform strategy
- **Multiple stakeholders work at appropriate abstraction levels** while everything stays synchronized
- **Organizational memory lives in git** with full auditability and human oversight

The unique power: Any document can depend on any other document. You design the knowledge graph. When sources change, dependent documents update surgically. Teams review the updates in PRs, discuss implications, and approve or adjust. The entire system is git-backed, human-auditable, and enables team consensus on how knowledge evolves.

## Document Purpose

Create a document that positions Graft as an organizational knowledge management tool that enables:
- Living policies, principles, and agreements that reference their sources
- Strategic pivots that propagate through tactical documents with team oversight
- Cross-functional alignment where engineering, product, and leadership stay synchronized
- PR-based consensus building on how changes cascade through the dependency graph
- Knowledge flowing in whatever direction makes sense: meeting notes inform policies, incidents inform runbooks, technical constraints inform strategy, principles guide implementation
- Git-backed audit trails showing how organizational knowledge evolved

## Structure

### Introduction (3-4 paragraphs)

Position Graft's unique capabilities:
- Git-native change detection with surgical edits (not full regeneration)
- Documents can depend on other documents - you design the dependency graph to match your knowledge flows
- PR-based review enabling team discussion on knowledge propagation
- Knowledge can flow in any direction: principles cascade to implementation, but also meeting notes inform policies, incidents inform runbooks, technical realities inform strategy
- Multi-level DAGs that enable different stakeholders to work at appropriate abstraction levels while staying synchronized

Emphasize: This enables organizational workflows that aren't possible with traditional documentation tools.

### Core Use Cases

Provide exactly 5 use cases. For cases 1-3, use standard structure showing complete workflow including PR review. For cases 4-5, show the DESIGN PROCESS from vague organizational need to complete knowledge architecture.

#### The 5 Use Cases

1. **Living Team Agreements Referenced to Meeting Notes** (STANDARD)
   - Meeting note added with action item: "We should clarify our code review expectations"
   - Team working agreement has this meeting in its deps
   - Surgical edit updates the working agreement section on code review
   - Team reviews the PR: discusses the change, suggests refinements, approves
   - Shows: Meeting notes as sources for policy docs, team consensus via PR review, living policies that reference their origins
   - Emphasize: The surgical edit enables discussion—not just "regenerate everything"

2. **Strategic Principles Cascading to Implementation** (STANDARD)
   - 4-level hierarchy: Company values → Engineering principles → Service design patterns → Code standards
   - Company value added: "Sustainability is a core value"
   - Each level gets surgical edits propagating the principle
   - PRs at each level: leadership approves strategy change, eng team discusses how to operationalize, platform team defines patterns, devs review coding standards
   - Shows: First principles propagation, multi-stakeholder review, human-guided cascade through abstraction layers
   - Emphasize: Each team reviews and discusses how principles apply to their layer

3. **Architecture Decisions with Progressive Synthesis** (STANDARD)
   - Multi-level: Individual ADRs → Domain architecture docs → System-wide architecture overview → Executive technical strategy
   - New ADR added for microservice communication pattern
   - Domain architecture (backend services) gets surgical update incorporating this decision
   - System overview updates to reflect new pattern
   - Executive brief updates to mention improved service architecture
   - PRs enable discussion: domain teams discuss implications, architects review system coherence, leadership sees strategic impact
   - Shows: Progressive abstraction serving multiple audiences, reviewable synthesis, cross-team visibility
   - Emphasize: Different stakeholders see appropriate abstractions, but knowledge stays synchronized

4. **Cross-Functional Product to Engineering Alignment** (DESIGN PROCESS)
   - Start from: "Product and engineering are misaligned. Requirements are always out of date. Engineers don't understand user needs. Product doesn't understand technical constraints."
   - Show iteration:
     - First attempt: Direct coupling user research → technical specs (too big a gap)
     - Second attempt: Add intermediate layers but wrong boundaries (product requirements too technical, design docs too abstract)
     - Exploration: What do different stakeholders need? What should depend on what?
     - Discovery: Need knowledge flowing multiple directions—user research informs strategy, but also technical constraints need to inform strategy
   - Final architecture:
     - User research → Product insights
     - Product insights + Technical constraints → Product strategy (strategy doc depends on BOTH)
     - Product strategy → Feature requirements
     - Feature requirements → Technical design
     - Technical design → Implementation tasks
     - Technical design also feeds constraints back: Technical constraints doc depends on Technical design
     - Product strategy depends on both insights AND constraints
   - Show workflow:
     - User research added → insights update → strategy adjusts surgically → requirements cascade
     - PRs at each level: PMs review insights, leadership reviews strategy shifts, eng reviews requirements clarity
     - Technical constraint discovered during design → constraint doc updates → strategy depends on it → strategy adjusts → product reviews and adapts direction
   - Shows: Cross-functional alignment, flexible dependency graphs (not just linear hierarchies), stakeholder-appropriate abstractions
   - Emphasize: This is organizational alignment machinery—not just docs

5. **Living Organizational Policies and Governance** (DESIGN PROCESS)
   - Start from: "We have lots of policies, meeting notes, decisions scattered everywhere. Hard to know what's current. Hard to understand why policies exist. When policies change, nothing updates."
   - Show iteration:
     - First attempt: Single "team handbook" (becomes stale, no clear sources)
     - Second attempt: Each policy as separate doc (no connections, can't trace reasoning)
     - Exploration: What should depend on what? Policies reference decisions, decisions come from meetings, context comes from incidents/retrospectives
     - Discovery: Design the dependency graph to match how knowledge actually flows—meetings inform policies, incidents inform runbooks, runbooks inform policies
   - Final architecture (showing deps explicitly):
     - Meeting notes (with action items) - source documents
     - Retrospectives and incident post-mortems - source documents
     - Working agreements (deps: [relevant meeting notes, retrospectives])
     - Team processes (deps: [working agreements])
     - Runbooks (deps: [incidents, team processes])
     - Policy decisions (deps: [meetings, retrospectives, working agreements])
   - Show workflow:
     - Retrospective added: "We need clearer on-call expectations"
     - Working agreement lists this retrospective in its deps → updates surgically to incorporate this
     - Team reviews PR: discusses expectations, refines language, approves
     - Team process docs list working agreement in their deps → cascade update
     - Runbooks list team processes in their deps → reference new process
     - Full git history shows: why this policy exists, what meeting/incident prompted it, how it evolved
   - Shows: Living organizational memory, traceability, team consensus, flexible dependency graphs that match knowledge flows
   - Emphasize: This is how organizational knowledge stays alive and traceable

### Cross-Cutting Patterns (3 patterns, 1 concise paragraph each)

#### Pattern: First Principles Propagation
How strategic principles cascade through abstraction layers to tactical implementation. Surgical edits at each layer enable team discussion on how principles apply to their domain. Human oversight prevents blind propagation—teams review, discuss, and guide how high-level values become concrete practices.

#### Pattern: Flexible Knowledge Dependencies
Documents can depend on any other documents. You design the dependency graph to match how knowledge flows in your organization. Not just top-down (strategy → implementation) but any direction that makes sense: meeting notes inform policies, incidents inform runbooks, code realities inform architecture, technical constraints inform strategy, principles guide implementation. This creates organizational alignment where all parts stay synchronized.

#### Pattern: PR-Based Consensus on Knowledge Evolution
Surgical edits enable team review and discussion in PRs. Not "the LLM regenerated everything"—specific, reviewable changes that teams can discuss, refine, approve or reject. Git-backed audit trail shows how organizational knowledge evolved and why. This enables team consensus on how knowledge propagates through hierarchies.

### Getting Started with Organizational Knowledge Management (4-5 paragraphs)

Provide practical guidance for organizations adopting Graft as a knowledge management system:

1. **Start with one bidirectional coupling** - Pick something small: meeting notes → working agreement. Prove the workflow with your team.

2. **Identify your knowledge hierarchies** - Where do you have strategic → tactical gaps? Where do first principles need to cascade? Where are docs always out of sync?

3. **Design for your stakeholders** - What abstraction levels do different roles need? Engineers need implementation details, leadership needs strategic overview—but they should stay synchronized.

4. **Enable PR-based review** - The surgical edits are the feature. Teams should review and discuss how changes propagate. This builds consensus.

5. **Think beyond documentation** - Policies, agreements, principles, processes—anything that should reference sources and update when sources change.

Include a simple framework for identifying where Graft adds value:
- Where do you have multi-level knowledge hierarchies?
- Where do changes in one place require updates elsewhere?
- Where do teams need to review and discuss how knowledge propagates?
- Where do you need audit trails of how knowledge evolved?

## Writing Guidelines

**Tone & Style:**
- Positioning tone - this enables organizational workflows other tools cannot
- Concrete and specific - every use case shows real workflows with PR review
- Emphasize the unique capabilities: surgical edits, PR review, cascading changes, bidirectional coupling
- Professional but visionary - this is a new way of managing organizational knowledge
- Focus on workflows and organizational benefits, not technical implementation details

**Key Emphasis Throughout:**
- Surgical edits enable PR-based discussion (not "regenerate everything")
- Documents can depend on other documents - you design the dependency graph
- Knowledge flows in whatever direction makes sense (not just top-down hierarchies)
- First principles can cascade through layers, but also: meeting notes inform policies, incidents inform runbooks, technical realities inform strategy
- Multi-stakeholder alignment at appropriate abstraction levels
- Git-backed auditability and team consensus
- Living organizational memory, not static documentation

**Examples Quality:**
- Show complete workflows including PR review and team discussion
- Include git commands, frontmatter deps, and PR review scenarios
- Show how changes propagate through the dependency graph
- Demonstrate flexible knowledge flows (not just hierarchies)
- Make deps explicit in examples - show what depends on what
- For design process examples: show realistic organizational pain points and iteration to find the right architecture

**Structure:**
- Use clear headings and subheadings
- Include code blocks showing frontmatter, commands, and workflows
- Use emphasis (bold) for key unique capabilities
- Keep sections focused but comprehensive
- Make the use cases feel real—like actual organizational challenges

## Output Format

Generate the complete use cases document as clean markdown suitable for docs/use-cases.md. The document should:
- Introduction positioning Graft's unique organizational capabilities (3-4 paragraphs)
- 5 use cases total:
  - Cases 1-3: Standard structure with complete workflows including PR review
  - Cases 4-5: Design process showing iteration from organizational pain to complete knowledge architecture
- 3 cross-cutting patterns (1 paragraph each) emphasizing unique capabilities
- Getting started section with practical organizational adoption guidance (4-5 paragraphs)
- Total length: 600-800 lines to accommodate detailed workflows and design process

**IMPORTANT**: Output ONLY the markdown content. Do NOT wrap in code fences. Start with level-1 heading.

**CRITICAL**: COMPLETE THE ENTIRE DOCUMENT. All 5 use cases (including both full design process examples with iteration), all 3 patterns, and the complete getting started section MUST be included. Do not stop mid-section. Every use case should show PR-based review workflows. Design process cases must show realistic iteration and organizational thinking.
