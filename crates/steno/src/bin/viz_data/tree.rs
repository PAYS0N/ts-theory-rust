//! The category/stroke tree's data model, consumed by `viz/index.html`.
//! Categories (in file order) hold root-level choices; every stroke gets a
//! node with its rendered preview text, terminal flag, and explicit
//! next-step keys. `@count`/`@fuse` alternatives of one authored line
//! collapse into a single choice ("axis") node instead of flat
//! near-identical siblings — see `axis.rs` for the bucketing rule and
//! `build.rs` for the assembly pass that populates a [`Tree`].

use std::collections::HashMap;

use steno::{KeySet, TypedEntry, parse_stroke, render_stroke};

use crate::axis::Axis;

/// One stroke's rendered preview text(s) and its next-step keys.
pub struct Node {
    /// True for a completed construct (never itself extended by a later
    /// stroke).
    pub terminal: bool,
    /// Rendered preview text for the multi-line variant, if this stroke has
    /// one.
    pub multi_line: Option<String>,
    /// Rendered preview text for the one-liner (`U`) variant, if this
    /// stroke has one.
    pub one_liner: Option<String>,
    /// Next-step keys: real stroke children, or a single synthetic axis key
    /// standing in for several.
    pub children: Vec<String>,
    /// Present only on a synthetic choice node.
    pub axis: Option<Axis>,
    /// A `### text` comment attached to this stroke's authored line, shown
    /// in place of a blank/uninformative render.
    pub description: Option<String>,
    /// The `U`-augmented first `/`-segment of this stroke's one-liner
    /// variant (the physical stroke that actually changes — see
    /// `lineflag.rs`), present only when this node has a `one_liner` text.
    pub one_liner_root: Option<String>,
    /// True for a synthetic `##>` section grouping node that has no real
    /// stroke of its own — the viewer skips the keyboard diagram for these.
    pub synthetic: bool,
}

/// A named group of root-level (first-stroke) choices, in file order.
pub struct Category {
    /// The `## label` this category was named from.
    pub label: String,
    /// Root-level next-step keys belonging to this category.
    pub roots: Vec<String>,
}

/// Categories in file order, plus every node (leaf or choice), in
/// first-seen/first-created order for deterministic output.
pub struct Tree {
    /// Categories in file order.
    pub categories: Vec<Category>,
    /// Node keys in first-seen/first-created order.
    pub node_order: Vec<String>,
    /// Node data keyed by identity.
    pub nodes: HashMap<String, Node>,
}

impl Tree {
    /// Total number of distinct stroke nodes (leaves and choice nodes).
    #[must_use]
    pub const fn node_count(&self) -> usize {
        self.node_order.len()
    }

    /// Total number of categories.
    #[must_use]
    pub const fn category_count(&self) -> usize {
        self.categories.len()
    }
}

/// The leaf-identity key for an entry: its `/`-joined stroke, with the
/// injected `U` one-liner marker removed from the first segment's key set
/// (then correctly re-rendered — hyphen placement depends on which keys are
/// present, so this can't be done by string-splicing) so a one-liner variant
/// merges into the same node as its multi-line sibling.
///
/// # Errors
/// Returns the parse error message if the first segment isn't valid stroke
/// syntax (can't happen for a `U`-injected segment `add_key` itself produced).
pub fn group_key(entry: &TypedEntry) -> Result<String, String> {
    if !entry.one_liner {
        return Ok(entry.stroke.clone());
    }
    match entry.stroke.split_once('/') {
        Some((first, rest)) => Ok(format!("{}/{rest}", strip_u(first)?)),
        None => strip_u(&entry.stroke),
    }
}

/// Remove the `U` key from a single stroke segment's mid bank and
/// re-render it in canonical form.
fn strip_u(seg: &str) -> Result<String, String> {
    let mut keys = parse_stroke(seg).map_err(|e| e.to_string())?;
    let mut mid = KeySet::default();
    for ch in keys.mid.keys().filter(|&c| c != 'U') {
        mid.insert(ch);
    }
    keys.mid = mid;
    Ok(render_stroke(&keys))
}

/// Prefix marking a synthetic section-grouping node's key (see `position`).
const SEC_PREFIX: &str = "sec:";

/// A leaf key's parent tree position: a synthetic per-category position for a
/// root (so category-root grouping reuses the same bucketing as ordinary
/// descendants), a synthetic per-section position for a descendant carrying a
/// `##>` section (so same-section siblings bucket together under their shared
/// parent), or the bare stroke prefix otherwise.
pub fn position(key: &str, category_label: &str, section: Option<&str>) -> String {
    match key.rsplit_once('/') {
        None => format!("cat:{category_label}"),
        Some((parent, _)) => {
            section.map_or_else(|| parent.to_owned(), |s| section_position(parent, s))
        },
    }
}

/// The synthetic key grouping a descendant with its same-section siblings
/// under the shared `parent` stroke. `parent` is `/`-joined stroke text with
/// no spaces, so the single space delimiter re-splits cleanly in
/// [`section_parts`].
fn section_position(parent: &str, label: &str) -> String {
    format!("{SEC_PREFIX}{parent} {label}")
}

/// Split a section-node key back into `(parent_position, section_label)`, or
/// `None` if `key` isn't a section key.
pub fn section_parts(key: &str) -> Option<(&str, &str)> {
    key.strip_prefix(SEC_PREFIX)?.split_once(' ')
}
