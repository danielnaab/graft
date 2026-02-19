//! Shared text buffer with cursor and editing operations.
//!
//! Used by both `CommandLineState` and `ArgumentInputState` to provide
//! consistent text editing: insert, delete, cursor movement, and
//! shortcuts like Ctrl+U (clear) and Ctrl+W (delete word backward).

/// A single-line text buffer with a cursor position.
#[derive(Debug, Clone)]
pub(super) struct TextBuffer {
    pub buffer: String,
    pub cursor_pos: usize, // Character position (not byte position)
}

impl TextBuffer {
    pub(super) fn new() -> Self {
        Self {
            buffer: String::new(),
            cursor_pos: 0,
        }
    }

    /// Create a `TextBuffer` with initial content and a specific cursor position.
    #[cfg(test)]
    pub(super) fn with_content(s: &str, cursor_pos: usize) -> Self {
        Self {
            buffer: s.to_string(),
            cursor_pos,
        }
    }

    /// Insert a character at the cursor position.
    pub(super) fn insert_char(&mut self, c: char) {
        let mut chars: Vec<char> = self.buffer.chars().collect();
        chars.insert(self.cursor_pos, c);
        self.buffer = chars.into_iter().collect();
        self.cursor_pos += 1;
    }

    /// Delete the character before the cursor (Backspace).
    pub(super) fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            let mut chars: Vec<char> = self.buffer.chars().collect();
            chars.remove(self.cursor_pos - 1);
            self.buffer = chars.into_iter().collect();
            self.cursor_pos -= 1;
        }
    }

    /// Delete the character at the cursor (Delete key).
    pub(super) fn delete_forward(&mut self) {
        let chars: Vec<char> = self.buffer.chars().collect();
        if self.cursor_pos < chars.len() {
            let mut chars = chars;
            chars.remove(self.cursor_pos);
            self.buffer = chars.into_iter().collect();
        }
    }

    /// Clear the entire buffer (Ctrl+U).
    pub(super) fn clear(&mut self) {
        self.buffer.clear();
        self.cursor_pos = 0;
    }

    /// Delete the word before the cursor (Ctrl+W).
    ///
    /// Deletes backward from the cursor, first skipping any whitespace,
    /// then deleting until the next whitespace or start of buffer.
    pub(super) fn delete_word_backward(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }

        let chars: Vec<char> = self.buffer.chars().collect();
        let mut pos = self.cursor_pos;

        // Skip trailing whitespace
        while pos > 0 && chars[pos - 1].is_whitespace() {
            pos -= 1;
        }

        // Delete word characters
        while pos > 0 && !chars[pos - 1].is_whitespace() {
            pos -= 1;
        }

        let new_buffer: String = chars[..pos]
            .iter()
            .chain(chars[self.cursor_pos..].iter())
            .collect();
        self.buffer = new_buffer;
        self.cursor_pos = pos;
    }

    /// Move cursor one character left.
    pub(super) fn move_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    /// Move cursor one character right.
    pub(super) fn move_right(&mut self) {
        let char_count = self.buffer.chars().count();
        if self.cursor_pos < char_count {
            self.cursor_pos += 1;
        }
    }

    /// Move cursor to the beginning of the buffer (Home).
    pub(super) fn move_home(&mut self) {
        self.cursor_pos = 0;
    }

    /// Move cursor to the end of the buffer (End).
    pub(super) fn move_end(&mut self) {
        self.cursor_pos = self.buffer.chars().count();
    }

    /// Set the buffer content and place cursor at the end.
    pub(super) fn set(&mut self, s: &str) {
        self.buffer = s.to_string();
        self.cursor_pos = self.buffer.chars().count();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_char() {
        let mut buf = TextBuffer::new();
        buf.insert_char('h');
        buf.insert_char('i');
        assert_eq!(buf.buffer, "hi");
        assert_eq!(buf.cursor_pos, 2);
    }

    #[test]
    fn insert_char_mid_buffer() {
        let mut buf = TextBuffer::new();
        buf.insert_char('a');
        buf.insert_char('c');
        buf.cursor_pos = 1;
        buf.insert_char('b');
        assert_eq!(buf.buffer, "abc");
        assert_eq!(buf.cursor_pos, 2);
    }

    #[test]
    fn backspace() {
        let mut buf = TextBuffer::new();
        buf.set("hello");
        buf.backspace();
        assert_eq!(buf.buffer, "hell");
        assert_eq!(buf.cursor_pos, 4);
    }

    #[test]
    fn backspace_at_start() {
        let mut buf = TextBuffer::new();
        buf.backspace();
        assert_eq!(buf.buffer, "");
        assert_eq!(buf.cursor_pos, 0);
    }

    #[test]
    fn delete_forward() {
        let mut buf = TextBuffer::new();
        buf.set("hello");
        buf.cursor_pos = 2;
        buf.delete_forward();
        assert_eq!(buf.buffer, "helo");
        assert_eq!(buf.cursor_pos, 2);
    }

    #[test]
    fn delete_forward_at_end() {
        let mut buf = TextBuffer::new();
        buf.set("hello");
        buf.delete_forward();
        assert_eq!(buf.buffer, "hello");
        assert_eq!(buf.cursor_pos, 5);
    }

    #[test]
    fn clear_line() {
        let mut buf = TextBuffer::new();
        buf.set("hello world");
        buf.clear();
        assert_eq!(buf.buffer, "");
        assert_eq!(buf.cursor_pos, 0);
    }

    #[test]
    fn delete_word_backward() {
        let mut buf = TextBuffer::new();
        buf.set("hello world");
        buf.delete_word_backward();
        assert_eq!(buf.buffer, "hello ");
        assert_eq!(buf.cursor_pos, 6);
    }

    #[test]
    fn delete_word_backward_multiple_spaces() {
        let mut buf = TextBuffer::new();
        buf.set("hello   world");
        buf.cursor_pos = 8; // at 'w' — delete trailing spaces and preceding word
        buf.delete_word_backward();
        assert_eq!(buf.buffer, "world");
        assert_eq!(buf.cursor_pos, 0);
    }

    #[test]
    fn delete_word_backward_at_start() {
        let mut buf = TextBuffer::new();
        buf.set("hello");
        buf.cursor_pos = 0;
        buf.delete_word_backward();
        assert_eq!(buf.buffer, "hello");
        assert_eq!(buf.cursor_pos, 0);
    }

    #[test]
    fn delete_word_backward_single_word() {
        let mut buf = TextBuffer::new();
        buf.set("hello");
        buf.delete_word_backward();
        assert_eq!(buf.buffer, "");
        assert_eq!(buf.cursor_pos, 0);
    }

    #[test]
    fn move_left_right() {
        let mut buf = TextBuffer::new();
        buf.set("abc");
        assert_eq!(buf.cursor_pos, 3);
        buf.move_left();
        assert_eq!(buf.cursor_pos, 2);
        buf.move_right();
        assert_eq!(buf.cursor_pos, 3);
    }

    #[test]
    fn move_left_at_start() {
        let mut buf = TextBuffer::new();
        buf.set("abc");
        buf.cursor_pos = 0;
        buf.move_left();
        assert_eq!(buf.cursor_pos, 0);
    }

    #[test]
    fn move_right_at_end() {
        let mut buf = TextBuffer::new();
        buf.set("abc");
        buf.move_right();
        assert_eq!(buf.cursor_pos, 3);
    }

    #[test]
    fn home_end() {
        let mut buf = TextBuffer::new();
        buf.set("hello");
        buf.move_home();
        assert_eq!(buf.cursor_pos, 0);
        buf.move_end();
        assert_eq!(buf.cursor_pos, 5);
    }
}
