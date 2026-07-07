//! Type definitions for Pass B: the append set collected from `@type`
//! entries.

use crate::error::ExpandError;
use crate::parse::{Chunk, Entry};

/// One appendable type from a `@type` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDef {
    /// The type's own stroke, e.g. `TPH`.
    pub stroke: String,
    /// Generic-argument count (0 = concrete type).
    pub arity: u32,
    /// Rendered text with `%t` arg markers, e.g. "string" or "Map<%t, %t>".
    pub text: String,
    /// Empty type (SKP): a free/custom type — leaves `: ` and a tabstop to
    /// type by hand.
    pub free_type: bool,
}

/// The append set available to `%t` slots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeOptions {
    /// Top-level types a `%t` may take (all of them, generics included).
    pub types: Vec<TypeDef>,
    /// Types usable as a generic's argument (arity-0 only; no nesting).
    pub generic_args: Vec<TypeDef>,
}

/// The collected `@type` entries: the full set plus the arity-0 pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeSet {
    /// Every `@type` entry as a [`TypeDef`].
    pub types: Vec<TypeDef>,
    /// Arity-0 concrete types realistic as generic arguments.
    pub arity0: Vec<TypeDef>,
}

/// Render a `@type` entry's template to its text form (only literals and
/// `%t` markers are legal).
fn render_type_text(template: &[Chunk]) -> Result<String, ExpandError> {
    let mut out = String::new();
    for c in template {
        match c {
            Chunk::Lit(text) => out.push_str(text),
            Chunk::TypeSlot => out.push_str("%t"),
            other => {
                return Err(ExpandError::new(format!(
                    "@type entry has an unexpected chunk \"{}\"",
                    other.kind_name()
                )));
            },
        }
    }
    Ok(out)
}

/// Collect the `@type` entries into the append set (full set + arity-0
/// pool).
///
/// # Errors
/// Returns [`ExpandError`] when a `@type` template contains anything besides
/// literals and `%t` markers.
pub fn build_type_set(entries: &[Entry]) -> Result<TypeSet, ExpandError> {
    let mut types = Vec::new();
    let mut no_arg: Vec<&str> = Vec::new();
    for e in entries.iter().filter(|e| e.flags.is_type()) {
        let text = render_type_text(&e.template)?;
        types.push(TypeDef {
            stroke: e.stroke_raw.clone(),
            arity: e.arity.unwrap_or(0),
            free_type: text.is_empty(),
            text,
        });
        if e.flags.no_arg() {
            no_arg.push(&e.stroke_raw);
        }
    }
    // Generic-arg pool = arity-0 concrete types realistic as type arguments.
    let arity0 = types
        .iter()
        .filter(|t| t.arity == 0 && !t.free_type && !no_arg.contains(&t.stroke.as_str()))
        .cloned()
        .collect();
    Ok(TypeSet { types, arity0 })
}
