# Status Bar Improvements - Implementation Plan

## Summary of Critical Analysis

After reviewing the current status bar implementation, I've identified **14 issues** ranging from critical to nice-to-have. This plan focuses on the **highest-impact, lowest-effort improvements**.

---

## Phase 1: Critical Fixes (Recommended for Immediate Implementation)

### 1. Message Truncation with Ellipsis ⚠️ CRITICAL

**Problem:** Long messages silently truncate, losing critical information

**Example:**
```
Terminal width: 80
Message: "Error loading graft.yaml: unexpected token at line 42, column 15 in repository /home/user/projects/..."
Result: "Error loading graft.yaml: unexpected token at line 42, column 15 in repo[CUTOFF]"
```

**Solution:**
```rust
// In render_status_bar()
fn render_status_bar(&self, frame: &mut ratatui::Frame, area: Rect) {
    let (mut text, fg_color, bg_color) = if let Some((msg, msg_type, _)) = &self.status_message {
        let (fg, bg, sym) = match msg_type {
            MessageType::Error => (Color::White, Color::Red, "✗"),
            MessageType::Warning => (Color::Black, Color::Yellow, "⚠"),
            MessageType::Info => (Color::White, Color::Blue, "ℹ"),
            MessageType::Success => (Color::Black, Color::Green, "✓"),
        };
        (format!(" {} {}", sym, msg), fg, bg)
    } else {
        (
            " Ready • Press ? for help".to_string(),
            Color::White,
            Color::DarkGray,
        )
    };

    // NEW: Truncate with ellipsis if too long
    let max_width = area.width as usize;
    if text.len() > max_width {
        text.truncate(max_width.saturating_sub(3));
        text.push_str("...");
    }

    let status_bar = Paragraph::new(text)
        .style(Style::default().fg(fg_color).bg(bg_color));

    frame.render_widget(status_bar, area);
}
```

**Effort:** 5 minutes
**Impact:** Prevents information loss

---

### 2. Replace Tuple with StatusMessage Struct ⚠️ IMPORTANT

**Problem:** `Option<(String, MessageType, Instant)>` is becoming unwieldy

**Solution:**
```rust
// Add to domain types
#[derive(Debug, Clone)]
struct StatusMessage {
    text: String,
    msg_type: MessageType,
    shown_at: Instant,
}

impl StatusMessage {
    fn new(text: impl Into<String>, msg_type: MessageType) -> Self {
        Self {
            text: text.into(),
            msg_type,
            shown_at: Instant::now(),
        }
    }

    fn error(text: impl Into<String>) -> Self {
        Self::new(text, MessageType::Error)
    }

    fn warning(text: impl Into<String>) -> Self {
        Self::new(text, MessageType::Warning)
    }

    fn info(text: impl Into<String>) -> Self {
        Self::new(text, MessageType::Info)
    }

    fn success(text: impl Into<String>) -> Self {
        Self::new(text, MessageType::Success)
    }

    fn is_expired(&self) -> bool {
        self.shown_at.elapsed() > Duration::from_secs(3)
    }
}

// Update App struct
pub struct App<R, D> {
    // ...
    status_message: Option<StatusMessage>,  // Was: Option<(String, MessageType, Instant)>
    // ...
}

// Update usage
// Before:
self.status_message = Some((
    "No commands defined".to_string(),
    MessageType::Warning,
    Instant::now(),
));

// After:
self.status_message = Some(StatusMessage::warning("No commands defined"));
```

**Effort:** 30 minutes (find and replace all usages)
**Impact:** Much cleaner code, easier to extend

---

### 3. Unicode Fallback Detection ⚠️ IMPORTANT

**Problem:** Symbols may not render in all terminals

**Solution:**
```rust
// Add to top of file
fn supports_unicode() -> bool {
    std::env::var("TERM")
        .map(|term| {
            !term.contains("linux") &&
            !term.contains("ascii") &&
            !term.contains("vt100")
        })
        .unwrap_or(true)  // Default to Unicode
}

// Update MessageType
impl MessageType {
    fn symbol(&self, unicode: bool) -> &'static str {
        match (self, unicode) {
            (MessageType::Error, true) => "✗",
            (MessageType::Error, false) => "X",
            (MessageType::Warning, true) => "⚠",
            (MessageType::Warning, false) => "!",
            (MessageType::Info, true) => "ℹ",
            (MessageType::Info, false) => "i",
            (MessageType::Success, true) => "✓",
            (MessageType::Success, false) => "*",
        }
    }
}

// Update render_status_bar
let unicode = supports_unicode();
let (fg, bg, sym) = match msg_type {
    MessageType::Error => (Color::White, Color::Red, msg_type.symbol(unicode)),
    // ...
};
```

**Effort:** 15 minutes
**Impact:** Works in more terminal types

