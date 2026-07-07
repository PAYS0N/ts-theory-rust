//! Type-filling machinery for Pass B: enumerating append steps and
//! substituting rendered type text into templates.

use super::types::{TypeDef, TypeOptions};
use crate::parse::Chunk;

/// The bracketless head of a type ("Map" from "Map<%t, %t>").
fn type_name(t: &TypeDef) -> &str {
    t.text.split_once('<').map_or(t.text.as_str(), |(h, _)| h)
}

/// Substitute `args` into the type's `%t` markers (missing args -> empty).
pub(super) fn render_type(t: &TypeDef, args: &[String]) -> String {
    let mut out = String::new();
    for (i, part) in t.text.split("%t").enumerate() {
        if i > 0 {
            out.push_str(args.get(i - 1).map_or("", String::as_str));
        }
        out.push_str(part);
    }
    out
}

/// The bracketless partial form: "Map" or "Map string".
fn partial_type(t: &TypeDef, args: &[String]) -> String {
    if args.is_empty() {
        type_name(t).to_string()
    } else {
        format!("{} {}", type_name(t), args.join(" "))
    }
}

/// One step of a type-append chain: the strokes to append, the rendered type
/// text so far, and whether the step terminates the entry.
pub(super) struct Filling {
    /// Appended stroke segments, in order.
    pub suffix: Vec<String>,
    /// Rendered type text for this step.
    pub text: String,
    /// True when the chain is complete at this step.
    pub terminal: bool,
    /// True for the free/custom (SKP) type.
    pub free_type: bool,
}

/// Convenience constructor for a non-free filling.
const fn filling(suffix: Vec<String>, text: String, terminal: bool) -> Filling {
    Filling {
        suffix,
        text,
        terminal,
        free_type: false,
    }
}

/// Recursively enumerate arg fillings for a generic type.
fn fill_args(
    t: &TypeDef,
    suffix: &[String],
    args: &[String],
    pool: &[TypeDef],
    out: &mut Vec<Filling>,
) {
    for a in pool {
        let mut sfx = suffix.to_vec();
        sfx.push(a.stroke.clone());
        let mut ar = args.to_vec();
        ar.push(render_type(a, &[]));
        if u32::try_from(ar.len()).is_ok_and(|n| n == t.arity) {
            out.push(filling(sfx, render_type(t, &ar), true));
        } else {
            out.push(filling(sfx.clone(), partial_type(t, &ar), false));
            fill_args(t, &sfx, &ar, pool, out);
        }
    }
}

/// Enumerate every append step reachable from the type set.
pub(super) fn fillings(opts: &TypeOptions) -> Vec<Filling> {
    let mut out = Vec::new();
    for t in &opts.types {
        if t.free_type {
            out.push(Filling {
                suffix: vec![t.stroke.clone()],
                text: String::new(),
                terminal: true,
                free_type: true,
            });
        } else if t.arity == 0 {
            out.push(filling(vec![t.stroke.clone()], render_type(t, &[]), true));
        } else {
            out.push(filling(vec![t.stroke.clone()], partial_type(t, &[]), false));
            fill_args(
                t,
                std::slice::from_ref(&t.stroke),
                &[],
                &opts.generic_args,
                &mut out,
            );
        }
    }
    out
}

/// Replace the chunks at `slots` with literal `texts` (paired by position).
pub(super) fn with_types(template: &[Chunk], slots: &[usize], texts: &[String]) -> Vec<Chunk> {
    let mut out = template.to_vec();
    for (idx, text) in slots.iter().zip(texts) {
        if let Some(slot) = out.get_mut(*idx) {
            *slot = Chunk::Lit(text.clone());
        }
    }
    out
}

/// Free-type fill: replace the type slot with a numbered tabstop above all
/// existing landings, so the plain profile still lands on `%0` (the name) but
/// the snippet profile gets a stop at the type. The `: ` stays.
fn with_free_type(template: &[Chunk], slot: usize) -> Vec<Chunk> {
    let max = template
        .iter()
        .filter_map(|c| match c {
            Chunk::Landing(n) => Some(*n),
            _ => None,
        })
        .max();
    let mut out = template.to_vec();
    if let Some(s) = out.get_mut(slot) {
        *s = Chunk::Landing(max.map_or(0, |m| m + 1));
    }
    out
}

/// Apply one filling to the template's single type slot.
pub(super) fn fill_template(template: &[Chunk], slot: usize, f: &Filling) -> Vec<Chunk> {
    if f.free_type {
        with_free_type(template, slot)
    } else {
        with_types(template, &[slot], std::slice::from_ref(&f.text))
    }
}
