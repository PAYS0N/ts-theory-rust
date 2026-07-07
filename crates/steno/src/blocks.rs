//! v2 smart emitter for pre-formatted multi-line blocks (the `@literal` data
//! structures). A smart editor auto-closes every `{`, auto-indents each line,
//! and block-expands `{`+Enter — so instead of typing literal `\t` and `}`
//! (which the editor would double / reflow), we drive it structurally:
//!
//! * type only the OPENING / content lines (every `}`-only line is
//!   auto-supplied)
//! * Enter after a line ending in `{` block-expands one level deeper
//! * Enter between siblings keeps the level
//! * to drop K levels, walk Down×K onto the outermost auto-`}`, End, Enter
//! * a line that *starts* with `}` (e.g. `} else {`) appends onto that auto-`}`
//!
//! Indentation is the editor's, so the result uses its indent unit (4 spaces),
//! not the template's tabs.

use crate::editor::{Event, KeyName};
use crate::parse::Chunk;

/// A non-blank, non-closer-only line: its tab depth and content.
struct SLine {
    /// Number of leading tabs.
    depth: usize,
    /// Content after the leading tabs.
    content: String,
}

/// Render a literal block template to its raw text (tabs, newlines, braces).
#[must_use]
pub fn struct_text(template: &[Chunk]) -> String {
    let mut out = String::new();
    for c in template {
        match c {
            Chunk::Lit(text) => out.push_str(text),
            Chunk::Brace { open } => out.push(if *open { '{' } else { '}' }),
            Chunk::Newline | Chunk::BodyBreak => out.push('\n'),
            Chunk::Tab => out.push('\t'),
            // landings/typeslots don't occur in @literal blocks
            _ => {},
        }
    }
    out
}

/// Split into (tab-depth, content) lines, dropping blanks and `}`-only lines
/// (those closers are auto-supplied by the editor).
fn struct_lines(template: &[Chunk]) -> Vec<SLine> {
    let text = struct_text(template);
    let mut lines = Vec::new();
    for raw in text.split('\n') {
        let chars: Vec<char> = raw.chars().collect();
        let mut d = 0;
        while chars.get(d) == Some(&'\t') {
            d += 1;
        }
        let content: String = chars.get(d..).unwrap_or(&[]).iter().collect();
        if content.is_empty() || content == "}" {
            continue;
        }
        lines.push(SLine { depth: d, content });
    }
    lines
}

/// Build a `Key` event.
const fn key(k: KeyName, n: usize) -> Event {
    Event::Key { key: k, n }
}

/// Emit the navigation between the previous line and `line`. The three
/// same-result branches of the TS original (descend after an opener, sibling
/// at the same level, deeper-without-opener) collapse to the trailing `Enter`.
fn nav(out: &mut Vec<Event>, line: &SLine, prev_depth: usize, prev_opens: bool, closer: bool) {
    if closer {
        // Append onto the auto-`}` that sits `prev_depth - depth` lines down.
        if prev_depth > line.depth {
            out.push(key(KeyName::Down, prev_depth - line.depth));
        }
        out.push(key(KeyName::End, 1));
    } else if !prev_opens && line.depth < prev_depth {
        // Exit: walk past the auto-`}`s, then open a sibling line.
        out.push(key(KeyName::Down, prev_depth - line.depth));
        out.push(key(KeyName::End, 1));
        out.push(key(KeyName::Enter, 1));
    } else {
        out.push(key(KeyName::Enter, 1));
    }
}

/// Emit the keystroke IR for an `@literal` block under a smart auto-indent
/// editor.
#[must_use]
pub fn emit_struct(template: &[Chunk]) -> Vec<Event> {
    let lines = struct_lines(template);
    let mut out = Vec::new();
    let mut prev_depth = 0;
    let mut prev_opens = false;
    for (i, line) in lines.iter().enumerate() {
        let closer = line.content.starts_with('}'); // e.g. `} else {`
        if i > 0 {
            nav(&mut out, line, prev_depth, prev_opens, closer);
        }
        // A `}`-prefixed line's brace is already present; type only the rest.
        let text = if closer {
            line.content.get(1..).unwrap_or("").to_owned()
        } else {
            line.content.clone()
        };
        out.push(Event::Text(text));
        prev_depth = line.depth;
        prev_opens = line.content.ends_with('{');
    }
    out
}
