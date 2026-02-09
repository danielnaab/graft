---
title: "Status Check Syntax Exploration"
date: 2026-02-08
status: working
participants: ["human", "agent"]
tags: [exploration, grove, graft, status, syntax, design]
related: ["2026-02-07-grove-workflow-hub-primitives.md"]
---

# Status Check Syntax Exploration

## Context

Following the [Grove Workflow Hub Primitives](./2026-02-07-grove-workflow-hub-primitives.md) exploration, which proposed that repos could define status checks in graft.yaml, this session examined the syntax and semantics for how status checks should be expressed.

The triggering question: "The variable assignment feels odd. How does the variable surface? What alternative syntax is available to meet the ultimate goal?"

This led to a deeper examination of what status checks are actually trying to accomplish and what the simplest effective syntax would be.

**Graduated to specifications:**
- [Workspace Configuration](../docs/specifications/grove/workspace-config.md) - Status script contract, exit code semantics

---

## The Original Proposal (Inline Shell Scripts)

The initial design included inline shell scripts with variables:

```yaml
status:
  - name: overdue
    run: |
      days=$(( ($(date +%s) - $(git log -1 --format=%ct)) / 86400 ))
      [ $days -gt 30 ] && echo "⚠️ Monthly close overdue ($days days)"
    description: "Check if monthly close is overdue"

  - name: inbox-overflow
    run: |
      count=$(ls inbox/ | wc -l)
      [ $count -gt 10 ] && echo "📥 $count captures to organize"
```

**The problem identified:**
- Variables like `$days` and `$count` are computed inside the shell script but aren't surfaced in the YAML structure
- The variables are internal to the bash implementation
- Requires bash knowledge to understand what's happening
- Makes the config less declarative and harder to read at a glance
- Inline bash in YAML is verbose and error-prone

---

## The Fundamental Goal

**What are we actually trying to accomplish?**

Enable repos to declare "here's how to tell if I need attention" in a way that:
1. Tools can query programmatically
2. Humans can understand by reading the config
3. Works for diverse repos with different needs
4. Doesn't require building a complex DSL

**The desired output:**
```
● personal-finances
  ⚠️ Monthly close overdue (32 days)
  📋 3 uncategorized transactions

● general-notes
  📥 12 captures to organize
```

Each signal has:
- An indicator (emoji/symbol)
- A message (what needs attention)
- Optionally: dynamic data (32 days, 3 transactions)

**The irreducible minimum:**
- A way to execute logic (check a condition)
- A way to produce a message if the condition is true
- Support for dynamic data in messages

---

## Alternative Syntax Options

### Option 1: Separate Script Files

**Move complexity out of config:**

```yaml
status:
  overdue: "./status/overdue.sh"
  inbox: "./status/inbox.sh"
  uncategorized: "./status/uncategorized.sh"
```

Scripts live in the repo:
```bash
# status/overdue.sh
#!/bin/bash
days=$(( ($(date +%s) - $(git log -1 --format=%ct)) / 86400 ))
if [ $days -gt 30 ]; then
  echo "⚠️ Monthly close overdue ($days days)"
  exit 0
fi
exit 1
```

**Pros:**
- Config is minimal and readable
- Full flexibility (scripts can do anything, any language)
- No inline bash in YAML
- Scripts can be tested independently (`./status/overdue.sh`)
- Clear separation: config declares what checks exist, scripts implement them

**Cons:**
- Logic is hidden from the config (have to open script files)
- More files to manage
- Harder to see "at a glance" what checks do

### Option 2: Declarative with Built-in Check Types

**Predefined check types with configuration:**

```yaml
status:
  - type: time_since_commit
    threshold_days: 30
    message: "Monthly close overdue"

  - type: file_count
    path: "inbox/"
    threshold: 10
    message: "{{count}} captures to organize"

  - type: uncommitted_files
    message: "{{count}} uncommitted files"
```

Grove implements built-in check types. Variables like `{{count}}` are provided by the check type.

**Pros:**
- Very readable and declarative
- No scripting knowledge needed
- Variables are explicit in the config
- Common cases are simple

**Cons:**
- Limited to built-in checks (what if you need something custom?)
- Requires Grove to implement each check type
- Creates a DSL that must be documented and maintained
- What happens when built-ins don't cover your case?

