//! Passes A and B — count expansion and type-append — plus the one-liner
//! line-flag pass. Bridges the parsed AST to the `TypedEntry` records both
//! renderers consume.

mod append;
mod counts;
mod family;
mod fill;
mod lineflag;
mod types;

pub use append::{expand_types, expand_types_one};
pub use counts::{ITERATION_BASE, expand_all, expand_counts};
pub use family::fix_family_terminals;
pub use lineflag::expand_line_flag;
pub use types::{TypeDef, TypeOptions, TypeSet, build_type_set};

use crate::error::ExpandError;
use crate::parse::{Chunk, Entry};

/// One count-resolved entry (Pass A output).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedEntry {
    /// Full Plover key, e.g. `STKWR-PBGS/AOFLT`.
    pub stroke: String,
    /// Count-resolved chunks: no repeat/dcount/computed remain.
    pub template: Vec<Chunk>,
    /// The count value, or None for a non-count entry.
    pub count: Option<u32>,
    /// The parsed entry this expansion came from.
    pub source: Entry,
}

/// One fully expanded entry (Pass B output), ready for rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedEntry {
    /// Full Plover key including any appended type strokes.
    pub stroke: String,
    /// Template with every type slot resolved.
    pub template: Vec<Chunk>,
    /// False = non-terminal step (later passes drop bracing and skip
    /// movement).
    pub terminal: bool,
    /// True = the U one-liner variant (every `%b` collapses instead of
    /// breaking).
    pub one_liner: bool,
    /// The count value, or None for a non-count entry.
    pub count: Option<u32>,
    /// The parsed entry this expansion came from.
    pub source: Entry,
}

/// Full Pass A + Pass B + line-flag over a parsed dictionary.
///
/// `@type` entries become the append set (consumed, not emitted); generic
/// args use the full arity-0 pool unless `generic_arg_strokes` restricts it.
///
/// # Errors
/// Returns [`ExpandError`] on any count/type misconfiguration or stroke
/// arithmetic failure.
pub fn expand_dict(
    entries: &[Entry],
    generic_arg_strokes: Option<&[&str]>,
) -> Result<Vec<TypedEntry>, ExpandError> {
    let TypeSet { types, arity0 } = build_type_set(entries)?;
    let pool = generic_arg_strokes.map_or_else(
        || arity0.clone(),
        |strokes| {
            arity0
                .iter()
                .filter(|t| strokes.contains(&t.stroke.as_str()))
                .cloned()
                .collect()
        },
    );
    let opts = TypeOptions {
        types,
        generic_args: pool,
    };
    let mut typed = Vec::new();
    for e in entries.iter().filter(|e| !e.flags.is_type()) {
        for expanded in expand_counts(e)? {
            typed.extend(expand_types_one(&expanded, &opts)?);
        }
    }
    fix_family_terminals(&mut typed);
    expand_line_flag(&typed)
}
