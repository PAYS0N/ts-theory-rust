//! The programmatic (`dict.infinite.steno`) path: build two small tables from
//! Pass A output — a **type table** and a **construct table** — and never run
//! Pass B. The obligation-stack [reference walker](walk) replays the
//! type-append that Pass B would have enumerated, at a cost proportional to
//! the strokes pressed rather than `O(T^s)`. See `docs/pipeline.md`.

mod emit;
mod walk;

pub use emit::{emit_data_header, emit_test_header};
pub use walk::{WalkResult, render_filled, template_fragments, walk};

use std::collections::HashMap;

use super::counts::expand_counts;
use crate::error::ExpandError;
use crate::parse::{Chunk, Entry};
use crate::stroke::merge_strokes;

/// One record of the **type table**: an appendable `@type`, with its stroke,
/// generic arity, and text carrying `%t` argument markers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfType {
    /// The type's own stroke, e.g. `AR`.
    pub stroke: String,
    /// Generic-argument count (0 = concrete).
    pub arity: u32,
    /// Rendered text with `%t` arg markers, e.g. `Array<%t>` or `number`.
    pub text: String,
}

/// One record of the **construct table**: a count-resolved Pass-A output with
/// zero enumerated rows — just its base stroke, its `%t`/`%T` slot positions,
/// and the order strokes fill them (D12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Construct {
    /// Base stroke segments matched as a prefix. For a fused construct this is
    /// the stroke minus its last (shape) segment.
    pub base: Vec<String>,
    /// The fuse shape segment (the last stroke segment). `Some` iff `@fuse`
    /// applies; the fuse-target type stroke merges into it.
    pub shape: Option<String>,
    /// Count-resolved template with `TypeSlot`/`FusedTypeSlot` chunks intact.
    pub template: Vec<Chunk>,
    /// Template indices of the type slots, in template order.
    pub slots: Vec<usize>,
    /// Slot indices (into `slots`) in the order strokes fill them. For a fused
    /// construct the fuse-target slot leads; otherwise it is template order.
    pub fill_order: Vec<usize>,
    /// The full Pass-A stroke (diagnostics and the ambiguity check).
    pub source_stroke: String,
}

/// Render a `@type` template to its text form (only `Lit` and `%t` are legal;
/// `%T` is a construct-only marker).
fn render_type_text(template: &[Chunk]) -> Result<String, ExpandError> {
    let mut out = String::new();
    for c in template {
        match c {
            Chunk::Lit(text) => out.push_str(text),
            Chunk::TypeSlot => out.push_str("%t"),
            other => {
                return Err(ExpandError::new(format!(
                    "@type entry in the infinite corpus has an unexpected chunk \"{}\"",
                    other.kind_name()
                )));
            },
        }
    }
    Ok(out)
}

/// Collect the `@type` entries into the type table.
///
/// # Errors
/// Returns [`ExpandError`] when a `@type` template holds anything but literals
/// and `%t` markers.
pub fn build_types(entries: &[Entry]) -> Result<Vec<InfType>, ExpandError> {
    entries
        .iter()
        .filter(|e| e.flags.is_type())
        .map(|e| {
            Ok(InfType {
                stroke: e.stroke_raw.clone(),
                arity: e.arity.unwrap_or(0),
                text: render_type_text(&e.template)?,
            })
        })
        .collect()
}

/// True for a type slot (`%t` or the fuse-target `%T`).
const fn is_slot(c: &Chunk) -> bool {
    matches!(c, Chunk::TypeSlot | Chunk::FusedTypeSlot)
}

/// Template indices of the type slots, in template order.
fn slot_positions(template: &[Chunk]) -> Vec<usize> {
    template
        .iter()
        .enumerate()
        .filter(|(_, c)| is_slot(c))
        .map(|(i, _)| i)
        .collect()
}