### Option 3: Hybrid - Declarative + Scripts

**Built-ins for common cases, scripts for custom:**

```yaml
status:
  # Built-in checks
  - type: uncommitted_files
    message: "{{count}} uncommitted files"

  - type: days_since_commit
    threshold: 30
    message: "No activity in {{days}} days"

  # Custom check
  - name: uncategorized-transactions
    run: "./scripts/check-transactions.sh"
```

**Pros:**
- Simple cases are simple (built-ins)
- Complex cases are possible (scripts)
- Variables are explicit for built-ins

**Cons:**
- Two paradigms to learn and maintain
- Still requires implementing built-in checks
- Inconsistent syntax between built-ins and custom
- Complexity: which approach to use when?

### Option 4: Expression Language

**Simple expression language for queries:**

```yaml
status:
  - name: overdue
    when: "git.days_since_commit > 30"
    message: "Monthly close overdue ({{git.days_since_commit}} days)"

  - name: inbox-overflow
    when: "files.count('inbox/') > 10"
    message: "{{files.count('inbox/')}} captures to organize"
```

**Pros:**
- Declarative but flexible
- Variables are explicit
- Can express many checks without scripts
- Single paradigm

**Cons:**
- Requires an expression evaluator (parser, runtime)
- Another language to learn and document
- Still can't handle all cases (might need scripts anyway)
- Complexity: defining the expression language, error handling

### Option 5: Helper Commands

**Use helper utilities to handle common patterns:**

```yaml
status:
  overdue:
    run: "graft-check days-since-commit --threshold 30 --message 'Monthly close overdue'"

  inbox:
    run: "graft-check file-count inbox/ --min 10 --message '{count} captures to organize'"

  custom:
    run: "./scripts/custom-check.sh"
```

`graft-check` is a utility that handles common patterns. Variables like `{count}` are filled in by the helper.

**Pros:**
- No scripts needed for common cases
- Still flexible (can use any command)
- Variables handled by the helper
- Can fall back to custom scripts for complex cases

**Cons:**
- Requires implementing `graft-check` helpers
- Another tool to document and maintain
- Still has two paradigms (helpers vs custom scripts)

---

## Commands vs Status: Are They the Same?

**Similarities:**
- Both are shell scripts with metadata
- Both execute in repo context
- Both produce output and exit codes

**Differences:**

### Commands (imperative - "do this")

**Invocation:** User explicitly runs: `graft repo:test`

**Output:**
- All stdout/stderr streamed to user in real-time
- Shows progress, errors, detailed output
- User sees everything

**Exit code:**
- `0` = success
- `non-zero` = failure
- Reported to user

**Side effects:** Expected and normal
- Run tests, generate reports, modify files, make commits

**Example:**
```bash
$ graft finances:monthly-close
Generating report for January 2026...
Processing 127 transactions...
Report saved to reports/2026-01.pdf
✓ Success
```

### Status (interrogative - "does this need attention?")

**Invocation:** Automatically by Grove/graft during status checks

**Output:**
- Captured silently during execution
- If output is non-empty → signal present
- Empty output → no signal
- Only the message is displayed in aggregate view

