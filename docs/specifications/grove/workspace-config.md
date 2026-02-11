---
status: working
last-verified: 2026-02-10
owners: [human, agent]
---

# Workspace Configuration

## Intent

Define the workspace.yaml format that declares which repositories belong to a workspace and configures Grove's behavior. The workspace is Grove's core organizing primitive — a named collection of git repositories that you work with together.

## Non-goals

- **Not a complex project file** — Keep it simple, just repo declarations and basic settings
- **Not a replacement for graft.yaml** — Grove reads graft configs, doesn't duplicate them
- **Not environment-specific** — Same config works across machines (paths can be relative to home)
- **Not a build system** — Grove navigates and captures, doesn't build or deploy

## Behavior

### Basic Workspace Declaration [Slice 1]

```gherkin
Given a workspace config at ~/.config/grove/workspace.yaml:
  """
  name: "my-project"
  repositories:
    - path: ~/src/graft-knowledge
    - path: ~/src/meta-knowledge-base
  """
When Grove launches
Then it loads both repositories into the workspace registry
And displays their names and status
```

### Repository with Tags [Unimplemented]

```gherkin
Given a repository declaration with tags:
  """
  repositories:
    - path: ~/finances
      tags: [finances, monthly-cadence, high-priority]
  """
When Grove displays the repository list
Then it shows the tags for filtering
And can sort by tag weights
```

### Capture Configuration [Slice 3]

```gherkin
Given a workspace with capture config:
  """
  capture:
    default_inbox: ~/notes/inbox/
    auto_commit: true
  """
When a user captures a note
Then it is saved to ~/notes/inbox/YYYY-MM-DDTHH-MM-SS-title.md
And automatically committed with message "capture: <first line>"
```

### Capture Routing by Prefix [Slice 3]

```gherkin
Given a workspace with capture routes:
  """
  capture:
    default_inbox: ~/notes/inbox/
    routes:
      - prefix: "@finances"
        path: ~/finances/notes/
      - prefix: "@todo"
        path: ~/notes/todo/
  """
When a user captures "@finances lunch $15"
Then the note is created at ~/finances/notes/YYYY-MM-DDTHH-MM-SS-finances-lunch-15.md
And the prefix is stripped from the filename
```

```gherkin
Given the same routing config
When a user captures "random thought" (no prefix)
Then the note is created at ~/notes/inbox/YYYY-MM-DDTHH-MM-SS-random-thought.md
```

### Repository Status Scripts [Slice 1]

```gherkin
Given a repository with custom status checks:
  """
  repositories:
    - path: ~/finances
      status:
        - name: overdue
          run: |
            days=$(( ($(date +%s) - $(git log -1 --format=%ct)) / 86400 ))
            [ $days -gt 30 ] && echo "Monthly close overdue ($days days)"
  """
When Grove refreshes status for the finances repository
Then it executes the status script in the repo's directory
And if the script exits 0 with output, displays that output as a signal
And if the script exits 0 with no output, shows nothing (all clear)
And if the script exits non-zero, logs a warning and does not display a signal
```

### Multiple Workspace Configurations [Slice 1]

```gherkin
Given multiple workspace files exist:
  - ~/.config/grove/workspace-personal.yaml
  - ~/.config/grove/workspace-work.yaml
When user runs `grove --workspace work`
Then Grove loads workspace-work.yaml
```

```gherkin
Given the user launched Grove with workspace-personal yesterday
When user runs `grove` today (no workspace flag)
Then Grove loads workspace-personal.yaml (last used)
```

### Search Exclusions [Slice 6]

```gherkin
Given a workspace with search config:
  """
  search:
    exclude: ["node_modules", ".git", "vendor", "target"]
  """
When Grove indexes the workspace for search
Then it skips all directories matching the exclusion patterns
```

### Edge Cases

#### Missing Required Fields

```gherkin
Given a workspace config without 'name':
  """
  repositories:
    - path: ~/src/repo
  """
When Grove tries to load the config
Then it fails with error "workspace.yaml: missing required field 'name'"
```

#### Non-existent Repository Path

```gherkin
Given a repository path that doesn't exist:
  """
  repositories:
    - path: ~/nonexistent
  """
When Grove loads the workspace
Then it shows a warning "Repository not found: ~/nonexistent"
And continues loading other repositories
```

#### Repository Path is Not a Git Repo

```gherkin
Given a path that exists but isn't a git repository:
  """
  repositories:
    - path: ~/not-git
  """
When Grove loads the workspace
Then it shows a warning "Not a git repository: ~/not-git"
And continues loading other repositories
```

#### Status Script Exits Non-Zero

```gherkin
Given a status check that fails:
  """
  status:
    - name: broken
      run: exit 1
  """
When Grove executes the status check
Then it logs a warning about the script error
And does not display any signal for that check
```

#### Status Script Exits Zero with No Output

