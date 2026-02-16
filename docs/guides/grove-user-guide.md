---
status: stable
updated: 2026-02-16
---

# Grove User Guide

## Quick Start

Grove is a multi-repo workspace manager that shows git status across your repositories in a terminal UI.

### Installation

```bash
cd grove
cargo build --release
cp target/release/grove ~/.local/bin/  # or wherever you keep binaries
```

### First Run

1. Create a workspace configuration:
   ```bash
   mkdir -p ~/.config/grove
   ```

2. Edit `~/.config/grove/workspace.yaml` (see Configuration below)

3. Run Grove:
   ```bash
   grove
   ```

---

## Configuration

### Workspace Configuration File

Grove reads its configuration from `~/.config/grove/workspace.yaml` by default.

**Basic structure:**
```yaml
name: my-workspace
repositories:
  - path: ~/src/project1
    tags: [rust, cli]
  - path: ~/src/project2
    tags: [python, web]
```

### Configuration Options

#### `name` (required)
The name of your workspace. Must be non-empty.

```yaml
name: my-workspace
```

#### `repositories` (required)
List of repository declarations.

Each repository has:
- **`path`** (required): Absolute or tilde-expanded path to the repository
- **`tags`** (optional): List of tags for organization (not currently displayed in TUI)

**Examples:**
```yaml
repositories:
  # Absolute path
  - path: /home/user/src/graft
    tags: [python, cli, graft]

  # Tilde expansion
  - path: ~/src/grove
    tags: [rust, tui]

  # No tags
  - path: ~/work/project
```

### Custom Config Location

Override the default config location:
```bash
grove --workspace ~/my-workspace.yaml
```

Or set environment variable:
```bash
export GROVE_WORKSPACE=~/my-workspace.yaml
grove
```

---

## Using Grove

### Keyboard Controls

| Key | Action |
|-----|--------|
| `j` or `↓` | Move selection down |
| `k` or `↑` | Move selection up |
| `q` or `Esc` | Quit |

**Note:** Selection wraps around (from bottom to top, top to bottom).

### Status Indicators

Grove displays repository status with the following indicators:

#### Branch Name
```
[main]      Current branch
[feature]   Feature branch
[detached]  Detached HEAD state
```

#### Clean/Dirty Status
```
○  Clean working tree (no uncommitted changes)
●  Dirty working tree (uncommitted changes present)
```

Colors:
- Green `○` = clean
- Yellow `●` = dirty

#### Ahead/Behind Counts
```
↑3  3 commits ahead of remote
↓2  2 commits behind remote
↑1 ↓2  Both ahead and behind
```

**Note:** Ahead/behind counts require remote tracking branch configuration.

### Error Indicators

If Grove can't read a repository's status, it shows:
```
[error: Failed to open repository: ...]
```

Common causes:
- Path doesn't exist
- Path is not a git repository
- Permission denied

Grove continues displaying other repositories even if one fails.

---

## Debugging & Logging

Grove uses the `log` crate with `env_logger` for configurable logging output.

### Enabling Logs

Control log verbosity with the `RUST_LOG` environment variable:

```bash
# Info level (shows startup, summary, warnings)
RUST_LOG=grove=info grove

# Debug level (shows config loading, refresh progress)
RUST_LOG=grove=debug grove

# Trace level (shows all git operations, timing)
RUST_LOG=grove=trace grove

# All levels (very verbose)
RUST_LOG=trace grove
```

### Log Levels

| Level | What You'll See |
|-------|----------------|
| `error` | Critical errors only |
| `warn` | Errors + warnings (failed repos) |
| `info` | Warnings + startup/summary info |
| `debug` | Info + config loading, refresh progress |
| `trace` | Debug + individual git operations |

### Example Output

**Info level:**
```bash
$ RUST_LOG=grove=info grove
[INFO grove] Grove 0.1.0 starting
[INFO grove] Loaded workspace: my-workspace
[INFO grove] Successfully refreshed 10 repositories
```

