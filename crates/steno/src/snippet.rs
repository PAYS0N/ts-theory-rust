//! Snippet target — render a [`TypedEntry`] to an LSP snippet body for
//! editor-side expansion (Neovim's built-in `vim.snippet`, VS Code, etc.).
//!
//! Unlike [`crate::render`] (which bakes cursor movement into Plover
//! keystrokes), the editor's snippet engine owns cursor placement via tabstops.
//! So there is no movement math, no closer-dropping, and no plain/smart split.
//!
//! Landings map to LSP tabstops renumbered to tab order: the lowest landing
//! becomes `${1}` and ascends, and the HIGHEST landing becomes `${0}` (LSP's
//! final exit). Non-terminal strokes emit the same bracket-stripped partial
//! text they do in the plain profile, with no tabstops.

use crate::error::SnippetError;
use crate::expand::TypedEntry;
use crate::json_out::OrderedMap;
use crate::parse::Chunk;

/// Sentinel pair wrapping the keyset token Plover types (so the plugin can find
/// it without colliding with ordinary text). Keep in sync with the nvim plugin.
pub const SENTINEL_OPEN: &str = "@@";
/// Closing sentinel (see [`SENTINEL_OPEN`]).
pub const SENTINEL_CLOSE: &str = "@@";

/// True for any delimiter stripped from a non-terminal partial.
const fn is_bracket(ch: char) -> bool {
    matches!(ch, '(' | ')' | '[' | ']' | '<' | '>' | '{' | '}')
}

/// Drop every bracket from `s`.
fn strip_brackets(s: &str) -> String {
    s.chars().filter(|c| !is_bracket(*c)).collect()
}

/// Escape literal text for an LSP snippet body (tabstops are emitted raw).
fn esc(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        if "\\$}".contains(ch) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// LSP tabstop index for a landing: highest → 0 (exit), the rest 1..k ascending.
fn tab_index(n: u32, sorted: &[u32]) -> u32 {
    if sorted.last() == Some(&n) {
        return 0;
    }
    sorted
        .iter()
        .position(|&x| x == n)
        .and_then(|i| u32::try_from(i + 1).ok())
        .unwrap_or(0)
}

/// The unique landing indices in a template, ascending.
fn sorted_landings(template: &[Chunk]) -> Vec<u32> {
    let mut landings: Vec<u32> = template
        .iter()
        .filter_map(|c| match c {
            Chunk::Landing(n) => Some(*n),
            _ => None,
        })
        .collect();
    landings.sort_unstable();
    landings.dedup();
    landings
}

/// One rendered snippet.
pub struct SnippetEntry {
    /// Stable id: the Plover-emitted token (sentinel-wrapped) and table key.
    pub key_id: String,
    /// LSP snippet syntax body.
    pub body: String,
    /// Whether this is a terminal construct.
    pub terminal: bool,
}

/// Render a non-terminal "pre-function" partial: strip brackets, no tabstops.
fn render_non_terminal(entry: &TypedEntry) -> Result<SnippetEntry, SnippetError> {
    let mut body = String::new();
    for c in &entry.template {
        match c {
            Chunk::Lit(t) => body.push_str(&esc(&strip_brackets(t))),
            Chunk::Tab => body.push('\t'),
            // dropped: irreversible / not needed for an intermediate
            Chunk::Brace { .. } | Chunk::Newline | Chunk::BodyBreak | Chunk::Landing(_) => {},
            other => return Err(unresolved(other)),
        }
    }
    Ok(SnippetEntry {
        key_id: entry.stroke.clone(),
        body,
        terminal: false,
    })
}

/// Render a terminal construct with landings mapped to tabstops.
fn render_terminal(entry: &TypedEntry) -> Result<SnippetEntry, SnippetError> {
    let sorted = sorted_landings(&entry.template);
    let mut body = String::new();
    for c in &entry.template {
        match c {
            Chunk::Lit(t) => body.push_str(&esc(t)),
            Chunk::Brace { open } => push_brace(*open, &mut body),
            Chunk::BodyBreak => {
                if !entry.one_liner {
                    body.push('\n');
                }
            },
            Chunk::Newline => body.push('\n'),
            Chunk::Tab => body.push('\t'),
            Chunk::Landing(n) => push_tabstop(tab_index(*n, &sorted), &mut body),
            other => return Err(unresolved(other)),
        }
    }
    Ok(SnippetEntry {
        key_id: entry.stroke.clone(),
        body,
        terminal: true,
    })
}

/// Append an LSP tabstop `${index}` to `body`.
fn push_tabstop(index: u32, body: &mut String) {
    body.push('$');
    body.push('{');
    body.push_str(&index.to_string());
    body.push('}');
}

/// Append a brace: a lone `{` is literal in LSP; `}` must be escaped.
fn push_brace(open: bool, body: &mut String) {
    if open {
        body.push('{');
    } else {
        body.push('\\');
        body.push('}');
    }
}

/// Error for an unresolved chunk reaching the snippet renderer.
fn unresolved(chunk: &Chunk) -> SnippetError {
    SnippetError::new(format!(
        "unresolved chunk \"{}\" reached the snippet renderer",
        chunk.kind_name()
    ))
}

/// Render one expanded+typed entry to an LSP snippet.
///
/// # Errors
/// Returns [`SnippetError`] if the template still holds an unresolved chunk.
pub fn render_snippet(entry: &TypedEntry) -> Result<SnippetEntry, SnippetError> {
    if entry.terminal {
        render_terminal(entry)
    } else {
        render_non_terminal(entry)
    }
}

/// Both nvim artifacts built from the typed entries.
pub struct SnippetBuild {
    /// Plover dictionary: stroke → sentinel-wrapped token to type.
    pub plover_keys: OrderedMap,
    /// Snippet table: `key_id` → LSP body.
    pub snippets: OrderedMap,
    /// Strokes that mapped to two different bodies.
    pub collisions: Vec<String>,
}

/// Build both artifacts from the typed entries.
///
/// # Errors
/// Propagates any [`SnippetError`] from rendering an entry.
pub fn build_snippets(entries: &[TypedEntry]) -> Result<SnippetBuild, SnippetError> {
    let mut plover_keys = OrderedMap::new();
    let mut snippets = OrderedMap::new();
    let mut collisions = Vec::new();
    for e in entries {
        let SnippetEntry { key_id, body, .. } = render_snippet(e)?;
        if snippets.get(&key_id).is_some_and(|prev| prev != body) {
            collisions.push(key_id.clone());
        }
        let token = format!("{SENTINEL_OPEN}{key_id}{SENTINEL_CLOSE}");
        snippets.insert(key_id, body);
        plover_keys.insert(e.stroke.clone(), token);
    }
    Ok(SnippetBuild {
        plover_keys,
        snippets,
        collisions,
    })
}
