//! Smart-profile rendering: auto-close + type-over + block-expand, so only the
//! keystrokes the editor won't supply are emitted.

use super::{BuildResult, build_dict, entry_events, wrap};
use crate::editor::SMART;
use crate::error::RenderError;
use crate::expand::TypedEntry;

/// Render one expanded+typed entry to a Plover `(key, value)` (smart profile).
///
/// # Errors
/// Returns [`RenderError`] if the template still holds an unresolved chunk or
/// movement compilation fails.
pub fn render_smart(entry: &TypedEntry) -> Result<(String, String), RenderError> {
    Ok(wrap(&entry.stroke, &entry_events(entry, SMART)?))
}

/// Render many entries into the smart-brace Plover dictionary.
///
/// # Errors
/// Propagates any [`RenderError`] from rendering an entry.
pub fn build_smart_dict(entries: &[TypedEntry]) -> Result<BuildResult, RenderError> {
    build_dict(entries, render_smart)
}