**Debug level:**
```bash
$ RUST_LOG=grove=debug grove
[INFO grove] Grove 0.1.0 starting
[DEBUG grove] Platform: linux
[DEBUG grove] Loading workspace config from: ~/.config/grove/workspace.yaml
[INFO grove] Loaded workspace: my-workspace
[DEBUG grove] Repositories: 10
[DEBUG grove] Refreshing repository status...
[DEBUG grove_engine::registry] Refreshing 10 repositories...
[DEBUG grove_engine::registry] Refresh complete: 10 successful, 0 failed
[INFO grove] Successfully refreshed 10 repositories
[DEBUG grove] Launching TUI...
```

### Troubleshooting with Logs

When reporting issues or debugging problems:

1. **Enable debug logs:**
   ```bash
   RUST_LOG=grove=debug grove 2>&1 | tee grove-debug.log
   ```

2. **Check for warnings:**
   ```bash
   RUST_LOG=grove=warn grove 2>&1 | grep -i warn
   ```

3. **Identify slow operations:**
   ```bash
   RUST_LOG=grove=trace grove 2>&1 | grep "query_status"
   ```

### Performance Tuning

If Grove is slow to start:

```bash
# Enable trace logs to see which repos are slow
RUST_LOG=grove=trace grove 2>&1 | grep "query_status"

# Adjust git timeout (default: 5000ms = 5 seconds)
GROVE_GIT_TIMEOUT_MS=10000 grove  # 10 second timeout
```

---

## Troubleshooting

### "Failed to load workspace from '~/.config/grove/workspace.yaml'"

**Cause:** Config file doesn't exist or has wrong format.

**Fix:**
1. Check file exists: `ls -la ~/.config/grove/workspace.yaml`
2. Validate YAML syntax (use a YAML validator)
3. Ensure required fields are present (`name`, `repositories`)

### "Failed to open repository at /path/to/repo"

**Cause:** Path doesn't exist or isn't a git repository.

**Fix:**
1. Verify path: `ls -la /path/to/repo`
2. Check it's a git repo: `git -C /path/to/repo status`
3. Fix path in workspace.yaml

### Grove hangs on startup

**Cause:** Repository on slow network filesystem (NFS, networked drive) or git operation is stuck.

**Behavior:** Git operations timeout after 5 seconds per repository.

**Workaround:**
1. Check which repo is slow: `RUST_LOG=grove=debug grove` (shows progress)
2. Temporarily remove slow repos from workspace.yaml
3. Future: Configurable timeout coming in later release

**Note:** Maximum startup time = `5 seconds × number of repositories`

### Repository shows "[error: ...]" indicator

**Causes:**
- Not a valid git repository
- Git command failed or timed out
- Permission denied

**Debug Steps:**
1. Enable debug logging: `RUST_LOG=grove=debug grove`
2. Check logs for specific error
3. Verify repository manually: `git -C /path/to/repo status`
4. Fix the repository or remove from workspace.yaml

### Path expansion not working as expected

**Supported:**
- `~/path` - Expands to home directory
- `$VAR/path` - Expands environment variables

**NOT supported:**
- `~username/path` - Other user's home directories
- `~+` - Current directory (PWD)
- `~-` - Previous directory (OLDPWD)

**Undefined variables:**
- `$UNDEFINED/path` → Literal string `$UNDEFINED/path` (no error)
- Check with: `echo $VAR_NAME` before using in config

### Grove is slow with many repositories

**Cause:** Status refresh is serial (one repo at a time).

**Performance:**
- Typical: 50-100ms per repository
- Worst case: 5 seconds per repository (timeout)
- Example: 20 repos = 1-100 seconds startup time

**Workarounds:**
1. Split large workspaces into multiple config files:
   ```bash
   grove --workspace ~/.config/grove/work.yaml
   grove --workspace ~/.config/grove/personal.yaml
   ```
2. Remove inactive repositories from workspace
3. Future: Parallel queries planned for performance optimization

### Environment variable GROVE_WORKSPACE not recognized