```gherkin
Given a status check that succeeds silently:
  """
  status:
    - name: overdue
      run: |
        days=$(( ($(date +%s) - $(git log -1 --format=%ct)) / 86400 ))
        [ $days -gt 30 ] && echo "Monthly close overdue ($days days)"
  """
When the repo was committed to yesterday (not overdue)
And Grove executes the status check
Then the script exits 0 with no output
And Grove displays no signal for that check (all clear)
```

#### Capture to Non-existent Directory

```gherkin
Given a capture default_inbox that doesn't exist:
  """
  capture:
    default_inbox: ~/notes/new-inbox/
  """
When a user captures a note
Then Grove creates the directory ~/notes/new-inbox/
And saves the capture file
```

#### Overlapping Capture Prefixes

```gherkin
Given capture routes with overlapping prefixes:
  """
  routes:
    - prefix: "@fin"
      path: ~/fin/
    - prefix: "@finances"
      path: ~/finances/
  """
When a user captures "@finances note"
Then it matches "@finances" (longest prefix match)
And routes to ~/finances/
```

## Annotated Example

Complete workspace.yaml showing the full configuration surface:

```yaml
# Workspace identity
name: "my-project"

# Repository declarations (at least one required)
repositories:
  - path: ~/src/graft-knowledge         # Required: absolute or ~ path
    tags: [knowledge, high-priority]     # Optional: labels for filtering
    status:                              # Optional: custom status checks
      - name: inbox-overflow             # Status check identifier
        run: |                           # Shell script (any language)
          count=$(ls notes/inbox/ 2>/dev/null | wc -l)
          [ $count -gt 10 ] && echo "$count captures to organize"

  - path: ~/src/my-app
    tags: [app, active]

# Capture configuration (optional)
capture:
  default_inbox: ~/notes/inbox/          # Where unrouted captures go
  auto_commit: true                      # Auto-commit captures (default: false)
  template: |                            # Capture file template (optional)
    ---
    date: {{date}}
    ---
    {{content}}
  routes:                                # Prefix-based routing (optional)
    - prefix: "@finances"                # Match prefix (longest wins)
      path: ~/finances/notes/            # Target directory
      template: |                        # Override template per route
        ---
        date: {{date}}
        type: transaction
        ---
        {{content}}
    - prefix: "@todo"
      path: ~/notes/todo/

# Search configuration (optional)
search:
  exclude: ["node_modules", ".git", "vendor", "target"]

# Tag weights for sorting (optional, higher = higher priority)
tag_weights:
  high-priority: 100
  active: 50
```

## Constraints

### Performance
- Config parse time < 10ms
- Status script execution timeout: 5 seconds per script

### Security
- Status scripts run in repository directory with user's shell environment
- No elevation or special permissions
- Scripts inherit user's PATH and git credentials

### Compatibility
- YAML 1.2 format
- Paths support `~` expansion (home directory)
- Status scripts use `/bin/sh` (POSIX shell)
- Works across Linux, macOS, Windows (with sh available)

## Open Questions

- [ ] Should repos be auto-discovered (scan ~/src/) or explicit-only?
- [ ] Should status scripts have access to Grove's internal state?
- [ ] Should templates support more complex logic (conditionals, loops)?
- [ ] Should capture routing support regex patterns instead of just prefix matching?
- [ ] Should workspace configs support inheritance/composition (base + override)?
- [ ] Should status checks run in parallel or serially?
- [ ] Should there be a schema validation tool (grove validate-config)?
- [ ] Should status scripts be defined in workspace.yaml (current), per-repo `.grove.yaml`, or both?

## Decisions

- **2026-02-07**: Status scripts are shell commands, not a DSL
  - Maximally flexible — can call any program
  - Composes with everything (Python, jq, database queries)
  - Zero learning curve (everyone knows shell)
  - Same pattern as graft commands

- **2026-02-07**: Workstreams are just multiple workspace files
  - Reuses workspace primitive with no modification
  - Switching workstreams = switching config files
  - No complex "active subset" logic needed
  - Launch with `grove --workspace <name>`

- **2026-02-07**: Tags are simple strings, not hierarchical
  - Flat is simpler than nested categories
  - Users can adopt their own conventions
  - Compose with filters and weights naturally

- **2026-02-07**: Capture routing uses prefix matching
  - Simple to understand and use
  - Works well with natural language ("`@finances` for money stuff")
  - Longest prefix match wins (standard routing behavior)

- **2026-02-08**: Status script exit code semantics
  - Exit 0 + output → signal present (display the message)
  - Exit 0 + no output → no signal (all clear, silent)
  - Exit non-zero → script error (log warning, don't display as signal)
  - Output presence determines signal, not exit code value

## Sources

- [Workspace UI Exploration (2026-02-06)](../../../notes/2026-02-06-workspace-ui-exploration.md) — Original workspace config design
- [Grove Workflow Hub Primitives (2026-02-07)](../../../notes/2026-02-07-grove-workflow-hub-primitives.md) — Six design primitives: status scripts, workstreams as configs, tags, capture routing
- [Status Check Syntax Exploration (2026-02-08)](../../../notes/2026-02-08-status-check-syntax-exploration.md) — Status script semantics and examples
