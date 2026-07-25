//! The construct-template op-list the C++ data header emits. Unlike the old
//! flat `template_fragments` (which dropped landings, newlines, and braces),
//! an op-list preserves every position the firmware needs to reproduce plain
//! movement, smart closer-dropping, and nvim tabstops. Adjacent surface text
//! coalesces; a type slot / landing / hard newline each break the run.

use super::super::is_slot;
use crate::parse::Chunk;

/// One unit of a construct template, in template order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateOp {
    /// Literal surface text (coalesced `Lit`/`Brace`/`Tab` characters).
    Text(String),
    /// A type slot (`%t` or `%T`), filled at walk time (template order).
    Slot,
    /// An editor landing / LSP tabstop position.
    Landing(u32),
    /// A hard line break (`Newline` or `%b`; the infinite path never runs
    /// one-liner, so `%b` always breaks).
    Newline,
}

/// Append surface text, coalescing into the last `Text` op.
fn push_text(ops: &mut Vec<TemplateOp>, s: &str) {
    if let Some(TemplateOp::Text(last)) = ops.last_mut() {
        last.push_str(s);
    } else {
        ops.push(TemplateOp::Text(s.to_owned()));
    }
}

/// Lower a construct template (slots intact) to the op-list the C++ header stores.
///
/// `n` slots yield `n` `Slot` ops in template order; the C++ fills the k-th
/// `Slot` with the k-th template-order slot text (see `walk_construct`).
#[must_use]
pub fn template_ops(template: &[Chunk]) -> Vec<TemplateOp> {
    let mut ops = Vec::new();
    for c in template {
        if is_slot(c) {
            ops.push(TemplateOp::Slot);
            continue;
        }
        match c {
            Chunk::Landing(n) => ops.push(TemplateOp::Landing(*n)),
            Chunk::Newline | Chunk::BodyBreak => ops.push(TemplateOp::Newline),
            Chunk::Lit(s) => push_text(&mut ops, s),
            Chunk::Brace { open: true } => push_text(&mut ops, "{"),
            Chunk::Brace { open: false } => push_text(&mut ops, "}"),
            Chunk::Tab => push_text(&mut ops, "\t"),
            // Unresolved chunks never survive Pass A into a construct template.
            _ => {},
        }
    }
    ops
}