---

### 4. Basic Unit Tests ⚠️ IMPORTANT

**Solution:**
```rust
// Add to bottom of tui.rs

#[cfg(test)]
mod status_message_tests {
    use super::*;

    #[test]
    fn status_message_creates_with_timestamp() {
        let msg = StatusMessage::error("Test error");
        assert_eq!(msg.text, "Test error");
        assert_eq!(msg.msg_type, MessageType::Error);
        assert!(msg.shown_at.elapsed() < Duration::from_millis(10));
    }

    #[test]
    fn status_message_not_expired_immediately() {
        let msg = StatusMessage::info("Test");
        assert!(!msg.is_expired());
    }

    #[test]
    fn message_type_symbols_unicode() {
        assert_eq!(MessageType::Error.symbol(true), "✗");
        assert_eq!(MessageType::Warning.symbol(true), "⚠");
        assert_eq!(MessageType::Info.symbol(true), "ℹ");
        assert_eq!(MessageType::Success.symbol(true), "✓");
    }

    #[test]
    fn message_type_symbols_ascii() {
        assert_eq!(MessageType::Error.symbol(false), "X");
        assert_eq!(MessageType::Warning.symbol(false), "!");
        assert_eq!(MessageType::Info.symbol(false), "i");
        assert_eq!(MessageType::Success.symbol(false), "*");
    }

    #[test]
    fn truncate_long_message() {
        let msg = "This is a very long message that should be truncated";
        let mut truncated = msg.to_string();
        let max_width = 20;

        if truncated.len() > max_width {
            truncated.truncate(max_width.saturating_sub(3));
            truncated.push_str("...");
        }

        assert_eq!(truncated.len(), 20);
        assert!(truncated.ends_with("..."));
        assert!(truncated.starts_with("This is a very l"));
    }
}
```

**Effort:** 30 minutes
**Impact:** Prevents regressions

---

## Phase 2: Quality Improvements (Consider for Future)

### 5. Variable Auto-Dismiss Durations

**Enhancement:** Different message types should have different lifetimes

```rust
impl MessageType {
    fn default_duration(&self) -> Duration {
        match self {
            MessageType::Error => Duration::from_secs(5),    // Longer to read
            MessageType::Warning => Duration::from_secs(4),
            MessageType::Info => Duration::from_secs(2),     // Quick confirmation
            MessageType::Success => Duration::from_secs(3),
        }
    }
}

// Update StatusMessage
#[derive(Debug, Clone)]
struct StatusMessage {
    text: String,
    msg_type: MessageType,
    shown_at: Instant,
    duration: Duration,  // NEW
}

impl StatusMessage {
    fn new(text: impl Into<String>, msg_type: MessageType) -> Self {
        let duration = msg_type.default_duration();
        Self {
            text: text.into(),
            msg_type,
            shown_at: Instant::now(),
            duration,
        }
    }

    fn is_expired(&self) -> bool {
        self.shown_at.elapsed() > self.duration
    }
}
```

**Effort:** 15 minutes
**Impact:** Better UX for different message types

---

### 6. Message Queue with Priority

**Enhancement:** Don't immediately replace messages

```rust
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

#[derive(Debug, Clone)]
struct StatusMessage {
    text: String,
    msg_type: MessageType,
    priority: Priority,
    shown_at: Instant,
    duration: Duration,
}

pub struct App<R, D> {
    // ...
    status_messages: VecDeque<StatusMessage>,  // Queue of pending messages
    current_message: Option<StatusMessage>,     // Currently displayed
    // ...
}

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    fn push_status_message(&mut self, msg: StatusMessage) {
        // If current message is higher priority, queue new message
        if let Some(current) = &self.current_message {
            if current.priority > msg.priority && !current.is_expired() {
                self.status_messages.push_back(msg);
                return;
            }
        }

        // Otherwise, replace immediately (queue current if not expired)
        if let Some(current) = self.current_message.take() {
            if !current.is_expired() {
                self.status_messages.push_back(current);
            }
        }
        self.current_message = Some(msg);
    }

    fn update_status_messages(&mut self) {
        // Check if current message expired
        if let Some(current) = &self.current_message {
            if current.is_expired() {
                self.current_message = None;
            }
        }

        // If no current message, show next from queue
        if self.current_message.is_none() {
            self.current_message = self.status_messages.pop_front();
        }
    }
}
```

**Effort:** 1 hour
**Impact:** Users don't miss rapid messages

---

### 7. Accessibility: Text Prefixes

**Enhancement:** Add text labels for screen readers and clarity

```rust
impl MessageType {
    fn prefix(&self) -> &'static str {
        match self {
            MessageType::Error => "ERROR: ",
            MessageType::Warning => "WARN: ",
            MessageType::Info => "INFO: ",
            MessageType::Success => "OK: ",
        }
    }
}

// Update rendering
format!(" {} {}{}", sym, msg_type.prefix(), msg)
// Result: " ✗ ERROR: No graft.yaml found"
```

