//! Indent-independent movement from the resting cursor to a target offset.

use super::{Event, KeyName};
use crate::error::RenderError;

/// A (line, column) position in a buffer.
struct Pos {
    /// 0-based line index.
    line: usize,
    /// 0-based column within the line.
    col: usize,
}

/// Map a character offset to its (line, column) over the split buffer.
fn pos(lines: &[Vec<char>], off: usize) -> Pos {
    let mut col = off;
    for (i, line) in lines.iter().enumerate() {
        if col <= line.len() {
            return Pos { line: i, col };
        }
        col -= line.len() + 1;
    }
    let last = lines.len().saturating_sub(1);
    Pos {
        line: last,
        col: lines.get(last).map_or(0, Vec::len),
    }
}

/// Movement from `from` to `to` as key events: cross lines with `Up`×N then
/// `End`, then `Left` to the column; a same-line move goes straight `Left`.
///
/// # Errors
/// Returns [`RenderError`] if `to` is below or to the right of `from` (the
/// cursor only ever walks back toward `%0`, so this never fires in practice).
pub fn movement_events(buffer: &str, from: usize, to: usize) -> Result<Vec<Event>, RenderError> {
    let lines: Vec<Vec<char>> = buffer.split('\n').map(|l| l.chars().collect()).collect();
    let f = pos(&lines, from);
    let t = pos(&lines, to);
    if t.line > f.line {
        return Err(RenderError::new(
            "movement target is below the resting cursor",
        ));
    }
    if f.line == t.line {
        same_line(&f, &t)
    } else {
        Ok(cross_line(&lines, &f, &t))
    }
}

/// Same-line move: straight `Left` from the resting column.
fn same_line(f: &Pos, t: &Pos) -> Result<Vec<Event>, RenderError> {
    if t.col > f.col {
        return Err(RenderError::new(
            "movement target is right of the resting cursor",
        ));
    }
    let mut out = Vec::new();
    push_left(&mut out, f.col - t.col);
    Ok(out)
}

/// Cross-line move: `Up`×N, `End`, then `Left` to the target column.
fn cross_line(lines: &[Vec<char>], f: &Pos, t: &Pos) -> Vec<Event> {
    let mut out = vec![
        Event::Key {
            key: KeyName::Up,
            n: f.line - t.line,
        },
        Event::Key {
            key: KeyName::End,
            n: 1,
        },
    ];
    let line_len = lines.get(t.line).map_or(0, Vec::len);
    push_left(&mut out, line_len - t.col);
    out
}

/// Append a `Left`×n event when `n` is nonzero.
fn push_left(out: &mut Vec<Event>, n: usize) {
    if n > 0 {
        out.push(Event::Key {
            key: KeyName::Left,
            n,
        });
    }
}
