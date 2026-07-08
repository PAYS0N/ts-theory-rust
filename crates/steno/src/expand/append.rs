//! Pass B — type-append.
//!
//! A construct's single `%t` is filled by appending type strokes. Arity-0
//! types terminate immediately; an arity-N generic stays NON-TERMINAL until N
//! args are appended. Non-terminal steps accumulate the type as bracketless
//! space-joined tokens ("Map", "Map string"); the terminal step renders the
//! real brackets ("Map<string, number>"). Nesting (a generic as a generic's
//! arg) is out of scope, so generic args are drawn from a restricted arity-0
//! pool.

use super::fill::{Filling, fill_template, fillings, render_type, with_types};
use super::types::{TypeDef, TypeOptions};
use super::{ExpandedEntry, TypedEntry};
use crate::error::ExpandError;
use crate::parse::Chunk;
use crate::stroke::merge_strokes;

/// Build a [`TypedEntry`] step from a count-expanded entry.
fn typed(
    entry: &ExpandedEntry,
    stroke: String,
    template: Vec<Chunk>,
    terminal: bool,
    type_label: Option<String>,
) -> TypedEntry {
    TypedEntry {
        stroke,
        template,
        terminal,
        one_liner: false,
        count: entry.count,
        type_label,
        source: entry.source.clone(),
    }
}

/// Emit the multi-slot entry for one non-empty prefix of pool choices.
fn emit_multi(
    entry: &ExpandedEntry,
    slots: &[usize],
    chosen: &[&TypeDef],
    out: &mut Vec<TypedEntry>,
) {
    let texts: Vec<String> = slots
        .iter()
        .enumerate()
        .map(|(i, _)| {
            chosen
                .get(i)
                .map_or_else(String::new, |t| render_type(t, &[]))
        })
        .collect();
    let suffix: Vec<&str> = chosen.iter().map(|t| t.stroke.as_str()).collect();
    let stroke = format!("{}/{}", entry.stroke, suffix.join("/"));
    let template = with_types(&entry.template, slots, &texts);
    let type_label = chosen.last().map(|t| t.text.clone());
    out.push(typed(
        entry,
        stroke,
        template,
        chosen.len() == slots.len(),
        type_label,
    ));
}

/// Recursion for the multi-slot fill: emit every non-empty prefix of pool
/// choices.
fn rec_multi(
    entry: &ExpandedEntry,
    slots: &[usize],
    pool: &[TypeDef],
    chosen: &[&TypeDef],
    out: &mut Vec<TypedEntry>,
) {
    if !chosen.is_empty() {
        emit_multi(entry, slots, chosen, out);
    }
    if chosen.len() < slots.len() {
        for t in pool {
            let mut next = chosen.to_vec();
            next.push(t);
            rec_multi(entry, slots, pool, &next, out);
        }
    }
}

/// Multi-slot fill (typed params): every `%t` slot gets an arity-0 type from
/// the pool, appended one per slot. All slots must be filled to terminate.
fn expand_multi_slot(entry: &ExpandedEntry, slots: &[usize], pool: &[TypeDef]) -> Vec<TypedEntry> {
    let empty: Vec<String> = slots.iter().map(|_| String::new()).collect();
    let base = with_types(&entry.template, slots, &empty);
    let mut out = vec![typed(entry, entry.stroke.clone(), base, false, None)];
    rec_multi(entry, slots, pool, &[], &mut out);
    out
}

/// Fuse path: merge the construct's last segment (the shape selector) into
/// the first appended type stroke, and drop the type-less base entirely.
fn expand_fused(
    entry: &ExpandedEntry,
    slot: usize,
    opts: &TypeOptions,
) -> Result<Vec<TypedEntry>, ExpandError> {
    let mut segs: Vec<&str> = entry.stroke.split('/').collect();
    let shape = segs.pop().unwrap_or_default();
    let mut out = Vec::new();
    for f in fillings(opts) {
        let Some((head, rest)) = f.suffix.split_first() else {
            continue;
        };
        let first = merge_strokes(head, shape)?;
        let mut stroke_segs = segs.clone();
        stroke_segs.push(&first);
        stroke_segs.extend(rest.iter().map(String::as_str));
        out.push(typed(
            entry,
            stroke_segs.join("/"),
            fill_template(&entry.template, slot, &f),
            f.terminal,
            Some(f.text.clone()),
        ));
    }
    Ok(out)
}

/// Append one filling step to an unfused construct.
fn appended(entry: &ExpandedEntry, slot: usize, f: &Filling) -> TypedEntry {
    typed(
        entry,
        format!("{}/{}", entry.stroke, f.suffix.join("/")),
        fill_template(&entry.template, slot, f),
        f.terminal,
        Some(f.text.clone()),
    )
}

/// Indices of every `%t` slot in a template.
fn type_slots(template: &[Chunk]) -> Vec<usize> {
    template
        .iter()
        .enumerate()
        .filter_map(|(i, c)| matches!(c, Chunk::TypeSlot).then_some(i))
        .collect()
}

/// Build the type-append chain(s) for one count-expanded entry.
///
/// # Errors
/// Returns [`ExpandError`] when a fuse merge hits a shared key.
pub fn expand_types_one(
    entry: &ExpandedEntry,
    opts: &TypeOptions,
) -> Result<Vec<TypedEntry>, ExpandError> {
    let slots = type_slots(&entry.template);
    let Some(&slot) = slots.first() else {
        return Ok(vec![typed(
            entry,
            entry.stroke.clone(),
            entry.template.clone(),
            true,
            None,
        )]);
    };
    if slots.len() > 1 {
        return Ok(expand_multi_slot(entry, &slots, &opts.generic_args));
    }
    if entry.source.flags.fuse() {
        return expand_fused(entry, slot, opts);
    }
    // base: the skeleton before any type is appended (non-terminal)
    let base = with_types(&entry.template, &[slot], &[String::new()]);
    let mut out = vec![typed(entry, entry.stroke.clone(), base, false, None)];
    for f in fillings(opts) {
        out.push(appended(entry, slot, &f));
    }
    Ok(out)
}

/// Apply [`expand_types_one`] to every count-expanded entry.
///
/// # Errors
/// Returns [`ExpandError`] on the first entry that fails.
pub fn expand_types(
    entries: &[ExpandedEntry],
    opts: &TypeOptions,
) -> Result<Vec<TypedEntry>, ExpandError> {
    let mut out = Vec::new();
    for e in entries {
        out.extend(expand_types_one(e, opts)?);
    }
    Ok(out)
}
