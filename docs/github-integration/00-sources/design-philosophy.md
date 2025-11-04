# GitHub Integration Design Philosophy

## The Essential Nature of Graft

Graft embodies several fundamental principles that define its character:

### 1. Git-Native by Design
Everything lives in version control. Prompts, sources, and generated outputs are all tracked in git. This creates a complete audit trail where you can see not just what changed, but why it changed (through prompt modifications) and what sources drove the change.

### 2. Intelligent Change Detection
Graft distinguishes between two fundamentally different types of changes:
- **Source changes**: New information arrives, but the structure and style requirements remain constant
- **Instruction changes**: The requirements, tone, or structure evolve, necessitating a full rethink

This distinction enables surgical updates: when sources change, only the semantically affected sections update while everything else remains byte-identical. This minimizes diff noise and preserves human review quality.

### 3. DAG-Based Documentation Hierarchies
Generated documents can serve as inputs to other prompts, creating cascading documentation graphs. When a source changes, the impact propagates through the dependency graph automatically. This enables:
- Modular documentation components
- Consistent information flow
- Automatic synchronization across layers
- Clear separation of concerns

### 4. Reproducibility as a Core Guarantee
Same inputs produce identical outputs. This property is critical for:
- Code review: Reviewers can trust that regenerating docs produces what's shown
- CI/CD: Automated checks can verify docs match their sources
- Collaboration: Multiple developers get consistent results
- Debugging: Issues can be reproduced reliably

### 5. Minimal Regeneration Philosophy
Only regenerate what changed. Unchanged documents remain byte-identical, which:
- Reduces API costs
- Speeds up build times
- Keeps git diffs focused
- Makes reviews manageable

## GitHub Pull Requests as a Collaboration Primitive

Pull requests represent:

### 1. Proposed Changes Under Review
PRs are *not* final - they're conversations about changes. The PR is where:
- Authors explain their reasoning
- Reviewers examine impact
- Iterations happen before merge
- Quality gates enforce standards

### 2. The Unit of Change
A PR bundles related changes together. For documentation:
- Source material additions
- Prompt modifications
- Generated output updates
- All change together, atomically

### 3. A Trust Boundary
PRs separate:
- Unreviewed → Reviewed
- Proposed → Accepted
- Individual → Team
- Experiment → Canon

### 4. A Context for Discussion
PRs provide structure for:
- Questions about changes
- Suggestions for improvement
- Explanation of approach
- Knowledge transfer

## The Powerful Intersection

When Graft meets GitHub PRs, something powerful emerges:

### 1. **Documentation Changes Become Reviewable**

Traditional documentation has a problem: changes are opaque. You see the new text, but not:
- What sources drove the change
- What instructions shaped the synthesis
- What alternatives were considered
- Why this particular framing

Graft makes this transparent:
```
PR contains:
├── docs/00-sources/new-feature-spec.md      # The new information
├── docs/01-explorations/feature.prompt.md   # How to synthesize it
└── docs/01-explorations/feature.md          # The resulting synthesis
```

Reviewers see the complete picture: input, instructions, and output.

### 2. **Generated Docs Serve as Validation**

Including generated docs in the PR creates a feedback loop:
- Author edits sources and prompts
- Graft regenerates documentation
- Author reviews generated output
- If output is wrong, author refines prompts/sources
- Iteration continues until output is right
- Then PR is opened for team review

This means **PRs contain validated output**. The generated docs aren't theoretical - they're what will actually be produced.

### 3. **CI/CD Becomes a Trust Mechanism**

GitHub Actions can verify that:
- All dependencies are present
- Generated docs match their sources and prompts
- The DAG has no cycles
- No stale documentation exists

This creates trust: reviewers know that what they see is what will be merged, and that the documentation will remain synchronized.

### 4. **Claude Code Enables Assisted Authoring**

Claude Code can:
- Help edit source documents based on requirements
- Refine prompt instructions for better synthesis
- Analyze generated output for quality
- Suggest improvements to prompts based on output gaps
- Iterate rapidly on prompt design

This transforms documentation authoring from "writing" to "prompt engineering + review".

### 5. **The Commit History Tells the Full Story**

Each commit shows:
- What human-written content changed (sources/prompts)
- What AI-generated content changed (docs)
- The relationship between the two

This creates an auditable trail where you can understand:
- Why documentation evolved
- What requirements drove changes
- How synthesis instructions matured
- When regenerations occurred

## Performance-Enhancing Workflows

The integration enables several powerful workflows:

### Workflow 1: Preview-Driven Development
```
1. Developer adds new feature documentation source
2. Creates/updates prompt to synthesize it
3. Runs Graft locally to preview output
4. Iterates on prompt until output is right
5. Opens PR with sources + prompts + generated docs
6. Reviewers see the full context
7. Discussion happens in PR
8. Merge when approved
```

**Performance enhancement**: No surprises at merge time. Generated docs are validated before review.

### Workflow 2: Validation-First Merging
```
1. PR opened with doc changes
2. GitHub Actions runs Graft
3. Verifies generated docs match sources/prompts
4. Fails if docs are stale or inconsistent
5. Author fixes locally and pushes
6. Merge only when validation passes
```

**Performance enhancement**: Enforces synchronization automatically. No human mental overhead to remember to regenerate.

