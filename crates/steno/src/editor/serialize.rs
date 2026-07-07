//! Serialize a keystroke stream to a Plover value string, and the text escaper
//! it uses.

use super::{Event, KeyName};

/// Escape a run of spaces, returning the index just past the run.
fn escape_space_run(chars: &[char], start: usize, out: &mut String) -> usize {
    let mut j = start;
    while chars.get(j) == Some(&' ') {
        j += 1;
    }
    let run = j - start;
    let before = if start > 0 {
        chars.get(start - 1).copied()
    } else {
        None
    };
    let after = chars.get(j).copied();
    // A lone internal space is safe only with a real char on both sides;
    // touching a boundary/newline is a space Plover eats.
    let safe =
        run == 1 && matches!(before, Some(c) if c != '\n') && matches!(after, Some(c) if c != '\n');
    if safe {
        out.push(' ');
    } else {
        for _ in 0..run {
            out.push_str("{^ ^}");
        }
    }
    j
}

/// Append one non-space character, escaping the Plover-significant ones.
fn push_escaped_char(ch: char, out: &mut String) {
    match ch {
        '{' => out.push_str("\\{"),
        '}' => out.push_str("\\}"),
        '\n' => out.push_str("\\n"),
        '\t' => out.push_str("\\t"),
        _ => out.push(ch),
    }
}

/// Escape literal text for a Plover value (spaces at boundaries need `{^ ^}`).
#[must_use]
pub fn escape_text(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while let Some(&ch) = chars.get(i) {
        if ch == ' ' {
            i = escape_space_run(&chars, i, &mut out);
            continue;
        }
        push_escaped_char(ch, &mut out);
        i += 1;
    }
    out
}

/// Accumulator that batches typed text and `{#...}` key groups.
#[derive(Default)]
struct Serializer {
    /// Finished output.
    out: String,
    /// Pending typed characters (with real `\n` for Enter).
    text: String,
    /// Pending `{#...}` key names.
    group: Vec<&'static str>,
}

impl Serializer {
    /// Flush the pending text span (escaped as a whole).
    fn flush_text(&mut self) {
        if !self.text.is_empty() {
            self.out.push_str(&escape_text(&self.text));
            self.text.clear();
        }
    }

    /// Flush the pending key group as a `{#...}` token.
    fn flush_group(&mut self) {
        if !self.group.is_empty() {
            self.out.push_str("{#");
            self.out.push_str(&self.group.join(" "));
            self.out.push('}');
            self.group.clear();
        }
    }

    /// Fold one event into the accumulator.
    fn push_event(&mut self, ev: &Event) {
        match ev {
            Event::Mark(_) => {},
            Event::Text(s) => {
                self.flush_group();
                self.text.push_str(s);
            },
            Event::Key {
                key: KeyName::Enter,
                n,
            } => {
                self.flush_group();
                for _ in 0..*n {
                    self.text.push('\n');
                }
            },
            Event::Key { key, n } => {
                self.flush_text();
                for _ in 0..*n {
                    self.group.push(key.name());
                }
            },
        }
    }

    /// Emit the final string.
    fn finish(mut self) -> String {
        self.flush_text();
        self.flush_group();
        self.out
    }
}

/// Serialize a keystroke stream to a Plover value.
///
/// No surrounding `{^}` affixes. Typed characters and newlines accumulate into
/// one escaped span; marks are invisible; navigation keys emit a `{#...}` group
/// and break the span.
#[must_use]
pub fn serialize(events: &[Event]) -> String {
    let mut ser = Serializer::default();
    for ev in events {
        ser.push_event(ev);
    }
    ser.finish()
}
