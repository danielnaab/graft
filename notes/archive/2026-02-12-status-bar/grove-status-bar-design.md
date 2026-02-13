# Grove Status Bar Design

## Overview

Implemented a **dedicated bottom status bar** following TUI best practices (vim, htop, lazygit).

## Visual Design

```
┌─────────────────────────────────────────────────────────────┐
│ Grove: my-workspace (↑↓/jk navigate, x:commands, ?:help)   │
│ ┌──────────────────┬───────────────────────────────────────┐│
│ │ Repo List        │ Detail Pane                           ││
│ │                  │                                       ││
│ │ ▶ repo1          │ main ○ ↑2                             ││
│ │   repo2          │                                       ││
│ │   repo3          │ Changed Files (3)                     ││
│ └──────────────────┴───────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
 ⚠ No commands defined in graft.yaml                           ← STATUS BAR
```

## Status Bar States

### 1. Default (No Message)
```
 Ready • Press ? for help
```
- **Colors:** White text on dark gray background
- **Purpose:** Shows ready state, hints at help

### 2. Info Messages
```
 ℹ Refreshing...
```
- **Colors:** White text on blue background
- **Symbol:** ℹ (info icon)
- **Used for:** Progress indicators, informational messages
- **Auto-dismiss:** 3 seconds

### 3. Success Messages
```
 ✓ Refreshed 5 repositories
```
- **Colors:** Black text on green background
- **Symbol:** ✓ (checkmark)
- **Used for:** Successful operations
- **Auto-dismiss:** 3 seconds

### 4. Warning Messages
```
 ⚠ No commands defined in graft.yaml
```
- **Colors:** Black text on yellow background
- **Symbol:** ⚠ (warning triangle)
- **Used for:** Non-critical issues, user attention needed
- **Auto-dismiss:** 3 seconds

### 5. Error Messages
```
 ✗ Error loading graft.yaml: file not found
```
- **Colors:** White text on red background
- **Symbol:** ✗ (X mark)
- **Used for:** Errors, failed operations
- **Auto-dismiss:** 3 seconds

## Message Type Usage

| MessageType | When to Use | Example |
|-------------|-------------|---------|
| `Info` | Progress, neutral information | "Refreshing...", "Loading..." |
| `Success` | Successful completion | "Refreshed 5 repositories" |
| `Warning` | User attention needed (non-critical) | "No commands defined", "Partial success" |
| `Error` | Operation failed | "Refresh failed", "Error loading file" |

## Benefits Over Previous Design

### Before (Subtle Title Bar)
```
Grove: my-workspace - No commands defined in graft.yaml
```
- ❌ Easy to miss (blends with title)
- ❌ No color coding
- ❌ No visual emphasis
- ❌ Competes with workspace name

### After (Dedicated Status Bar)
```
Grove: my-workspace (↑↓/jk navigate, x:commands, ?:help)
...
 ⚠ No commands defined in graft.yaml
```
- ✅ Impossible to miss (dedicated area)
- ✅ Color-coded by type
- ✅ Visual symbols for quick recognition
- ✅ Consistent location (bottom of screen)
- ✅ Follows TUI conventions

## Implementation Details

### Code Structure

```rust
enum MessageType {
    Error,    // Red background, white text, ✗
    Warning,  // Yellow background, black text, ⚠
    Info,     // Blue background, white text, ℹ
    Success,  // Green background, black text, ✓
}

// Status message format
status_message: Option<(String, MessageType, Instant)>
                       ^^^^^^  ^^^^^^^^^^^  ^^^^^^^
                       text    type         timestamp
```

### Layout

```rust
// Main vertical split
Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Min(3),      // Content (flexible, fills space)
        Constraint::Length(1),   // Status bar (fixed, 1 line)
    ])
```

### Setting a Status Message

```rust
// Info
self.status_message = Some((
    "Refreshing...".to_string(),
    MessageType::Info,
    Instant::now(),
));

// Warning
self.status_message = Some((
    "No commands defined in graft.yaml".to_string(),
    MessageType::Warning,
    Instant::now(),
));

// Error
self.status_message = Some((
    format!("Error: {}", e),
    MessageType::Error,
    Instant::now(),
));

// Success
self.status_message = Some((
    "Operation completed successfully".to_string(),
    MessageType::Success,
    Instant::now(),
));
```

### Auto-Dismiss

Messages automatically clear after 3 seconds:

```rust
fn clear_expired_status_message(&mut self) {
    if let Some((_, _, set_at)) = &self.status_message {
        if set_at.elapsed() > Duration::from_secs(3) {
            self.status_message = None;
        }
    }
}
```

## Future Enhancements

### 1. Persistent Status
Add a separate field for persistent status (doesn't auto-dismiss):
```rust
persistent_status: Option<String>  // e.g., "5 repos dirty"
```

### 2. Multi-Line Status Bar
Expand to 2-3 lines for more complex status:
```
 Ready • 5/10 repos dirty • Last refresh: 2m ago
 Press r to refresh • x for commands • ? for help
```

### 3. Right-Aligned Info
Split status bar into left (messages) and right (metadata):
```
 ⚠ No commands defined                    Ready • 14:32
```

### 4. Progress Indicators
For long operations, show progress:
```
 ℹ Refreshing... [████████░░] 80%
```

### 5. Action Hints
Show contextual hints based on current pane:
```
 Ready • Repo list: x=commands, Enter=details, r=refresh
```

## Design Principles

1. **Visibility** - Always visible, consistent location
2. **Clarity** - Color + symbol makes type obvious
3. **Non-intrusive** - Single line at bottom, auto-dismisses
4. **Conventional** - Follows patterns from vim, htop, lazygit
5. **Extensible** - Easy to add new message types or expand functionality

## Accessibility

- **Color + Symbol** - Not relying on color alone (symbol provides redundancy)
- **High Contrast** - Background colors chosen for readability
- **Consistent Position** - Always bottom line, muscle memory

## Testing

### Visual Test Checklist
- [ ] Default state shows "Ready" in dark gray
- [ ] Info messages show blue with ℹ
- [ ] Success messages show green with ✓
- [ ] Warning messages show yellow with ⚠
- [ ] Error messages show red with ✗
- [ ] Messages auto-dismiss after 3 seconds
- [ ] Status bar doesn't overlap content
- [ ] Symbols render correctly (Unicode support)

### Test Scenarios
1. Press 'r' → See blue "Refreshing..."
2. Complete refresh → See green "Refreshed N repositories"
3. Press 'x' on repo with no commands → See yellow warning
4. Load invalid graft.yaml → See red error
5. Wait 3 seconds → Status clears to "Ready"

## Related TUI Patterns

This design can be extended for:
- **Loading spinners** - Animated progress in status bar
- **Notifications** - Toast-style messages that slide up
- **Command palette** - Bottom input bar for commands
- **Search bar** - Bottom input for search queries
- **Mode indicators** - Current mode (like vim's INSERT/NORMAL)

## Conclusion

The new status bar provides:
- **Clear visual feedback** that's impossible to miss
- **Consistent UX** following TUI best practices
- **Extensible design** for future enhancements
- **Better accessibility** with color + symbols

This addresses the original issue where "No commands defined" was too subtle, and provides a foundation for future status/notification improvements.