**Effort:** 10 minutes
**Impact:** Better accessibility, clearer messages

---

### 8. Smart Terminal Size Handling

**Enhancement:** Warn users about small terminals

```rust
fn render_status_bar(&self, frame: &mut ratatui::Frame, area: Rect) {
    // Check if terminal is too small
    if area.width < 60 {
        let warning = Paragraph::new(" Terminal too small (min 60 cols)")
            .style(Style::default().fg(Color::White).bg(Color::Red));
        frame.render_widget(warning, area);
        return;
    }

    // Normal rendering...
}
```

**Effort:** 10 minutes
**Impact:** Better UX on small terminals

---

## Phase 3: Future Enhancements (Defer)

These are valuable but lower priority:

### 9. Message History Log
- Press 'L' to view recent messages
- Effort: 2 hours
- Defer until users request it

### 10. Actionable Messages
- `⚠ No commands • Press 'e' to edit graft.yaml`
- Effort: 3 hours
- Defer until clear use cases emerge

### 11. Theme System
- Customizable colors
- Effort: 2 hours
- Defer until users request it

### 12. Progress Indicators
- `ℹ Refreshing... [████░░] 60%`
- Effort: 4 hours
- Defer until needed

---

## Implementation Timeline

### Immediate (30 minutes)
1. ✅ Message truncation (5 min)
2. ✅ Unicode fallback (15 min)
3. ✅ Basic tests (30 min)

**Total: 50 minutes**

### Soon (1 hour)
4. ✅ StatusMessage struct refactor (30 min)
5. ✅ Variable durations (15 min)
6. ✅ Text prefixes (10 min)

**Total: 55 minutes**

### Later (if needed)
7. Message queue (1 hour)
8. Terminal size handling (10 min)

**Total: 1 hour 10 minutes**

---

## Testing Plan

After each phase:

1. **Manual Testing**
   ```bash
   cd /tmp/grove-test
   grove --workspace workspace.yaml

   # Test each message type
   - Press 'x' on repo without commands → Warning
   - Press 'r' → Info then Success
   - Break graft.yaml → Error
   ```

2. **Automated Testing**
   ```bash
   cargo test status_message
   ```

3. **Terminal Compatibility**
   ```bash
   # Test in different terminals
   TERM=linux grove           # ASCII fallback
   TERM=xterm-256color grove  # Unicode
   ```

4. **Edge Cases**
   ```bash
   # Small terminal
   resize -s 24 40
   grove

   # Very long message
   # (Manually trigger with long path)
   ```

---

## Success Criteria

### Phase 1 Complete When:
- [ ] Long messages show ellipsis (no silent truncation)
- [ ] Code uses StatusMessage struct (no tuples)
- [ ] Works in TERM=linux (ASCII symbols)
- [ ] 5+ unit tests passing

### Phase 2 Complete When:
- [ ] Errors stay visible longer than info messages
- [ ] Messages include text prefix (ERROR:, WARN:, etc.)
- [ ] Works in 60-column terminal

### Overall Success:
- [ ] All original functionality preserved
- [ ] No test regressions
- [ ] Cleaner, more maintainable code
- [ ] Better accessibility
- [ ] Better UX for edge cases

---

## Estimated Total Effort

- **Phase 1 (Critical):** 50 minutes
- **Phase 2 (Quality):** 1-2 hours
- **Phase 3 (Future):** 8+ hours

**Recommended investment:** Phase 1 + Phase 2 = ~2 hours total

**Return on investment:**
- Much better code quality
- Handles edge cases
- More accessible
- Easier to extend later

---

## Open Questions for Discussion

1. **Message Queue:** Is replacement of messages actually a problem? Have users complained?

2. **Auto-Dismiss:** Should critical errors require manual dismiss? Or is auto-dismiss always OK?

3. **Default State:** Should status bar show "Ready" or be empty when no messages?

4. **Text Prefixes:** Does "ERROR: " add clarity or just noise?

5. **Phase 2 Priority:** Which Phase 2 improvements are most valuable?

---

## Recommendation

**Implement Phase 1 immediately** (50 minutes):
- Fixes critical bugs (truncation)
- Improves code quality (struct refactor)
- Adds safety net (tests)
- Better compatibility (Unicode fallback)

**Defer Phase 2** until after user feedback:
- Wait to see if message replacement is actually problematic
- See if users request longer error visibility
- Gather data on terminal sizes used

**Defer Phase 3** indefinitely:
- Implement only if users specifically request
- Nice-to-have features, not essential

This approach balances **immediate quality improvements** with **avoiding over-engineering**.