**Exit code:**
- `0` = check completed successfully
- `non-zero` = error running check (log/warn, don't show as signal)
- Exit code doesn't indicate signal presence—stdout does

**Side effects:** Should be avoided
- Status checks should be read-only queries
- Convention, not enforced

**Example:**
```bash
# Grove runs silently
$ ./status/overdue.sh
Monthly close overdue (32 days)

# Grove displays in aggregate:
● personal-finances
  ⚠️ Monthly close overdue (32 days)
```

**Conclusion:** They're similar primitives (shell scripts) but with different usage semantics. Whether to unify them with tags or keep them separate is an implementation choice.

---

## The Recommended Approach

**Start with the simplest possible primitive that could work.**

### Config syntax:

```yaml
# Minimal: just name → path mapping
status:
  overdue: "./status/overdue.sh"
  inbox: "./status/inbox.sh"
  uncategorized: "./status/uncategorized.sh"
```

Or even simpler with convention:
```yaml
# Names only, assumes ./status/<name>.sh
status:
  - overdue
  - inbox
  - uncategorized
```

### The contract:

**Scripts:**
- Exit 0 + output message → signal present, show the message
- Exit 1 or no output → no signal, show nothing
- That's it

**Scripts can be any language:**

```bash
#!/bin/bash
# status/overdue.sh
days=$(( ($(date +%s) - $(git log -1 --format=%ct)) / 86400 ))
[ $days -gt 30 ] && echo "⚠️ Monthly close overdue ($days days)"
```

```python
#!/usr/bin/env python3
# status/inbox.sh
import os
count = len(os.listdir('inbox/'))
if count > 10:
    print(f"📥 {count} captures to organize")
    exit(0)
exit(1)
```

```javascript
#!/usr/bin/env node
// status/uncategorized.sh
const fs = require('fs');
const content = fs.readFileSync('transactions.ledger', 'utf8');
const count = (content.match(/TODO/g) || []).length;
if (count > 0) {
  console.log(`📋 ${count} uncategorized transactions`);
  process.exit(0);
}
process.exit(1);
```

### Why this is the right starting point:

**Simplicity:**
- Config is a name → path map (or just names with convention)
- No YAML variables
- No inline code
- No DSL to learn or maintain
- No built-in checks to implement

**Flexibility:**
- Scripts use whatever language makes sense for the repo
- Full power of programming languages available
- Can check anything (files, git state, external APIs, databases)
- No artificial limitations

**Clarity:**
- Config declares "these checks exist"
- Scripts implement "how to check"
- Clear separation of concerns
- Variables aren't in the config at all—they're internal to scripts

**Testability:**
- Scripts can be run directly: `./status/overdue.sh`
- Easy to debug
- Easy to test in CI
- No special harness needed

**How it serves the goal:**
- ✅ Tools can query programmatically (run scripts, collect output)
- ✅ Humans can understand (read config for names, read scripts for logic)
- ✅ Works for diverse needs (any script, any language)
- ✅ No complex DSL (just exit codes and stdout)

---

## Evolution Path

### Version 1: Scripts only
```yaml
status:
  overdue: "./status/overdue.sh"
  inbox: "./status/inbox.sh"
```

Pure simplicity. Learn what patterns emerge.

### Version 2: Optional helpers (if needed)

If common patterns emerge, consider helper utilities:

```yaml
dependencies:
  graft-checks:
    source: "https://github.com/graft/graft-checks.git"

status:
  overdue: ".graft/graft-checks/days-since-commit.sh 30 'Monthly close overdue'"
  inbox: ".graft/graft-checks/file-count.sh inbox/ 10"
  custom: "./status/custom-check.sh"
```

Or a helper command:
```yaml
status:
  overdue:
    run: "graft-check days-since-commit 30"
    env:
      MESSAGE: "Monthly close overdue"
```

### Version 3: Declarative shortcuts (if truly needed)

Only if evidence shows scripts are too heavy for common cases:

```yaml
status:
  # Declarative for simple cases
  - type: days_since_commit
    threshold: 30
    message: "Monthly close overdue"

  # Scripts for custom cases
  - name: complex-check
    run: "./status/complex.sh"
```

**But don't build this until version 1 proves it's needed.**

---

## Key Design Principles Discovered

### 1. Stop trying to make the config executable

The config should be **declarative** (names → scripts), not executable (inline code). Let scripts handle logic.

### 2. Variables don't belong in the config

Variables are internal to script implementation. The config declares "these checks exist." The scripts compute and format messages.

### 3. Embrace the file system

Scripts are files. This is good—files are testable, versionable, reviewable, diffable. Don't fight it.

### 4. Optimize for the common case, but don't limit the uncommon

Most repos might have 2-5 status checks. A few extra script files is fine. Don't add complexity to save a few files.

### 5. Prefer conventions over configuration

If status scripts live in `./status/` by convention, the config can be even simpler:
```yaml
status: [overdue, inbox, uncategorized]
```

Maps to: `./status/overdue.sh`, `./status/inbox.sh`, `./status/uncategorized.sh`

---

## Relationship to Commands

Given this simpler framing for status, should status and commands be unified?

### Option A: Keep separate (recommended for v1)

```yaml
status:
  overdue: "./status/overdue.sh"
  inbox: "./status/inbox.sh"

commands:
  monthly-close:
    run: "./scripts/report.sh"
    description: "Generate monthly report"

  test:
    run: "npm test"
    description: "Run test suite"
```

Clear separation of intent. Easy to understand at a glance.

### Option B: Unify with tags (consider for v2+)

```yaml
commands:
  overdue:
    run: "./status/overdue.sh"
    tags: [status]

  inbox:
    run: "./status/inbox.sh"
    tags: [status]

  monthly-close:
    run: "./scripts/report.sh"
    description: "Generate monthly report"
    tags: [workflow]
```

Single primitive, but less explicit about status checks.

**Recommendation:** Start with separate sections. The distinction between "checking state" (status) and "taking action" (commands) is meaningful. If tags prove valuable for other reasons later, we can unify.

---

## Impact on Graft Spec

### Minimal graft.yaml with status:

```yaml
metadata:
  name: "personal-finances"

status:
  overdue: "./status/overdue.sh"
  uncategorized: "./status/uncategorized.sh"

commands:
  monthly-close:
    run: "./scripts/report.sh"
    description: "Generate monthly report"
```

### Status section spec:

```yaml
status:
  <name>: <path-to-script>
  # or
  <name>:
    run: <path-to-script>
    description: <optional-description>
```

**Contract:**
- Script exits 0 + outputs message → signal present
- Script exits non-zero or no output → no signal
- Script can be any executable (bash, python, node, ruby, compiled binary)
- Script runs in repo root directory
- Script can use git commands, read files, run programs

---

## Impact on Grove

### Running status checks:

```bash
# Run all status checks for a repo
graft status
# or
graft status --format json
```

Returns:
```json
{
  "overdue": {
    "signal": true,
    "message": "⚠️ Monthly close overdue (32 days)",
    "exit_code": 0
  },
  "uncategorized": {
    "signal": false,
    "message": "",
    "exit_code": 1
  }
}
```

### Grove home page:

```
┌─ Grove: personal workspace ─────────────────────────────────┐
│                                                              │
│ Needs Attention (2):                                         │
│   ● personal-finances                                        │
│     ⚠️ Monthly close overdue (32 days)                      │
│     📋 3 uncategorized transactions                          │
│                                                              │
│   ● general-notes                                            │
│     📥 12 captures to organize                              │
│                                                              │
│ Active (1):                                                  │
│   ● grove-brainstorming  main · 2 uncommitted files         │
│                                                              │
│ Clean (3): ...                                               │
└──────────────────────────────────────────────────────────────┘
```

Grove runs status scripts for each repo and aggregates the results.

---

## Open Questions

1. **Script location convention:** Require `./status/` directory, or allow arbitrary paths?
2. **Script naming:** `<name>.sh` or just `<name>` (no extension requirement)?
3. **Error handling:** If a status script exits non-zero, how should Grove report that? (Log? Warn? Show as "error checking status"?)
4. **Caching:** Should status results be cached, or always recomputed? If cached, what's the invalidation strategy?
5. **Async execution:** Should status checks run in parallel (faster but more complex) or serially (simpler but slower)?
6. **Standard library:** Should graft provide a collection of common status check scripts that repos can use/reference?

---

## Next Steps

1. **Finalize the status section spec** in graft.yaml format documentation
2. **Implement `graft status` command** to run status checks and output results
3. **Add status check support to Grove Slice 1** to display aggregate status on home page
4. **Document the contract** for status check scripts (exit codes, stdout semantics)
5. **Create example status scripts** for common patterns (days since commit, file count, git status, etc.)
6. **Test with real repos** to validate the approach works for diverse use cases

---

## Key Insight

**The core realization: Stop trying to put logic in YAML. Put declarations in YAML, logic in scripts.**

Config declares:
- "This repo has status checks"
- "They're named X, Y, Z"
- "They're implemented by these scripts"

Scripts implement:
- The checking logic
- The message formatting
- The dynamic data computation

This separation is clean, testable, and extensible without requiring Grove or Graft to understand every possible check type.

---

## Sources

- [Grove as Workflow Hub: Design Primitives (2026-02-07)](./2026-02-07-grove-workflow-hub-primitives.md) - Original status check proposal
- [Grove Vertical Slices (2026-02-06)](./2026-02-06-grove-vertical-slices.md) - Implementation plan
- [Graft Commands Specification](../docs/specifications/graft/graft-yaml-format.md) - Current command format (if exists)
