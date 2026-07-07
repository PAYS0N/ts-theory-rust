//! Stage 1 of the pipeline: source text -> typed AST.
//!
//! This module does NOT expand counts/types, compile movement, or emit JSON —
//! it only turns a `.steno` source into `Vec<Entry>` with each template
//! parsed to `Vec<Chunk>`.

mod expr;
mod source;
mod template;

pub use source::parse_source;
pub use template::parse_template;

/// A computed index `%(EXPR)`, stored in linear form `a*d + b` (d = repeat
/// index).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Expr {
    /// Coefficient of `d`.
    pub a: i32,
    /// Constant offset.
    pub b: i32,
}

/// One parsed template node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Chunk {
    /// Literal run of text.
    Lit(String),
    /// A raw structural `{` or `}` (a profile may strip closers).
    Brace {
        /// True for `{`, false for `}`.
        open: bool,
    },
    /// The `\n` escape.
    Newline,
    /// The `\t` escape.
    Tab,
    /// `%0`..`%9` ordered landing point.
    Landing(u32),
    /// `%d`, the in-scope count from the bank.
    Dcount,
    /// `%t`, filled by an appended type stroke.
    TypeSlot,
    /// `%b`, the one-liner-toggleable newline.
    BodyBreak,
    /// `%p` destructuring slot.
    Pattern,
    /// `%[ sep | body %]` repeat block.
    Repeat {
        /// Joiner emitted between items, never after the last.
        sep: Vec<Self>,
        /// Repeated body.
        body: Vec<Self>,
    },
    /// `%(EXPR)` computed landing point.
    Computed(Expr),
}

impl Chunk {
    /// The kind tag as named in the source DSL design docs (used in error
    /// messages and diagnostics).
    #[must_use]
    pub const fn kind_name(&self) -> &'static str {
        match self {
            Self::Lit(_) => "lit",
            Self::Brace { .. } => "brace",
            Self::Newline => "newline",
            Self::Tab => "tab",
            Self::Landing(_) => "landing",
            Self::Dcount => "dcount",
            Self::TypeSlot => "typeslot",
            Self::BodyBreak => "bodybreak",
            Self::Pattern => "pattern",
            Self::Repeat { .. } => "repeat",
            Self::Computed(_) => "computed",
        }
    }
}

/// Set of boolean `@`-directives attached to an entry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EntryFlags(u8);

impl EntryFlags {
    /// Bit for `@multiline`.
    const MULTILINE: u8 = 1;
    /// Bit for `@type`.
    const IS_TYPE: u8 = 1 << 1;
    /// Bit for `@noarg`.
    const NO_ARG: u8 = 1 << 2;
    /// Bit for `@fuse`.
    const FUSE: u8 = 1 << 3;
    /// Bit for `@literal`.
    const LITERAL: u8 = 1 << 4;

    /// `@multiline` — construct never collapses to one line (ignores the O
    /// flag).
    #[must_use]
    pub const fn multiline(self) -> bool {
        self.0 & Self::MULTILINE != 0
    }

    /// `@type` — this entry is an appendable type, not a standalone
    /// construct.
    #[must_use]
    pub const fn is_type(self) -> bool {
        self.0 & Self::IS_TYPE != 0
    }

    /// `@noarg` — a type that may be a return type but never a generic
    /// argument.
    #[must_use]
    pub const fn no_arg(self) -> bool {
        self.0 & Self::NO_ARG != 0
    }

    /// `@fuse` — fuse the last stroke segment into the first appended type
    /// stroke (so the type-less intermediate is never a required stroke).
    #[must_use]
    pub const fn fuse(self) -> bool {
        self.0 & Self::FUSE != 0
    }

    /// `@literal` — a complete pre-formatted block: the smart profile emits
    /// it byte-identical to plain (no closer-drop), since a smart editor
    /// mangles whole-code dumps regardless of profile.
    #[must_use]
    pub const fn literal(self) -> bool {
        self.0 & Self::LITERAL != 0
    }

    /// Set the flag named by its directive. Returns false when `name` is not
    /// one of the boolean directives (crate-internal; the directive parser is
    /// the only writer).
    pub(crate) const fn set_named(&mut self, name: &str) -> bool {
        let bit = match name.as_bytes() {
            b"multiline" => Self::MULTILINE,
            b"type" => Self::IS_TYPE,
            b"noarg" => Self::NO_ARG,
            b"fuse" => Self::FUSE,
            b"literal" => Self::LITERAL,
            _ => return false,
        };
        self.0 |= bit;
        true
    }
}

/// One dictionary entry: a stroke sequence, its parsed template, and any
/// attached directives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Stroke key split on `/`, e.g. `["STKWR-PBGS", "-FLT"]`.
    pub stroke: Vec<String>,
    /// The stroke exactly as written on the opening fence.
    pub stroke_raw: String,
    /// Parsed template chunks.
    pub template: Vec<Chunk>,
    /// Raw template text between the fences (for debugging / round-trip).
    pub raw: String,
    /// `@count` key spec, e.g. "AOEU". Weights are derived later from board
    /// geometry.
    pub count: Option<String>,
    /// `@arity N` — type-arg count for a generic type entry.
    pub arity: Option<u32>,
    /// Boolean directive flags.
    pub flags: EntryFlags,
    /// 1-based line of the opening fence.
    pub line: usize,
}
