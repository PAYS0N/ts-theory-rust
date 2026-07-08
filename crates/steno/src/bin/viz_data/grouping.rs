//! Second build pass: buckets leaves into axis groups, appends synthetic
//! axis nodes, and resolves category roots from the same buckets.

use std::collections::HashMap;

use steno::TypedEntry;

use crate::axis::{self, Leaf};
use crate::comments::{Marker, label_for};
use crate::tree::{Category, Node, position};

/// Build every leaf's grouping input: its key, tree position, and a
/// representative entry.
pub fn leaves<'a>(
    node_order: &[String],
    reps: &HashMap<String, &'a TypedEntry>,
    markers: &[Marker],
) -> Vec<Leaf<'a>> {
    node_order
        .iter()
        .filter_map(|key| {
            let entry = *reps.get(key)?;
            let label = label_for(markers, entry.source.line);
            Some(Leaf {
                key: key.clone(),
                position: position(key, label),
                entry,
            })
        })
        .collect()
}

/// Populate every leaf node's `children` from the bucketed positions.
fn apply_children(nodes: &mut HashMap<String, Node>, children_of: &HashMap<String, Vec<String>>) {
    for (key, node) in nodes.iter_mut() {
        if let Some(kids) = children_of.get(key) {
            node.children.clone_from(kids);
        }
    }
}

/// An axis node's mirrored preview fields, copied from its first option, so
/// anything that only reads `terminal`/`multiLine`/`oneLiner`/`children`/
/// `description`/`oneLinerRoot` degrades gracefully.
#[derive(Default)]
struct Mirror {
    /// Mirrors [`Node::terminal`].
    terminal: bool,
    /// Mirrors [`Node::multi_line`].
    multi_line: Option<String>,
    /// Mirrors [`Node::one_liner`].
    one_liner: Option<String>,
    /// Mirrors [`Node::children`].
    children: Vec<String>,
    /// Mirrors [`Node::description`].
    description: Option<String>,
    /// Mirrors [`Node::one_liner_root`].
    one_liner_root: Option<String>,
}

/// Mirror an axis's first option's preview fields.
fn mirror_fields(nodes: &HashMap<String, Node>, data: &axis::Axis) -> Mirror {
    data.choices
        .first()
        .and_then(|c| nodes.get(&c.key))
        .map_or_else(Mirror::default, |n| Mirror {
            terminal: n.terminal,
            multi_line: n.multi_line.clone(),
            one_liner: n.one_liner.clone(),
            children: n.children.clone(),
            description: n.description.clone(),
            one_liner_root: n.one_liner_root.clone(),
        })
}

/// Append one synthetic axis node.
fn add_axis_node(
    node_order: &mut Vec<String>,
    nodes: &mut HashMap<String, Node>,
    axis_key: String,
    data: axis::Axis,
) {
    let m = mirror_fields(nodes, &data);
    node_order.push(axis_key.clone());
    nodes.insert(
        axis_key,
        Node {
            terminal: m.terminal,
            multi_line: m.multi_line,
            one_liner: m.one_liner,
            children: m.children,
            axis: Some(data),
            description: m.description,
            one_liner_root: m.one_liner_root,
        },
    );
}

/// Apply the full grouping pass: children on every leaf, synthetic axis
/// nodes appended, and category roots resolved from the same buckets.
pub fn apply_grouping(
    node_order: &mut Vec<String>,
    nodes: &mut HashMap<String, Node>,
    categories: &mut [Category],
    grouped: axis::Grouped,
) {
    apply_children(nodes, &grouped.children_of);
    for (axis_key, data) in grouped.axes {
        add_axis_node(node_order, nodes, axis_key, data);
    }
    for cat in categories {
        if let Some(roots) = grouped.children_of.get(&format!("cat:{}", cat.label)) {
            cat.roots.clone_from(roots);
        }
    }
}
