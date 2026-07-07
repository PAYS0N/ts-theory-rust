//! Assemble the two nvim build artifacts from rendered [`SnippetEntry`]s.

use super::{SENTINEL_CLOSE, SENTINEL_OPEN, SnippetEntry, render_snippet};
use crate::error::SnippetError;
use crate::expand::TypedEntry;
use crate::json_out::OrderedMap;

/// Both nvim artifacts built from the typed entries.
pub struct SnippetBuild {
    /// Plover dictionary: stroke → either a plain `{^}...{^}` partial
    /// (non-terminal) or a sentinel-wrapped token the nvim plugin expands
    /// (terminal).
    pub plover_keys: OrderedMap,
    /// Snippet table: terminal `key_id` → LSP body.
    pub snippets: OrderedMap,
    /// Strokes that mapped to two different values.
    pub collisions: Vec<String>,
}

/// Insert a non-terminal's plain `{^}...{^}` value (no sentinel — see
/// [`build_snippets`]), flagging a collision if the stroke already mapped to
/// something different.
fn insert_non_terminal(
    plover_keys: &mut OrderedMap,
    collisions: &mut Vec<String>,
    key_id: String,
    body: &str,
) {
    let value = format!("{{^}}{body}{{^}}");
    if plover_keys.get(&key_id).is_some_and(|prev| *prev != value) {
        collisions.push(key_id.clone());
    }
    plover_keys.insert(key_id, value);
}

/// Insert a terminal's sentinel-wrapped token and its snippet body, flagging
/// a collision if `key_id` already mapped to a different body.
fn insert_terminal(
    plover_keys: &mut OrderedMap,
    snippets: &mut OrderedMap,
    collisions: &mut Vec<String>,
    key_id: String,
    body: String,
) {
    if snippets.get(&key_id).is_some_and(|prev| *prev != body) {
        collisions.push(key_id.clone());
    }
    // Plover inserts a space before a translation with no `{^}` glue;
    // without it, every expanded token would land one space off from
    // whatever precedes it in the code.
    let token = format!("{{^}}{SENTINEL_OPEN}{key_id}{SENTINEL_CLOSE}{{^}}");
    snippets.insert(key_id.clone(), body);
    plover_keys.insert(key_id, token);
}

/// Build both artifacts from the typed entries.
///
/// Non-terminal (type-append intermediate) strokes are typed as plain
/// literal text, same as the plain/smart profiles — never sentinel-wrapped.
/// A non-terminal partial always gets superseded by a longer chord, and
/// Plover corrects that by backspacing exactly what it last typed; if the
/// nvim plugin had already rewritten that text (expanding the sentinel
/// token into its snippet body), Plover's backspace count would no longer
/// match what's actually in the buffer and the correction would eat
/// whatever precedes it. Only a terminal (a completed construct, never
/// itself extended by a later stroke) is safe to sentinel-wrap and hand to
/// the nvim plugin.
///
/// # Errors
/// Propagates any [`SnippetError`] from rendering an entry.
pub fn build_snippets(entries: &[TypedEntry]) -> Result<SnippetBuild, SnippetError> {
    let mut plover_keys = OrderedMap::new();
    let mut snippets = OrderedMap::new();
    let mut collisions = Vec::new();
    for e in entries {
        let SnippetEntry {
            key_id,
            body,
            terminal,
        } = render_snippet(e)?;
        if terminal {
            insert_terminal(
                &mut plover_keys,
                &mut snippets,
                &mut collisions,
                key_id,
                body,
            );
        } else {
            insert_non_terminal(&mut plover_keys, &mut collisions, key_id, &body);
        }
    }
    Ok(SnippetBuild {
        plover_keys,
        snippets,
        collisions,
    })
}
