//! Command line input handler for the TUI.
//!
//! Provides text input with cursor movement and command history.

/// Command history with navigation
pub struct History {
    entries: Vec<String>,
    position: Option<usize>,
    max_entries: usize,
}

impl History {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            position: None,
            max_entries,
        }
    }

    pub fn add(&mut self, entry: String) {
        if entry.is_empty() {
            return;
        }
        // Don't add duplicates of the last entry
        if self.entries.last() == Some(&entry) {
            return;
        }
        self.entries.push(entry);
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
        self.position = None;
    }

    pub fn prev(&mut self) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }
        let new_pos = match self.position {
            None => self.entries.len().saturating_sub(1),
            Some(0) => 0,
            Some(p) => p - 1,
        };
        self.position = Some(new_pos);
        self.entries.get(new_pos).map(|s| s.as_str())
    }

    pub fn next(&mut self) -> Option<&str> {
        match self.position {
            None => None,
            Some(p) => {
                if p + 1 >= self.entries.len() {
                    self.position = None;
                    None
                } else {
                    self.position = Some(p + 1);
                    self.entries.get(p + 1).map(|s| s.as_str())
                }
            }
        }
    }

    pub fn reset_position(&mut self) {
        self.position = None;
    }
}

/// Text input line with cursor and history
pub struct InputLine {
    buffer: String,
    cursor: usize,
    history: History,
    /// Saved current input when navigating history
    saved_input: Option<String>,
}

impl InputLine {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            cursor: 0,
            history: History::new(100),
            saved_input: None,
        }
    }

    pub fn value(&self) -> &str {
        &self.buffer
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn insert(&mut self, c: char) {
        self.buffer.insert(self.cursor, c);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.buffer.remove(self.cursor);
        }
    }

    pub fn delete(&mut self) {
        if self.cursor < self.buffer.len() {
            self.buffer.remove(self.cursor);
        }
    }

    pub fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.buffer.len() {
            self.cursor += 1;
        }
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.buffer.len();
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
        self.history.reset_position();
        self.saved_input = None;
    }

    pub fn submit(&mut self) -> String {
        let line = std::mem::take(&mut self.buffer);
        self.cursor = 0;
        self.history.add(line.clone());
        self.saved_input = None;
        line
    }

    pub fn history_prev(&mut self) {
        // Save current input if starting history navigation
        if self.saved_input.is_none() && !self.buffer.is_empty() {
            self.saved_input = Some(self.buffer.clone());
        }
        
        if let Some(entry) = self.history.prev() {
            self.buffer = entry.to_string();
            self.cursor = self.buffer.len();
        }
    }

    pub fn history_next(&mut self) {
        match self.history.next() {
            Some(entry) => {
                self.buffer = entry.to_string();
                self.cursor = self.buffer.len();
            }
            None => {
                // Restore saved input or clear
                if let Some(saved) = self.saved_input.take() {
                    self.buffer = saved;
                } else {
                    self.buffer.clear();
                }
                self.cursor = self.buffer.len();
            }
        }
    }

    /// Set the buffer content (for tab completion)
    pub fn set(&mut self, content: &str) {
        self.buffer = content.to_string();
        self.cursor = self.buffer.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_basic_typing() {
        let mut input = InputLine::new();
        input.insert('h');
        input.insert('i');
        assert_eq!(input.value(), "hi");
        assert_eq!(input.cursor(), 2);
    }

    #[test]
    fn input_backspace() {
        let mut input = InputLine::new();
        input.insert('a');
        input.insert('b');
        input.insert('c');
        input.backspace();
        assert_eq!(input.value(), "ab");
        assert_eq!(input.cursor(), 2);
    }

    #[test]
    fn input_cursor_movement() {
        let mut input = InputLine::new();
        input.insert('a');
        input.insert('b');
        input.insert('c');
        input.move_left();
        input.move_left();
        assert_eq!(input.cursor(), 1);
        input.insert('X');
        assert_eq!(input.value(), "aXbc");
    }

    #[test]
    fn history_navigation() {
        let mut history = History::new(10);
        history.add("first".to_string());
        history.add("second".to_string());
        history.add("third".to_string());
        
        assert_eq!(history.prev(), Some("third"));
        assert_eq!(history.prev(), Some("second"));
        assert_eq!(history.prev(), Some("first"));
        assert_eq!(history.prev(), Some("first")); // stays at beginning
        
        assert_eq!(history.next(), Some("second"));
        assert_eq!(history.next(), Some("third"));
        assert_eq!(history.next(), None); // back to current
    }

    #[test]
    fn input_submit_adds_to_history() {
        let mut input = InputLine::new();
        input.insert('g');
        input.insert('o');
        let submitted = input.submit();
        assert_eq!(submitted, "go");
        assert_eq!(input.value(), "");
        
        // Can recall from history
        input.history_prev();
        assert_eq!(input.value(), "go");
    }
}


