//! The editor simulation: the mutable [`Sim`] state and its keystroke
//! handlers. [`interpret`](super::interpret) drives it.

use super::{Behaviors, KeyName, auto_close_of, ends_with_opener, is_auto_close, is_quote};

/// Mutable editor state threaded through the keystroke handlers.
pub(super) struct Sim {
    /// Buffer as characters (code templates are effectively ASCII).
    pub(super) buffer: Vec<char>,
    /// Cursor offset in characters.
    pub(super) cursor: usize,
    /// Active profile.
    behaviors: Behaviors,
}

impl Sim {
    /// A fresh, empty editor under the given profile.
    pub(super) const fn new(behaviors: Behaviors) -> Self {
        Self {
            buffer: Vec::new(),
            cursor: 0,
            behaviors,
        }
    }

    /// Insert `s` at the cursor and advance past it.
    fn insert(&mut self, s: &str) {
        let chars: Vec<char> = s.chars().collect();
        let n = chars.len();
        self.buffer.splice(self.cursor..self.cursor, chars);
        self.cursor += n;
    }

    /// Offset of the start of the line containing `off`.
    fn line_start(&self, off: usize) -> usize {
        if off == 0 {
            return 0;
        }
        let mut i = off - 1;
        loop {
            if self.buffer.get(i) == Some(&'\n') {
                return i + 1;
            }
            if i == 0 {
                return 0;
            }
            i -= 1;
        }
    }

    /// Offset of the newline (or buffer end) ending the line containing `off`.
    fn line_end(&self, off: usize) -> usize {
        let mut i = off;
        while let Some(&c) = self.buffer.get(i) {
            if c == '\n' {
                return i;
            }
            i += 1;
        }
        self.buffer.len()
    }

    /// The run of spaces/tabs starting at `start`.
    fn leading_indent(&self, start: usize) -> String {
        let mut i = start;
        while matches!(self.buffer.get(i), Some(&(' ' | '\t'))) {
            i += 1;
        }
        self.buffer
            .get(start..i)
            .map(|s| s.iter().collect())
            .unwrap_or_default()
    }

    /// The chars from `start` to the cursor, right-trimmed.
    fn line_so_far(&self, start: usize) -> String {
        let s: String = self
            .buffer
            .get(start..self.cursor)
            .unwrap_or(&[])
            .iter()
            .collect();
        s.trim_end().to_owned()
    }

    /// Type a single character, applying auto-close / type-over per profile.
    pub(super) fn type_char(&mut self, ch: char) {
        let b = self.behaviors;
        if b.auto_close {
            if let Some(close) = auto_close_of(ch) {
                self.insert_pair(ch, close);
                return;
            }
            if is_quote(ch) {
                self.type_quote(ch);
                return;
            }
        }
        if b.type_over
            && (is_auto_close(ch) || is_quote(ch))
            && self.buffer.get(self.cursor) == Some(&ch)
        {
            self.cursor += 1;
            return;
        }
        self.insert(&String::from(ch));
    }

    /// Insert `open`+`close` and sit between them.
    fn insert_pair(&mut self, open: char, close: char) {
        let s: String = [open, close].into_iter().collect();
        self.insert(&s);
        self.cursor -= 1;
    }

    /// Type a quote: step over an existing auto-quote, else auto-pair.
    fn type_quote(&mut self, ch: char) {
        if self.buffer.get(self.cursor) == Some(&ch) {
            self.cursor += 1;
        } else {
            self.insert_pair(ch, ch);
        }
    }

    /// Handle Enter: bare newline, block-expand, or auto-indented newline.
    fn enter(&mut self) {
        if !self.behaviors.auto_indent {
            self.insert("\n");
            return;
        }
        if self.cursor > 0
            && self.buffer.get(self.cursor - 1) == Some(&'{')
            && self.buffer.get(self.cursor) == Some(&'}')
        {
            self.block_expand();
            return;
        }
        self.indent_newline();
    }

    /// Expand `{|}` onto open / indented-body / dedented-close lines.
    fn block_expand(&mut self) {
        let base = self.leading_indent(self.line_start(self.cursor));
        let unit = self.behaviors.indent_unit;
        self.insert(&format!("\n{base}{unit}\n{base}"));
        self.cursor -= 1 + base.chars().count();
    }

    /// Insert a newline indented to the depth implied by the current line.
    fn indent_newline(&mut self) {
        let start = self.line_start(self.cursor);
        let mut indent = self.leading_indent(start);
        if ends_with_opener(&self.line_so_far(start)) {
            indent.push_str(self.behaviors.indent_unit);
        }
        self.insert(&format!("\n{indent}"));
    }

    /// Handle Backspace, deleting a whole indent level inside leading indent.
    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let start = self.line_start(self.cursor);
        let before: String = self
            .buffer
            .get(start..self.cursor)
            .unwrap_or(&[])
            .iter()
            .collect();
        let del = self.dedent_delete(&before);
        let from = self.cursor - del;
        self.buffer.drain(from..self.cursor);
        self.cursor = from;
    }

    /// How many characters Backspace removes: a whole indent level when inside
    /// pure leading indentation, otherwise one.
    fn dedent_delete(&self, before: &str) -> usize {
        if !(self.behaviors.auto_indent && !before.is_empty() && before.chars().all(|c| c == ' ')) {
            return 1;
        }
        let unit = self.behaviors.indent_unit.chars().count().max(1);
        let len = before.chars().count();
        len - (len - 1) / unit * unit
    }

    /// Move up one line, keeping the column where possible.
    fn up(&mut self) {
        let start = self.line_start(self.cursor);
        if start == 0 {
            return;
        }
        let col = self.cursor - start;
        let prev_end = start - 1;
        let prev_start = self.line_start(prev_end);
        self.cursor = (prev_start + col).min(prev_end);
    }

    /// Move down one line, keeping the column where possible.
    fn down(&mut self) {
        let col = self.cursor - self.line_start(self.cursor);
        let next_start = self.line_end(self.cursor) + 1;
        if next_start <= self.buffer.len() {
            self.cursor = (next_start + col).min(self.line_end(next_start));
        }
    }

    /// Dispatch one special key.
    pub(super) fn apply_key(&mut self, key: KeyName) {
        match key {
            KeyName::Enter => self.enter(),
            KeyName::BackSpace => self.backspace(),
            KeyName::Left => {
                self.cursor = self.cursor.saturating_sub(1);
            },
            KeyName::Right => {
                if self.cursor < self.buffer.len() {
                    self.cursor += 1;
                }
            },
            KeyName::Home => self.cursor = self.line_start(self.cursor),
            KeyName::End => self.cursor = self.line_end(self.cursor),
            KeyName::Up => self.up(),
            KeyName::Down => self.down(),
            KeyName::Tab => self.insert(self.behaviors.indent_unit),
        }
    }
}
