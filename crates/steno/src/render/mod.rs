//! Passes C + D + E — turn a typed entry into a Plover dictionary value.
//!
//! The editor model lives in [`crate::editor`]: build a keystroke IR, interpret
//! it under a [`Behaviors`] preset, then serialize. A "profile" is just a
//! preset:
//!
//! * [`PLAIN`] — a dumb editor: every closer typed, cursor walks back from the
//!   document end.
//! * [`SMART`] — auto-close + type-over + block-expand: emit only what the
//!   editor won't supply (interior closers stay via type-over, the trailing run
//!   of auto-closers drops), movement computed against the result buffer.
//!
//! Non-terminal (type-append intermediate) strokes are identical in both:
//! strip all brackets, no newlines, no movement.

mod plain;
mod smart;

pub use plain::{build_plain_dict, render_plain};
pub use smart::{build_smart_dict, render_smart};

use crate::blocks::emit_struct;
use crate::editor::{Behaviors, Event, interpret, movement_events, serialize};
use crate::error::RenderError;
use crate::expand::TypedEntry;
use crate::json_out::OrderedMap;
use crate::parse::Chunk;

/// True for the auto-supplied closers `)` `]` `}`.
const fn is_closer(ch: char) -> bool {
    matches!(ch, ')' | ']' | '}')
}

/// True for the auto-pairing quote characters.
const fn is_quote(ch: char) -> bool {
    matches!(ch, '`' | '"' | '\'')
}

/// True for any delimiter stripped from a non-terminal partial.
const fn is_bracket(ch: char) -> bool {
    matches!(ch, '(' | ')' | '[' | ']' | '<' | '>' | '{' | '}')
}

/// Drop every bracket from `s`.
fn strip_brackets(s: &str) -> String {
    s.chars().filter(|c| !is_bracket(*c)).collect()
}

/// An intermediate keystroke token before coalescing into [`Event`]s.
enum Tok {
    /// One typed character.
    Ch(char),
    /// An Enter key.
    Enter,
    /// A landing mark `%n`.
    Mark(u32),
}

/// Turn a resolved template into keystroke tokens.
fn tokenize(template: &[Chunk], one_liner: bool) -> Result<Vec<Tok>, RenderError> {
    let mut toks = Vec::new();
    for c in template {
        match c {
            Chunk::Lit(text) => toks.extend(text.chars().map(Tok::Ch)),
            Chunk::Brace { open } => toks.push(Tok::Ch(if *open { '{' } else { '}' })),
            Chunk::BodyBreak => {
                if !one_liner {
                    toks.push(Tok::Enter);
                }
            },
            Chunk::Newline => toks.push(Tok::Enter),
            Chunk::Tab => toks.push(Tok::Ch('\t')),
            Chunk::Landing(n) => toks.push(Tok::Mark(*n)),
            other => {
                return Err(RenderError::new(format!(
                    "unresolved chunk \"{}\" reached the renderer",
                    other.kind_name()
                )));
            },
        }
    }
    Ok(toks)
}

/// Drop the trailing run of auto-closers (the editor supplies them); marks are
/// skipped, and the scan stops at the first real token.
fn drop_trailing_closers(mut toks: Vec<Tok>) -> Vec<Tok> {
    let mut i = toks.len();
    while i > 0 {
        i -= 1;
        match toks.get(i) {
            Some(Tok::Mark(_)) => {},
            Some(Tok::Ch(ch)) if is_closer(*ch) || is_quote(*ch) => {
                toks.remove(i);
            },
            _ => break,
        }
    }
    toks
}

/// Flush a pending character run into a text event.
fn flush_run(run: &mut String, events: &mut Vec<Event>) {
    if !run.is_empty() {
        events.push(Event::Text(std::mem::take(run)));
    }
}

/// Coalesce adjacent character tokens into text events.
fn coalesce(toks: &[Tok]) -> Vec<Event> {
    let mut events = Vec::new();
    let mut run = String::new();
    for tk in toks {
        match tk {
            Tok::Ch(c) => run.push(*c),
            Tok::Enter => {
                flush_run(&mut run, &mut events);
                events.push(Event::Key {
                    key: crate::editor::KeyName::Enter,
                    n: 1,
                });
            },
            Tok::Mark(n) => {
                flush_run(&mut run, &mut events);
                events.push(Event::Mark(*n));
            },
        }
    }
    flush_run(&mut run, &mut events);
    events
}

/// The content keystrokes for a terminal entry under the given behaviors.
fn emit_content(
    template: &[Chunk],
    one_liner: bool,
    b: Behaviors,
) -> Result<Vec<Event>, RenderError> {
    let mut toks = tokenize(template, one_liner)?;
    if b.auto_close {
        toks = drop_trailing_closers(toks);
    }
    Ok(coalesce(&toks))
}

/// Non-terminal: bracket-stripped partial, no newlines, no movement.
fn non_terminal_text(template: &[Chunk]) -> String {
    let mut text = String::new();
    for c in template {
        match c {
            Chunk::Lit(t) => text.push_str(&strip_brackets(t)),
            Chunk::Tab => text.push('\t'),
            _ => {},
        }
    }
    text
}

/// Content keystrokes plus the movement that lands the cursor on `%0`.
fn content_and_move(entry: &TypedEntry, b: Behaviors) -> Result<Vec<Event>, RenderError> {
    let mut events = emit_content(&entry.template, entry.one_liner, b)?;
    let state = interpret(&events, b);
    if let Some(target) = state.target {
        events.extend(movement_events(&state.buffer, state.rest, target)?);
    }
    Ok(events)
}

/// The full keystroke stream a profile types for one entry.
fn entry_events(entry: &TypedEntry, b: Behaviors) -> Result<Vec<Event>, RenderError> {
    // Smart-only: @literal blocks drive the editor structurally.
    if b.auto_close && entry.source.flags.literal() {
        return Ok(emit_struct(&entry.template));
    }
    if !entry.terminal {
        return Ok(vec![Event::Text(non_terminal_text(&entry.template))]);
    }
    content_and_move(entry, b)
}

/// Wrap a serialized keystroke stream in the Plover `{^}` affixes.
fn wrap(stroke: &str, events: &[Event]) -> (String, String) {
    (
        stroke.to_owned(),
        format!("{{^}}{}{{^}}", serialize(events)),
    )
}

/// A built dictionary and the strokes that collided with a different value.
pub struct BuildResult {
    /// Stroke → Plover value, in insertion order.
    pub dict: OrderedMap,
    /// Strokes that appeared twice with DIFFERENT values.
    pub collisions: Vec<String>,
}

/// Render many entries into a dictionary, flagging value collisions.
///
/// # Errors
/// Propagates any [`RenderError`] from rendering an individual entry.
fn build_dict(
    entries: &[TypedEntry],
    render: impl Fn(&TypedEntry) -> Result<(String, String), RenderError>,
) -> Result<BuildResult, RenderError> {
    let mut dict = OrderedMap::new();
    let mut collisions = Vec::new();
    for e in entries {
        let (key, value) = render(e)?;
        if dict.get(&key).is_some_and(|prev| prev != value) {
            collisions.push(key.clone());
        }
        dict.insert(key, value);
    }
    Ok(BuildResult { dict, collisions })
}
