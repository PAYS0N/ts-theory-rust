//! Plain-profile rendering: a dumb editor types every closer and walks the
//! cursor back from the document end.

use super::{BuildResult, build_dict, entry_events, wrap};
use crate::editor::PLAIN;
use crate::error::RenderError;
use crate::expand::TypedEntry;

/// Render one expanded+typed entry to a Plover `(key, value)` (plain profile).
///
/// # Errors
/// Returns [`RenderError`] if the template still holds an unresolved chunk or
/// movement compilation fails.
pub fn render_plain(entry: &TypedEntry) -> Result<(String, String), RenderError> {
    Ok(wrap(&entry.stroke, &entry_events(entry, PLAIN)?))
}

/// Render many entries into the plain Plover dictionary, flagging collisions.
///
/// # Errors
/// Propagates any [`RenderError`] from rendering an entry.
pub fn build_plain_dict(entries: &[TypedEntry]) -> Result<BuildResult, RenderError> {
    build_dict(entries, render_plain)
}
