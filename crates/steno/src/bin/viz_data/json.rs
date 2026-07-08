//! Serializes a [`Tree`] to the JSON shape `viz/index.html` expects:
//! `{"categories": [{"label", "roots"}, ...], "nodes": {stroke: {"terminal",
//! "multiLine", "oneLiner", "children", "axis", "description",
//! "oneLinerRoot"}, ...}}`. Hand-rolled (no
//! `serde`, per the crate's zero-runtime-deps invariant), reusing
//! `steno::json_string`'s escaping.

use std::collections::HashMap;

use steno::json_string;

use crate::axis::{Axis, Choice};
use crate::tree::{Category, Node, Tree};

/// Serialize the tree.
#[must_use]
pub fn to_json(tree: &Tree) -> String {
    let mut out = String::from("{\n\"categories\": [\n");
    write_categories(&mut out, &tree.categories);
    out.push_str("],\n\"nodes\": {\n");
    write_nodes(&mut out, &tree.node_order, &tree.nodes);
    out.push_str("}\n}\n");
    out
}

/// Write every category object, comma-separated, one per line.
fn write_categories(out: &mut String, categories: &[Category]) {
    for (i, cat) in categories.iter().enumerate() {
        out.push_str("{\"label\": ");
        out.push_str(&json_string(&cat.label));
        out.push_str(", \"roots\": ");
        write_str_array(out, &cat.roots);
        out.push('}');
        if i + 1 < categories.len() {
            out.push(',');
        }
        out.push('\n');
    }
}

/// Write a JSON array of plain strings.
fn write_str_array(out: &mut String, items: &[String]) {
    out.push('[');
    for (i, item) in items.iter().enumerate() {
        out.push_str(&json_string(item));
        if i + 1 < items.len() {
            out.push_str(", ");
        }
    }
    out.push(']');
}

/// Write every node object, comma-separated, one per line, in first-seen
/// order.
fn write_nodes(out: &mut String, node_order: &[String], nodes: &HashMap<String, Node>) {
    for (i, key) in node_order.iter().enumerate() {
        if let Some(node) = nodes.get(key) {
            write_node(out, key, node);
        }
        if i + 1 < node_order.len() {
            out.push(',');
        }
        out.push('\n');
    }
}

/// Write one `"stroke": {...}` node entry.
fn write_node(out: &mut String, key: &str, node: &Node) {
    out.push_str(&json_string(key));
    out.push_str(": {\"terminal\": ");
    out.push_str(if node.terminal { "true" } else { "false" });
    out.push_str(", \"multiLine\": ");
    write_opt_string(out, node.multi_line.as_deref());
    out.push_str(", \"oneLiner\": ");
    write_opt_string(out, node.one_liner.as_deref());
    out.push_str(", \"children\": ");
    write_str_array(out, &node.children);
    out.push_str(", \"axis\": ");
    write_opt_axis(out, node.axis.as_ref());
    out.push_str(", \"description\": ");
    write_opt_string(out, node.description.as_deref());
    out.push_str(", \"oneLinerRoot\": ");
    write_opt_string(out, node.one_liner_root.as_deref());
    out.push('}');
}

/// Write `null` or a JSON string.
fn write_opt_string(out: &mut String, value: Option<&str>) {
    match value {
        Some(s) => out.push_str(&json_string(s)),
        None => out.push_str("null"),
    }
}

/// Write `null` or an axis object: `{"kind", "options": [{"label", "key"}]}`.
fn write_opt_axis(out: &mut String, axis: Option<&Axis>) {
    let Some(axis) = axis else {
        out.push_str("null");
        return;
    };
    out.push_str("{\"kind\": ");
    out.push_str(&json_string(axis.kind.as_str()));
    out.push_str(", \"options\": [");
    for (i, choice) in axis.choices.iter().enumerate() {
        write_choice(out, choice);
        if i + 1 < axis.choices.len() {
            out.push_str(", ");
        }
    }
    out.push_str("]}");
}

/// Write one `{"label", "key"}` axis option.
fn write_choice(out: &mut String, choice: &Choice) {
    out.push_str("{\"label\": ");
    out.push_str(&json_string(&choice.label));
    out.push_str(", \"key\": ");
    out.push_str(&json_string(&choice.key));
    out.push('}');
}
