//! Compiler from steno-theory source (`.steno`) to Plover JSON dictionaries
//! and LSP snippet artifacts.
//!
//! The pipeline is a chain of one-way passes over `dict.steno`:
//!
//! 1. parse — fenced source text to an entry AST
//! 2. expand — count fan-out, type-append chains, one-liner variants
//! 3. render — Plover dictionaries (plain and smart editor profiles),
//!    LSP snippets, and `@literal` structural blocks
//!
//! Data flows strictly forward; every failure is a typed error carrying the
//! 1-based source line where applicable. Output files are written only by the
//! `build-dict`, `build-nvim`, `measure`, and `viz-data` binaries, never by
//! library code.

mod blocks;
mod editor;
mod error;
mod expand;
mod json_out;
mod parse;
mod render;
mod snippet;
mod stroke;

pub use blocks::{emit_struct, struct_text};
pub use editor::{
    Behaviors, EditorState, Event, INDENT_UNIT, KeyName, PLAIN, SMART, SMART_INDENT, escape_text,
    interpret, movement_events, serialize,
};
pub use error::{ExpandError, RenderError, SnippetError, StenoError, StrokeError};
pub use expand::{
    Construct, ExpandedEntry, ITERATION_BASE, InfType, TypeDef, TypeOptions, TypeSet, TypedEntry,
    WalkResult, build_constructs, build_tables, build_type_set, build_types, check_fuse_ambiguity,
    emit_data_header, emit_test_header, expand_all, expand_counts, expand_dict, expand_line_flag,
    expand_types, expand_types_one, render_filled, template_fragments, walk,
};
pub use json_out::{OrderedMap, json_string, to_json};
pub use parse::{Chunk, Entry, EntryFlags, Expr, parse_source, parse_template};
pub use render::{BuildResult, build_plain_dict, build_smart_dict, render_plain, render_smart};
pub use snippet::{
    SENTINEL_CLOSE, SENTINEL_OPEN, SnippetBuild, SnippetEntry, build_snippets, render_snippet,
};
pub use stroke::{
    CountBank, CountBit, KeySet, LEFT_ORDER, MID_ORDER, RIGHT_ORDER, Side, StrokeKeys, add_key,
    apply_count, count_bank, merge_strokes, parse_stroke, render_stroke,
};
