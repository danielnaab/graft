# Testing the Grove Status Bar

## Quick Test

```bash
cd /tmp/grove-test
grove --workspace workspace.yaml
```

## Status Bar Test Scenarios

### 1. Default State
**What:** Launch Grove
**Expected:** Bottom line shows:
```
 Ready • Press ? for help
```
**Color:** White text on dark gray background

---

### 2. Info Message (Blue)
**What:** Press `r` to refresh
**Expected:** Bottom line immediately shows:
```
 ℹ Refreshing...
```
**Color:** White text on blue background
**Duration:** Shows until refresh completes (~1 second)

---

### 3. Success Message (Green)
**What:** Wait for refresh to complete
**Expected:** Bottom line shows:
```
 ✓ Refreshed 1 repositories
```
**Color:** Black text on green background
**Duration:** Auto-dismisses after 3 seconds, returns to "Ready"

---

### 4. Warning Message (Yellow) - THE FIX FOR YOUR ISSUE
**What:**
1. Create a repo without commands:
   ```bash
   cd /tmp/grove-test
   mkdir repo-no-commands
   cd repo-no-commands
   git init
   echo "name: test" > graft.yaml
   git add . && git commit -m "Init"

   cd ..
   cat >> workspace.yaml <<'EOF'
   - path: ./repo-no-commands
     tags: []
   EOF
   ```
2. Restart Grove
3. Select `repo-no-commands`
4. Press `x`

**Expected:** Bottom line shows:
```
 ⚠ No commands defined in graft.yaml
```
**Color:** Black text on yellow background (VERY VISIBLE!)
**Duration:** Auto-dismisses after 3 seconds

**BEFORE:** Message was subtle, in title bar, easy to miss
**AFTER:** Impossible to miss! Yellow bar at bottom with warning symbol

---

### 5. Error Message (Red)
**What:**
1. Create invalid graft.yaml:
   ```bash
   cd /tmp/grove-test/repo1
   echo "invalid: yaml: structure: [" > graft.yaml
   ```
2. Restart Grove, select repo1
3. Press `x`

**Expected:** Bottom line shows:
```
 ✗ Error loading graft.yaml: <error details>
```
**Color:** White text on red background
**Duration:** Auto-dismisses after 3 seconds

**Cleanup:**
```bash
cd /tmp/grove-test/repo1
git checkout graft.yaml
```

---

### 6. Auto-Dismiss Behavior
**What:** Trigger any message (e.g., press `r`)
**Expected:**
1. Message appears immediately
2. Remains visible for 3 seconds
3. Automatically clears to "Ready" state
4. No user action required

---

### 7. Status Bar Visibility
**What:** Resize terminal window (make it very small)
**Expected:**
- Status bar always visible at bottom
- Takes exactly 1 line
- Doesn't overlap content
- Content area shrinks but status bar remains

---

### 8. Multiple Message Types
**What:** Trigger different messages in sequence
**Expected:**
1. Press `r` → Blue "Refreshing..."
2. Wait for completion → Green "Refreshed..."
3. Wait 3 sec → "Ready"
4. Press `x` on repo without commands → Yellow warning
5. Wait 3 sec → "Ready"
6. Press `?` for help → See status bar documentation

---

## Visual Comparison

### Before (Subtle)
```
┌─ Grove: my-workspace - No commands defined in graft.yaml ─┐
│ ▶ repo1                                                    │
│   repo2                                                    │
└────────────────────────────────────────────────────────────┘
```
- Message buried in title
- No color emphasis
- Easy to miss

### After (Visible!)
```
┌─ Grove: my-workspace (↑↓/jk navigate, x:commands, ?:help) ┐
│ ▶ repo1                                                    │
│   repo2                                                    │
└────────────────────────────────────────────────────────────┘
 ⚠ No commands defined in graft.yaml
 ▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲
 Yellow background - Impossible to miss!
```

---

## Keyboard Test Flow

Quick test sequence to see all states:

1. Launch: `grove` → See "Ready"
2. Press `r` → See blue "Refreshing..."
3. Wait → See green "Refreshed X repositories"
4. Wait 3s → Back to "Ready"
5. Press `x` (on repo without commands) → See yellow warning
6. Wait 3s → Back to "Ready"
7. Press `?` → Help shows status bar legend

---

## Benefits Demonstrated

✅ **Visibility** - Status bar is always at the same location
✅ **Attention** - Yellow/red backgrounds grab attention
✅ **Clarity** - Symbols (⚠, ✗, ✓, ℹ) indicate type at a glance
✅ **Non-intrusive** - Auto-dismisses, doesn't require action
✅ **Conventional** - Follows vim/htop patterns (familiar)
✅ **Accessible** - Color + symbol (not color alone)

---

## Success Criteria

- [ ] Default "Ready" state is clear
- [ ] Warning for "No commands" is impossible to miss
- [ ] All message types use correct colors
- [ ] Symbols render properly (Unicode)
- [ ] Messages auto-dismiss after 3 seconds
- [ ] Status bar doesn't interfere with content
- [ ] Resizing terminal doesn't break layout

---

## Next: Try It Yourself!

The status bar solves your original issue:

> "the status bar that says 'no commands defined in <..>' is kind of hidden"

**Now:** It's a bright yellow bar at the bottom with a warning symbol. Impossible to miss! ⚠️
