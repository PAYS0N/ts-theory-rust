//! Line-expansion flag: every TERMINAL entry that contains a `%b` (and isn't
//! a forced-multiline construct) also gets a U-keyed one-liner variant where
//! the body break collapses. The default (no U) stays multi-line.

use super::TypedEntry;
use crate::error::ExpandError;
use crate::parse::Chunk;
use crate::stroke::add_key;

/// Emit each entry plus, where eligible, its U-keyed one-liner variant.
///
/// # Errors
/// Returns [`ExpandError`] when the U key cannot be added to an entry's
/// first stroke segment.
pub fn expand_line_flag(entries: &[TypedEntry]) -> Result<Vec<TypedEntry>, ExpandError> {
    let mut out = Vec::new();
    for e in entries {
        out.push(e.clone()); // default: multi-line
        let has_break = e.template.iter().any(|c| matches!(c, Chunk::BodyBreak));
        if e.terminal && has_break && !e.source.flags.multiline() {
            // The one-liner decision is made up front, so U rides the FIRST
            // stroke (the construct's base), not the final type stroke.
            let mut segs: Vec<String> = e.stroke.split('/').map(String::from).collect();
            if let Some(first) = segs.first_mut() {
                *first = add_key(first, 'U')?;
            }
            out.push(TypedEntry {
                stroke: segs.join("/"),
                one_liner: true,
                ..e.clone()
            });
        }
    }
    Ok(out)
}
