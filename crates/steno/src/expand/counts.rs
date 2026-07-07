//! Pass A — count expansion.
//!
//! Each `@count` entry fans out to one expanded entry per count value
//! (0..=bank.max): the bank keys are merged into the stroke, the
//! `%[ sep | body %]` repeat is run `count` times, and `%d` / `%(EXPR)` are
//! resolved. Entries without `@count` pass through unchanged. Type-slots
//! (`%t`), body-breaks (`%b`), braces and landings are left for later passes.
//!
//! Scope of `d`: total count outside a repeat, iteration index inside one.
//! The iteration index is 0-based ([`ITERATION_BASE`]) — a one-line knob.

use super::ExpandedEntry;
use crate::error::ExpandError;
use crate::parse::{Chunk, Entry};
use crate::stroke::{apply_count, count_bank};

/// Base of the iteration index inside a repeat (0-based).
pub const ITERATION_BASE: u32 = 0;

/// True when any top-level chunk consumes the count.
fn uses_count(chunks: &[Chunk]) -> bool {
    chunks
        .iter()
        .any(|c| matches!(c, Chunk::Repeat { .. } | Chunk::Dcount | Chunk::Computed(_)))
}

/// Resolve one computed landing `a*d + b` at the in-scope `d`.
fn resolve_computed(a: i32, b: i32, scope_d: u32) -> Result<Chunk, ExpandError> {
    let n = i64::from(a) * i64::from(scope_d) + i64::from(b);
    let landing = u32::try_from(n).map_err(|_| {
        ExpandError::new(format!(
            "computed landing resolves to {n} (< 0) at d={scope_d}"
        ))
    })?;
    Ok(Chunk::Landing(landing))
}

/// Resolve `%d`, `%(EXPR)`, and repeats against the in-scope `d` (`scope_d`)
/// and the total count (`total`).
fn resolve(chunks: &[Chunk], scope_d: u32, total: u32) -> Result<Vec<Chunk>, ExpandError> {
    let mut out = Vec::new();
    for c in chunks {
        match c {
            Chunk::Dcount => out.push(Chunk::Lit(scope_d.to_string())),
            Chunk::Computed(expr) => out.push(resolve_computed(expr.a, expr.b, scope_d)?),
            Chunk::Repeat { sep, body } => {
                for i in 0..total {
                    let iter_d = ITERATION_BASE + i;
                    if i > 0 {
                        out.extend(resolve(sep, iter_d, total)?);
                    }
                    out.extend(resolve(body, iter_d, total)?);
                }
            },
            other => out.push(other.clone()),
        }
    }
    Ok(out)
}

/// Expand one entry over its count bank (or pass through if it has no
/// `@count`).
///
/// # Errors
/// Returns [`ExpandError`] when `@count` and count operators are mismatched,
/// the stroke is empty, or a computed landing resolves negative.
pub fn expand_counts(entry: &Entry) -> Result<Vec<ExpandedEntry>, ExpandError> {
    let uses = uses_count(&entry.template);
    let Some(spec) = entry.count.as_deref() else {
        return pass_through(entry, uses);
    };
    if !uses {
        return Err(ExpandError::new(format!(
            "\"{}\" has @count but no count operator in its template",
            entry.stroke_raw
        )));
    }
    fan_out(entry, spec)
}

/// Pass a non-count entry through unchanged (or reject a stray operator).
fn pass_through(entry: &Entry, uses: bool) -> Result<Vec<ExpandedEntry>, ExpandError> {
    if uses {
        return Err(ExpandError::new(format!(
            "\"{}\" uses a count operator but has no @count",
            entry.stroke_raw
        )));
    }
    Ok(vec![ExpandedEntry {
        stroke: entry.stroke_raw.clone(),
        template: entry.template.clone(),
        count: None,
        source: entry.clone(),
    }])
}

/// Fan one `@count` entry out to an expansion per count value.
fn fan_out(entry: &Entry, spec: &str) -> Result<Vec<ExpandedEntry>, ExpandError> {
    let Some((last, front)) = entry.stroke.split_last() else {
        return Err(ExpandError::new(format!(
            "\"{}\" has an empty stroke",
            entry.stroke_raw
        )));
    };
    let bank = count_bank(spec)?;
    let mut out = Vec::new();
    for d in 0..=bank.max {
        let merged = apply_count(last, spec, d)?;
        let mut segs: Vec<&str> = front.iter().map(String::as_str).collect();
        segs.push(&merged);
        out.push(ExpandedEntry {
            stroke: segs.join("/"),
            template: resolve(&entry.template, d, d)?,
            count: Some(d),
            source: entry.clone(),
        });
    }
    Ok(out)
}

/// Expand a whole parsed dictionary through Pass A.
///
/// # Errors
/// Returns [`ExpandError`] on the first entry that fails to expand.
pub fn expand_all(entries: &[Entry]) -> Result<Vec<ExpandedEntry>, ExpandError> {
    let mut out = Vec::new();
    for e in entries {
        out.extend(expand_counts(e)?);
    }
    Ok(out)
}
