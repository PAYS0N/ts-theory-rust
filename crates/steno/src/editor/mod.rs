//! The editor model, separated from rendering so each editor behavior is an
//! independent, testable knob.
//!
//! ```text
//!   interpret(events, behaviors) -> EditorState   (how an editor reacts)
//!   movement_events(buffer, from, to) -> Event[]  (indent-independent moves)
//!   serialize(events)            -> Plover value string
//! ```
//!
//! A "profile" is just a [`Behaviors`] preset: [`PLAIN`] = a dumb editor (all
//! features off), [`SMART`] = auto-close + type-over + block-expand,
//! [`SMART_INDENT`] adds auto-indent (so multi-level blocks need no literal
//! `\t` and no typed closers). Tests assert `interpret(emit(t, B), B)`
//! reproduces the intended code with the cursor on `%0`, for each `B`.

mod interpret;
mod movement;
mod serialize;
mod sim;

pub use interpret::interpret;
pub use movement::movement_events;
pub use serialize::{escape_text, serialize};

/// A special (non-text) key the editor understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyName {
    /// Cursor up one line.
    Up,
    /// Cursor down one line.
    Down,
    /// Cursor left one column.
    Left,
    /// Cursor right one column.
    Right,
    /// Cursor to line start.
    Home,
    /// Cursor to line end.
    End,
    /// Delete backward (a whole indent level inside leading indentation).
    BackSpace,
    /// Insert one indent unit.
    Tab,
    /// Newline (auto-indents / block-expands under `auto_indent`).
    Enter,
}

impl KeyName {
    /// The Plover key name, as it appears inside a `{#...}` group.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Up => "Up",
            Self::Down => "Down",
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Home => "Home",
            Self::End => "End",
            Self::BackSpace => "BackSpace",
            Self::Tab => "Tab",
            Self::Enter => "Enter",
        }
    }
}

/// One keystroke-IR event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Literal characters typed (may trigger auto-close).
    Text(String),
    /// A special key, repeated `n` times.
    Key {
        /// Which key.
        key: KeyName,
        /// Repeat count.
        n: usize,
    },
    /// Record the cursor as landing `%n` (types nothing).
    Mark(u32),
}

/// The knobs that define an editor profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Behaviors {
    /// Typing `(` `[` `{` (and quotes) inserts the matching closer.
    pub auto_close: bool,
    /// Typing a closer while the cursor sits on the matching auto-closer steps
    /// over it.
    pub type_over: bool,
    /// A new line is auto-indented to the current brace depth, and Enter
    /// between empty braces block-expands onto three lines; Backspace inside
    /// leading indentation deletes a whole `indent_unit` level.
    pub auto_indent: bool,
    /// One indentation level — VS Code's default is four spaces.
    pub indent_unit: &'static str,
}

/// One indentation level (four spaces, VS Code default).
pub const INDENT_UNIT: &str = "    ";

/// A dumb editor: no auto-close, no type-over, no auto-indent.
pub const PLAIN: Behaviors = Behaviors {
    auto_close: false,
    type_over: false,
    auto_indent: false,
    indent_unit: INDENT_UNIT,
};

/// Auto-close + type-over + block-expand, but no auto-indent.
pub const SMART: Behaviors = Behaviors {
    auto_close: true,
    type_over: true,
    auto_indent: false,
    indent_unit: INDENT_UNIT,
};

/// [`SMART`] plus auto-indent.
pub const SMART_INDENT: Behaviors = Behaviors {
    auto_close: true,
    type_over: true,
    auto_indent: true,
    indent_unit: INDENT_UNIT,
};

/// The auto-supplied closer for an opener, if any.
const fn auto_close_of(ch: char) -> Option<char> {
    match ch {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        _ => None,
    }
}

/// True for the three auto-supplied closers.
const fn is_auto_close(ch: char) -> bool {
    matches!(ch, ')' | ']' | '}')
}

/// True for the three auto-pairing quote characters.
const fn is_quote(ch: char) -> bool {
    matches!(ch, '`' | '"' | '\'')
}

/// True if `s` ends with an open bracket (so the next line indents deeper).
fn ends_with_opener(s: &str) -> bool {
    s.chars().next_back().is_some_and(|c| "([{".contains(c))
}

/// The result of running a keystroke stream through an editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorState {
    /// Final buffer contents.
    pub buffer: String,
    /// Final cursor offset (where the last keystroke left it).
    pub rest: usize,
    /// `%0`'s offset, or `None`.
    pub target: Option<usize>,
}
