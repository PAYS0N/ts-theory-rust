//! Assembles a [`Tree`] from expanded entries: renders each leaf's preview
//! text, then runs the `axis` bucketing pass to fill in `children`/`axis`
//! and resolve category roots.

use std::collections::HashMap;

use steno::{TypedEntry, render_snippet};

use crate::axis;
use crate::comments::{Marker, label_for};
use crate::grouping;
use crate::tree::{Category, Node, Tree, group_key};

/// A freshly created node for `entry`, with no rendered text yet.
const fn blank_node(entry: &TypedEntry) -> Node {
    Node {
        terminal: entry.terminal,
        multi_line: None,
        one_liner: None,
        children: Vec::new(),
        axis: None,
        description: None,
        one_liner_root: None,
        synthetic: false,
    }
}

/// The first `/`-segment of `stroke` (the whole string when there's no
/// `/`) — the physical stroke a one-liner variant's `U` key actually rides,
/// before `tree::group_key` strips it back out to merge variants.
fn root_segment(stroke: &str) -> String {
    stroke
        .split_once('/')
        .map_or(stroke, |(first, _)| first)
        .to_owned()
}

/// Merge one entry's rendered text into `nodes`/`node_order`/`reps`,
/// creating the node and recording a representative entry on first sight.
fn insert_leaf<'a>(
    node_order: &mut Vec<String>,
    nodes: &mut HashMap<String, Node>,
    reps: &mut HashMap<String, &'a TypedEntry>,
    entry: &'a TypedEntry,
    key: &str,
    text: String,
    desc: Option<&str>,
) {
    if !nodes.contains_key(key) {
        node_order.push(key.to_owned());
        reps.insert(key.to_owned(), entry);
        nodes.insert(key.to_owned(), blank_node(entry));
    }
    let Some(node) = nodes.get_mut(key) else {
        return;
    };
    node.terminal = entry.terminal;
    if let Some(d) = desc {
        node.description = Some(d.to_owned());
    }
    if entry.one_liner {
        node.one_liner = Some(text);
        node.one_liner_root = Some(root_segment(&entry.stroke));
    } else {
        node.multi_line = Some(text);
    }
}

/// Record `label`'s category on first sight, in file order.
fn ensure_category(
    categories: &mut Vec<Category>,
    category_index: &mut HashMap<String, usize>,
    label: &str,
) {
    category_index.entry(label.to_owned()).or_insert_with(|| {
        categories.push(Category {
            label: label.to_owned(),
            roots: Vec::new(),
        });
        categories.len() - 1
    });
}

/// Mutable build-in-progress state threaded through entry insertion,
/// bundled into one struct to stay under clippy's argument-count limit.
struct BuildState<'s, 'a> {
    /// First-seen node key order (preserves file order in the output).
    node_order: &'s mut Vec<String>,
    /// Every node built so far, keyed by its grouping key.
    nodes: &'s mut HashMap<String, Node>,
    /// Each key's first-seen representative entry, for the grouping pass.
    reps: &'s mut HashMap<String, &'a TypedEntry>,
    /// Categories in first-seen order.
    categories: &'s mut Vec<Category>,
    /// Category label -> index into `categories`, for dedup.
    category_index: &'s mut HashMap<String, usize>,
}

/// Render and record one entry: its leaf node, and its category.
///
/// # Errors
/// Propagates a snippet-rendering failure as its message.
fn insert_entry<'a>(
    state: &mut BuildState<'_, 'a>,
    markers: &[Marker],
    desc_map: &HashMap<usize, String>,
    entry: &'a TypedEntry,
) -> Result<(), String> {
    let snippet = render_snippet(entry).map_err(|e| e.to_string())?;
    let key = group_key(entry)?;
    let desc = desc_map.get(&entry.source.line).map(String::as_str);
    insert_leaf(
        state.node_order,
        state.nodes,
        state.reps,
        entry,
        &key,
        snippet.body,
        desc,
    );
    let label = label_for(markers, entry.source.line);
    ensure_category(state.categories, state.category_index, label);
    Ok(())
}

/// Insert every entry into `state`, populating nodes/categories.
///
/// # Errors
/// Propagates the first entry's rendering failure as its message.
fn insert_all<'a>(
    typed: &'a [TypedEntry],
    markers: &[Marker],
    desc_map: &HashMap<usize, String>,
    state: &mut BuildState<'_, 'a>,
) -> Result<(), String> {
    for entry in typed {
        insert_entry(state, markers, desc_map, entry)?;
    }
    Ok(())
}

/// Build the full tree from expanded entries and the source's `##` category
/// and `##>` section markers.
///
/// # Errors
/// Propagates any rendering failure as its message.
pub fn build(
    typed: &[TypedEntry],
    markers: &[Marker],
    sections: &[Marker],
    desc_map: &HashMap<usize, String>,
) -> Result<Tree, String> {
    let (mut tree, reps) = insert_leaves(typed, markers, desc_map)?;
    group_leaves(&mut tree, &reps, markers, sections);
    Ok(tree)
}

/// Render every entry into a leaf-only [`Tree`], returning it alongside the
/// per-key representative entries the grouping pass still needs.
///
/// # Errors
/// Propagates any rendering failure as its message.
fn insert_leaves<'a>(
    typed: &'a [TypedEntry],
    markers: &[Marker],
    desc_map: &HashMap<usize, String>,
) -> Result<(Tree, HashMap<String, &'a TypedEntry>), String> {
    let mut categories = Vec::new();
    let mut category_index = HashMap::new();
    let mut node_order = Vec::new();
    let mut nodes = HashMap::new();
    let mut reps: HashMap<String, &TypedEntry> = HashMap::new();
    let mut state = BuildState {
        node_order: &mut node_order,
        nodes: &mut nodes,
        reps: &mut reps,
        categories: &mut categories,
        category_index: &mut category_index,
    };
    insert_all(typed, markers, desc_map, &mut state)?;
    Ok((
        Tree {
            categories,
            node_order,
            nodes,
        },
        reps,
    ))
}

/// Run both grouping passes over the freshly-inserted leaf nodes: bucket
/// leaves into axes, then splice in section/axis nodes and resolve roots.
fn group_leaves(
    tree: &mut Tree,
    reps: &HashMap<String, &TypedEntry>,
    markers: &[Marker],
    sections: &[Marker],
) {
    let inputs = grouping::leaves(&tree.node_order, reps, markers, sections);
    let grouped = axis::group(&inputs.leaves);
    grouping::apply_grouping(
        &mut tree.node_order,
        &mut tree.nodes,
        &mut tree.categories,
        grouped,
        &inputs.sections,
    );
}