**Fix:** Ensure you have Grove version 0.1.0+ (environment variable support added in Slice 1 Phase 3A).

**Check version:**
```bash
grove --version
```

**Usage:**
```bash
export GROVE_WORKSPACE=~/my-workspace.yaml
grove
```

---

## Limitations (Slice 1)

### Current Limitations

1. **Selection doesn't do anything**
   - Can navigate list with j/k
   - Pressing Enter has no effect
   - Future: open repo details (Slice 2)

2. **Serial git queries**
   - Repositories queried one at a time
   - May be slow with many repos (>20)
   - Future: parallel queries for performance

### Planned Features (Future Slices)

- **Slice 2:** Repository detail pane (commit log, changed files)
- **Slice 3:** Quick capture (create notes from TUI)
- **Slice 4:** File navigation and $EDITOR integration
- **Slice 5:** Graft metadata display
- **Slice 6:** Cross-repo search
- **Slice 7:** Command execution

---

## Example Workspace Configurations

### Single Project
```yaml
name: my-project
repositories:
  - path: ~/src/my-project
```

### Multi-Project Development
```yaml
name: work-projects
repositories:
  - path: ~/work/frontend
    tags: [javascript, react]
  - path: ~/work/backend
    tags: [rust, api]
  - path: ~/work/mobile
    tags: [swift, ios]
  - path: ~/work/docs
    tags: [markdown]
```

### Graft + Dependencies
```yaml
name: graft-workspace
repositories:
  - path: ~/src/graft
    tags: [python, cli, graft]
  - path: ~/src/graft/grove
    tags: [rust, tui, grove]
  - path: ~/src/graft/.graft/rust-starter
    tags: [template, rust]
  - path: ~/src/graft/.graft/meta-knowledge-base
    tags: [docs, knowledge-base]
```

### Learning Rust
```yaml
name: rust-learning
repositories:
  - path: ~/learning/rust-book-examples
    tags: [rust, learning]
  - path: ~/learning/exercism-rust
    tags: [rust, exercises]
  - path: ~/projects/my-first-cli
    tags: [rust, cli, project]
```

---

## Tips & Best Practices

### Organizing with Tags

While tags aren't displayed in Slice 1, they're useful for:
- **Documentation:** Remembering what each repo is for
- **Future features:** Filtering, grouping, searching
- **Context switching:** Quick reference when editing config

**Suggested tag categories:**
- **Language:** `rust`, `python`, `javascript`, `go`
- **Type:** `cli`, `web`, `api`, `library`, `docs`
- **Status:** `active`, `archived`, `learning`
- **Project:** `graft`, `work`, `personal`

### Keeping Config in Sync

**Version control your workspace config:**
```bash
mkdir ~/dotfiles
cp ~/.config/grove/workspace.yaml ~/dotfiles/grove-workspace.yaml
cd ~/dotfiles && git add grove-workspace.yaml && git commit -m "Add Grove workspace"
```

**Symlink approach:**
```bash
ln -s ~/dotfiles/grove-workspace.yaml ~/.config/grove/workspace.yaml
```

### Performance with Many Repos

If Grove feels slow with many repositories:
1. **Split into multiple workspaces** (use `--workspace` flag)
2. **Remove archived repos** (ones you rarely touch)
3. **Wait for future optimization** (parallel queries coming)

---

## Getting Help

### Resources

- **Planning Docs:** `docs/grove/planning/`
- **Architecture:** `docs/grove/implementation/`
- **Specifications:** `docs/specifications/grove/`
- **Source Code:** `crates/grove-*/`

### Reporting Issues

When reporting issues, include:
1. Grove version: `grove --version`
2. Your OS and terminal emulator
3. Workspace config (sanitize paths if needed)
4. Steps to reproduce
5. Expected vs. actual behavior

### Contributing

Grove is in active development! Contributions welcome:
- Bug fixes
- Feature implementations (see roadmap)
- Documentation improvements
- Test coverage

See planning docs for upcoming features and current status.
