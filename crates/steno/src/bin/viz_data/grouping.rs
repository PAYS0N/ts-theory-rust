//! Second build pass: buckets leaves into axis groups, appends synthetic
//! axis nodes, and resolves category roots from the same buckets.

use std::collections::HashMap;

use steno::TypedEntry;

use std::collections::HashSet;

use crate::axis::{self, Leaf};
use crate::comments::{Marker, label_for, section_for};
use crate::tree::{Category, Node, position, section_parts};

/// A synthetic `##>` section node to splice into the tree: its own position
/// key, the parent stroke position it hangs off, and the label to display.
pub struct Section {
    /// The section node's key (also its bucketing position).
    pub key: String,
    /// The parent stroke position whose children list gains this section.
    pub parent: String,
    /// The label shown for the section (its `##>` text).
    pub label: String,
}

/// Every leaf's bucketing input plus the section nodes to splice in, both in
/// first-seen (file) order.
pub struct LeafInputs<'a> {
    /// One entry per original node, ready for axis bucketing.
    pub leaves: Vec<Leaf<'a>>,
    /// Synthetic section nodes, deduped in first-seen order.
    pub sections: Vec<Section>,
}

/// Build every leaf's grouping input (key, tree position, representative
/// entry) and the section nodes their positions imply. A descendant carrying
/// an active `##>` section gets a synthetic `sec:` position, which also
/// records a [`Section`] to splice under the shared parent stroke.
pub fn leaves<'a>(
    node_order: &[String],
    reps: &HashMap<String, &'a TypedEntry>,
    categories: &[Marker],
    sections: &[Marker],
) -> LeafInputs<'a> {
    let mut leaves = Vec::new();
    let mut secs = Vec::new();
    let mut seen = HashSet::new();
    for key in node_order {
        let Some(&entry) = reps.get(key) else {
            continue;
        };
        let label = label_for(categories, entry.source.line);
        let section = section_for(sections, categories, entry.source.line);
        let pos = position(key, label, section);
        record_section(&mut secs, &mut seen, &pos);
        leaves.push(Leaf {
            key: key.clone(),
            position: pos,
            entry,
        });
    }
    LeafInputs {
        leaves,
        sections: secs,
    }
}

/// Record a first-seen [`Section`] when `pos` is a synthetic section key.
fn record_section(secs: &mut Vec<Section>, seen: &mut HashSet<String>, pos: &str) {
    if let Some((parent, label)) = section_parts(pos)
        && seen.insert(pos.to_owned())
    {
        secs.push(Section {
            key: pos.to_owned(),
            parent: parent.to_owned(),
            label: label.to_owned(),
        });
    }
}

/// A synthetic section grouping node: no stroke of its own, just a labeled
/// drill-down whose children (filled by `apply_children`) are its members.
fn section_node(label: &str) -> Node {
    Node {
        terminal: false,
        multi_line: None,
        one_liner: None,
        children: Vec::new(),
        axis: None,
        description: Some(label.to_owned()),
        one_liner_root: None,
        synthetic: true,
    }
}

/// Create each section node and link it into its parent's children list, in
/// first-seen order. Runs before `apply_children` so the new nodes and parent
/// links are visible when children are copied in.
fn link_sections(
    node_order: &mut Vec<String>,
    nodes: &mut HashMap<String, Node>,
    children_of: &mut HashMap<String, Vec<String>>,
    sections: &[Section],
) {
    for sec in sections {
        if !nodes.contains_key(&sec.key) {
            node_order.push(sec.key.clone());
            nodes.insert(sec.key.clone(), section_node(&sec.label));
        }
        let kids = children_of.entry(sec.parent.clone()).or_default();
        if !kids.contains(&sec.key) {
            kids.push(sec.key.clone());
        }
    }
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
            synthetic: false,
        },
    );
}

/// Apply the full grouping pass: section nodes spliced in, children on every
/// leaf, synthetic axis nodes appended, and category roots resolved from the
/// same buckets.
pub fn apply_grouping(
    node_order: &mut Vec<String>,
    nodes: &mut HashMap<String, Node>,
    categories: &mut [Category],
    grouped: axis::Grouped,
    sections: &[Section],
) {
    let mut children_of = grouped.children_of;
    link_sections(node_order, nodes, &mut children_of, sections);
    apply_children(nodes, &children_of);
    for (axis_key, data) in grouped.axes {
        add_axis_node(node_order, nodes, axis_key, data);
    }
    for cat in categories {
        if let Some(roots) = children_of.get(&format!("cat:{}", cat.label)) {
            cat.roots.clone_from(roots);
        }
    }
}