### Workflow 3: Claude Code Assisted Iteration
```
1. Developer asks Claude Code to improve docs
2. Claude Code reads current sources and prompts
3. Suggests edits to prompts or sources
4. Developer reviews and accepts/refines
5. Runs Graft to see result
6. Claude Code reviews output, suggests further refinements
7. Iterate until satisfied
8. Commit and push
```

**Performance enhancement**: Leverages AI for both prompt engineering and output review. Faster iteration cycles.

### Workflow 4: Dependency-Aware Reviews
```
1. PR modifies a source file
2. GitHub Actions identifies which docs depend on it
3. Regenerates only affected docs
4. Shows reviewers exactly what will change downstream
5. Reviewers understand full impact
6. Can request changes to prompts if synthesis is wrong
```

**Performance enhancement**: Impact analysis is automatic. Reviewers have complete context.

### Workflow 5: Slash Command Shortcuts
```
1. Developer types `/graft-preview` in PR
2. GitHub Actions regenerates all docs
3. Commits updated docs to PR branch
4. Developer sees immediate feedback
5. Can request `/graft-validate` to check without committing
```

**Performance enhancement**: No context switching to local environment. Fast iteration in PR context.

## Output Quality Enhancement

The integration improves output quality through:

### 1. **Iterative Refinement Loops**
PRs enable rapid iteration:
- Generate → Review → Refine → Regenerate
- All in version control
- All with team visibility
- All with validation

### 2. **Prompt Evolution**
Prompts improve over time:
- Team learns what works
- Prompts get more precise
- Common patterns emerge
- Best practices crystallize

The PR review process surfaces prompt quality issues early.

### 3. **Source Material Completeness**
When generated docs have gaps, it's obvious:
- Reviewers see missing information
- Can trace back to source gaps
- Author adds missing sources
- Regeneration fills the gaps

### 4. **Consistency Through Automation**
Graft enforces consistency:
- Same prompt + same sources = same output
- No manual copy-paste errors
- No forgetting to update dependent docs
- No style drift

### 5. **Auditability Drives Quality**
When everything is tracked:
- Bad outputs can be traced to bad prompts
- Good patterns can be identified and replicated
- Quality trends are visible over time
- Accountability is clear

## Design Constraints

Any implementation must respect:

### 1. **Docker-Based Execution**
Graft runs in Docker containers. GitHub Actions must:
- Build or pull the Graft image
- Mount workspace appropriately
- Handle AWS credentials securely
- Run in CI environment

### 2. **AWS Bedrock Authentication**
LLM calls require AWS credentials. Options:
- GitHub Secrets for AWS keys
- OIDC for temporary credentials
- Environment-specific configurations

### 3. **Build Time Considerations**
Documentation regeneration takes time:
- Each LLM call is ~5-30 seconds
- Multiple documents multiply this
- Large DAGs can be slow
- Need smart caching/parallelism

### 4. **Git Hygiene**
Generated docs are checked into git:
- Creates large diffs
- Requires thoughtful commit messages
- May need separate commits for clarity
- Needs clear attribution (human vs AI changes)

### 5. **Review Ergonomics**
PRs with generated docs:
- Can be large
- Mix human and AI content
- Need clear visual distinction
- Should highlight what matters

## Key Design Questions

The implementation must answer:

1. **When should docs regenerate?**
   - On every push?
   - On demand via comment/label?
   - Only when sources/prompts change?

2. **Should regenerated docs be committed automatically?**
   - Auto-commit keeps PRs current
   - But may surprise developers
   - Or left as validation check only?

3. **How should AWS credentials be managed?**
   - GitHub Secrets?
   - OIDC federation?
   - Per-developer configuration?

4. **What validation should be required?**
   - Docs match sources/prompts (always)
   - No stale docs (always)
   - No missing dependencies (always)
   - Output quality checks (optional?)

5. **How should Claude Code integration work?**
   - Slash commands for common operations?
   - Hooks for automatic suggestions?
   - Skills for complex workflows?

## Success Criteria

The integration succeeds if:

1. **Documentation stays synchronized automatically**
   - No stale docs can be merged
   - Validation catches inconsistencies
   - CI enforces standards

2. **Authoring becomes faster and easier**
   - Claude Code assists with prompt refinement
   - Iteration cycles are quick
   - Feedback is immediate

3. **Reviews are higher quality**
   - Reviewers have full context
   - Impact is clear
   - Changes are justified

4. **The system is easy to use**
   - Setup is straightforward
   - Errors are actionable
   - Documentation is clear

5. **It demonstrates Graft's value**
   - Dogfooding builds confidence
   - Real-world usage validates design
   - Issues surface and get fixed
   - The docs themselves prove the approach

## Conclusion

The integration of Graft with GitHub PRs and Claude Code creates a powerful documentation pipeline where:

- **Changes are transparent** (sources + prompts + outputs all visible)
- **Quality is enforced** (validation prevents staleness)
- **Iteration is fast** (assisted authoring + quick regeneration)
- **Reviews are informed** (full context available)
- **Trust is automated** (CI verifies consistency)

This isn't just about automation - it's about creating a workflow where documentation naturally stays synchronized, where quality emerges from process, and where AI assistance amplifies human capability rather than replacing it.
