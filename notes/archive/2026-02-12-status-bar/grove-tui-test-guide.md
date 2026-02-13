# Grove TUI Command Execution - Test Guide

## Build and Run

```bash
# Build Grove
cd /home/coder/src/graft/grove
cargo build --release

# Run Grove with test workspace
cd /tmp/grove-test
grove --workspace workspace.yaml
```

## Testing the Command Execution Feature

### Test 1: Open Command Picker
1. Launch Grove (you should see repo1 in the list)
2. Press `x` to open the command picker
3. **Expected:** Modal overlay appears with 5 commands listed
4. **Verify:** Each command shows name and description

### Test 2: Navigate Commands
1. With command picker open, press `j` or `↓` to move down
2. Press `k` or `↑` to move up
3. **Expected:** Selection highlight moves between commands
4. **Verify:** Currently selected command is highlighted

### Test 3: Execute Simple Command
1. Navigate to "hello" command
2. Press `Enter` to execute
3. **Expected:**
   - Output pane opens immediately
   - Header shows "Running: hello"
   - Output appears: "Hello from Grove TUI!"
   - Header changes to "✓ hello: Completed successfully (exit 0)"
4. Press `q` to close output pane
5. **Verify:** Returns to repository list

### Test 4: Test Streaming Output
1. Press `x` to open command picker
2. Select "slow" command
3. Press `Enter`
4. **Expected:**
   - Output appears line-by-line (every 0.5 seconds)
   - "Line 1", "Line 2", etc. appear incrementally
   - UI remains responsive during execution
5. **Verify:** Lines appear one at a time, not all at once

### Test 5: Test Failed Command
1. Press `x`, select "fail"
2. Press `Enter`
3. **Expected:**
   - Output shows "This will fail"
   - Header shows "✗ fail: Failed with exit code 42"
4. **Verify:** Exit code is displayed correctly

### Test 6: Test Multiline Output
1. Press `x`, select "multiline"
2. Press `Enter`
3. **Expected:** All 3 lines displayed
4. **Verify:** Multiple lines of output shown correctly

### Test 7: Test Output Scrolling
1. Create a command with many lines:
   ```bash
   cd /tmp/grove-test/repo1
   cat >> graft.yaml <<'EOF'
   longoutput:
     run: "for i in {1..100}; do echo \"Line $i\"; done"
     description: "100 lines of output"
   EOF
   ```
2. Restart Grove
3. Execute "longoutput" command
4. Press `j` to scroll down, `k` to scroll up
5. **Expected:** Output scrolls smoothly
6. **Verify:** Can navigate through all 100 lines

### Test 8: Close Command Picker Without Executing
1. Press `x` to open picker
2. Press `q` or `Esc`
3. **Expected:** Picker closes, returns to repo list
4. **Verify:** No command executed

### Test 9: Repository Without Commands
1. Create another repo without commands:
   ```bash
   mkdir /tmp/grove-test/repo2
   cd /tmp/grove-test/repo2
   git init
   echo "name: test" > graft.yaml
   git add . && git commit -m "Init"

   # Add to workspace
   cd /tmp/grove-test
   cat >> workspace.yaml <<'EOF'
   - path: ./repo2
     tags: []
   EOF
   ```
2. Restart Grove, select repo2
3. Press `x`
4. **Expected:** Status message "No commands defined in graft.yaml"
5. **Verify:** Picker doesn't open

### Test 10: Repository Without graft.yaml
1. Create repo without any graft.yaml:
   ```bash
   mkdir /tmp/grove-test/repo3
   cd /tmp/grove-test/repo3
   git init
   touch README.md
   git add . && git commit -m "Init"

   cd /tmp/grove-test
   cat >> workspace.yaml <<'EOF'
   - path: ./repo3
     tags: []
   EOF
   ```
2. Restart Grove, select repo3
3. Press `x`
4. **Expected:** Status message shows no commands available
5. **Verify:** Handled gracefully, no crash

## Feature Checklist

- [x] Press `x` opens command picker
- [x] Command picker shows all commands from graft.yaml
- [x] j/k navigation works in picker
- [x] Enter executes selected command
- [x] q/Esc closes picker without executing
- [x] Output pane shows command status (Running/Completed/Failed)
- [x] Streaming output appears incrementally
- [x] Exit codes displayed correctly (0 = success, non-zero = failure)
- [x] Multiple lines of output displayed
- [x] j/k scrolling works in output pane
- [x] q closes output pane
- [x] Repos without commands handled gracefully
- [x] Repos without graft.yaml handled gracefully
- [x] Command picker caches parsed commands (press x twice on same repo = instant)
- [x] Help overlay updated with 'x' keybinding

## Known Limitations

1. **No ANSI color support yet** - Colors are rendered as escape codes
   - The `ansi-to-tui` dependency is added but not yet integrated
   - Future enhancement: parse ANSI codes in output lines

2. **No output size limit warning** - Will truncate silently at 1MB
   - Truncation warning IS shown, but only when limit reached

3. **Cannot cancel running commands** - Must wait for completion
   - Pressing `q` closes the pane but command continues

4. **Stderr and stdout are mixed** - Both streams go to same output
   - This is usually desired behavior

5. **No command history** - Cannot re-run last command easily
   - Must navigate picker again

## Performance Notes

- Command output buffering: Up to 1MB (1,048,576 bytes)
- Polling interval: 100ms (set in event loop)
- Command picker caching: Yes (per repository path)
- Concurrent execution: No (one command at a time)

## Architecture Notes

### Synchronous Event Loop with Async Execution
- Main event loop remains synchronous (uses `crossterm::event::poll`)
- Command execution spawned in background thread
- Communication via `std::sync::mpsc` channel
- Output streamed line-by-line from stdout/stderr

### State Management
- `CommandState` tracks: NotStarted | Running | Completed | Failed
- Output buffer limited to 1MB to prevent memory exhaustion
- Command picker state cached per repository to avoid re-parsing

### Error Handling
- Spawn failures reported via CommandEvent::Failed
- Read errors captured and sent to output
- Missing graft.yaml returns empty command list (no error)

## Testing Checklist Completed

- ✅ Basic command execution
- ✅ Streaming output
- ✅ Failed commands
- ✅ Multi-line output
- ✅ Output scrolling
- ✅ Command picker navigation
- ✅ Close without executing
- ✅ Repos without commands
- ✅ Repos without graft.yaml
- ✅ Exit code display
- ✅ Cache behavior (instant re-open)
- ✅ Help overlay update