/// The D12 fill order for a fused construct: the fuse-target slot first (its
/// stroke is the first after the base), then the rest in template order.
fn fused_fill_order(template: &[Chunk], slots: &[usize]) -> Vec<usize> {
    let fuse_slot = slots
        .iter()
        .position(|&pos| matches!(template.get(pos), Some(Chunk::FusedTypeSlot)))
        .unwrap_or(0);
    let mut order = vec![fuse_slot];
    order.extend((0..slots.len()).filter(|&k| k != fuse_slot));
    order
}

/// Build one construct record from a count-resolved entry.
fn build_construct(e: &super::ExpandedEntry) -> Construct {
    let slots = slot_positions(&e.template);
    let mut segs: Vec<String> = e.stroke.split('/').map(str::to_owned).collect();
    let (shape, fill_order) = if e.source.flags.fuse() && !slots.is_empty() {
        (segs.pop(), fused_fill_order(&e.template, &slots))
    } else {
        (None, (0..slots.len()).collect())
    };
    Construct {
        base: segs,
        shape,
        template: e.template.clone(),
        slots,
        fill_order,
        source_stroke: e.stroke.clone(),
    }
}

/// Build the construct table: run Pass A over every non-`@type` entry, then
/// record each count-resolved output as one construct (no Pass B).
///
/// # Errors
/// Returns [`ExpandError`] on any Pass A (count) misconfiguration.
pub fn build_constructs(entries: &[Entry]) -> Result<Vec<Construct>, ExpandError> {
    let mut out = Vec::new();
    for e in entries.iter().filter(|e| !e.flags.is_type()) {
        for expanded in expand_counts(e)? {
            out.push(build_construct(&expanded));
        }
    }
    Ok(out)
}

/// Build both tables for the programmatic corpus.
///
/// # Errors
/// Returns [`ExpandError`] on a bad `@type` template or Pass A failure.
pub fn build_tables(entries: &[Entry]) -> Result<(Vec<InfType>, Vec<Construct>), ExpandError> {
    Ok((build_types(entries)?, build_constructs(entries)?))
}

/// Maps a `(base, merged-stroke)` key to the `(shape, type-head)` pair that
/// produced it, so a second pair reaching the same stroke is a collision.
type FuseSeen = HashMap<(Vec<String>, String), (String, String)>;

/// Record one fused construct's `(shape, type)` merges, erroring on a clash.
fn record_construct_fuse(
    c: &Construct,
    types: &[InfType],
    seen: &mut FuseSeen,
) -> Result<(), ExpandError> {
    let Some(shape) = c.shape.as_ref() else {
        return Ok(());
    };
    for t in types {
        let Ok(merged) = merge_strokes(&t.stroke, shape) else {
            continue;
        };
        let pair = (shape.clone(), t.stroke.clone());
        match seen.insert((c.base.clone(), merged.clone()), pair.clone()) {
            Some(prev) if prev != pair => {
                return Err(ExpandError::new(format!(
                    "fuse ambiguity: shape \"{}\"+type \"{}\" and shape \"{}\"+type \"{}\" \
                     both fuse to \"{}\"",
                    prev.0, prev.1, pair.0, pair.1, merged
                )));
            },
            _ => {},
        }
    }
    Ok(())
}

/// Build-time **fuse ambiguity check** (criterion 5).
///
/// No two `(construct shape, type head)` pairs may merge to the same stroke
/// under the same base. Enumeration used to catch this as an output collision;
/// the replay emits no rows, so it must be checked explicitly or an ambiguous
/// fuse becomes a silent wrong lookup. `O(constructs × types)`.
///
/// # Errors
/// Returns [`ExpandError`] naming the two colliding pairs.
pub fn check_fuse_ambiguity(
    types: &[InfType],
    constructs: &[Construct],
) -> Result<(), ExpandError> {
    let mut seen = FuseSeen::new();
    for c in constructs {
        record_construct_fuse(c, types, &mut seen)?;
    }
    Ok(())
}
