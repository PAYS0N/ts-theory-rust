//! Generate a spread of stroke sequences that exercise the programmatic space
//! — every construct's minimal fill, deep nesting past any enumerable depth,
//! multi-arity generics, fused returns, partials, and negatives. The
//! [test header](super::test) runs the reference walker over these and records
//! its verdict, so the C++ dictionary is pinned to the walker (D9), not to a
//! table of pre-computed answers.

use super::super::{Construct, InfType};
use crate::stroke::merge_strokes;

/// Pull the type strokes the generator needs by arity.
struct Palette<'a> {
    /// Concrete (arity-0) type strokes, e.g. `number`, `string`.
    zero: Vec<&'a str>,
    /// First arity-1 generic stroke, if the corpus has one.
    one: Option<&'a str>,
    /// First arity-2 generic stroke, if the corpus has one.
    two: Option<&'a str>,
}

impl<'a> Palette<'a> {
    /// Collect the type strokes by arity, or `None` if there is no concrete type
    /// to fill an argument with.
    fn from(types: &'a [InfType]) -> Option<Self> {
        let zero: Vec<&str> = types
            .iter()
            .filter(|t| t.arity == 0)
            .map(|t| t.stroke.as_str())
            .collect();
        zero.first()?;
        Some(Self {
            zero,
            one: types
                .iter()
                .find(|t| t.arity == 1)
                .map(|t| t.stroke.as_str()),
            two: types
                .iter()
                .find(|t| t.arity == 2)
                .map(|t| t.stroke.as_str()),
        })
    }

    /// A concrete type stroke, cycling through the pool by index. The pool is
    /// non-empty by construction, so the fallback is never taken.
    fn concrete(&self, i: usize) -> String {
        self.zero
            .get(i % self.zero.len())
            .copied()
            .unwrap_or("")
            .to_owned()
    }
}

/// All sampled sequences for the corpus, or empty if it has no concrete type.
pub(super) fn sample_sequences(types: &[InfType], constructs: &[Construct]) -> Vec<Vec<String>> {
    let Some(p) = Palette::from(types) else {
        return Vec::new();
    };
    let mut seqs = Vec::new();
    for c in constructs {
        push_construct_fills(&mut seqs, c, &p);
    }
    push_deep(&mut seqs, constructs, &p);
    push_negatives(&mut seqs, constructs, &p);
    seqs
}

/// A construct's minimal terminal fill and its one-short partial.
fn push_construct_fills(seqs: &mut Vec<Vec<String>>, c: &Construct, p: &Palette) {
    let Some(seq) = terminal_fill(c, p) else {
        return;
    };
    if seq.len() > c.base.len() {
        let mut partial = seq.clone();
        partial.pop(); // drop the last fill -> partial
        seqs.push(partial);
    }
    seqs.push(seq);
}

/// Base strokes followed by one concrete type per slot (the fused slot, if any,
/// merged into the shape). `None` when a fused merge is impossible.
fn terminal_fill(c: &Construct, p: &Palette) -> Option<Vec<String>> {
    let mut seq = c.base.clone();
    for (k, _) in c.fill_order.iter().enumerate() {
        let concrete = p.concrete(k);
        if k == 0
            && let Some(shape) = &c.shape
        {
            seq.push(merge_strokes(&concrete, shape).ok()?);
        } else {
            seq.push(concrete);
        }
    }
    Some(seq)
}

/// Deep and multi-arity sequences hung off the first standalone emitter (a
/// non-fused, single-slot construct): nesting depth 1..=3, a partial, an
/// arity-2 map, and a fused generic return.
fn push_deep(seqs: &mut Vec<Vec<String>>, constructs: &[Construct], p: &Palette) {
    let Some(emitter) = constructs
        .iter()
        .find(|c| c.shape.is_none() && c.slots.len() == 1 && !c.base.is_empty())
    else {
        return;
    };
    if let Some(one) = p.one {
        for depth in 1..=3 {
            let mut seq = emitter.base.clone();
            seq.extend(std::iter::repeat_n(one.to_owned(), depth));
            seq.push(p.concrete(0));
            seqs.push(seq);
        }
        let mut partial = emitter.base.clone();
        partial.extend([one.to_owned(), one.to_owned()]); // ends mid-obligation
        seqs.push(partial);
    }
    if let Some(two) = p.two {
        let mut seq = emitter.base.clone();
        seq.extend([two.to_owned(), p.concrete(0), p.concrete(1)]);
        seqs.push(seq);
    }
    push_fused_generic(seqs, constructs, p);
}

/// A fused construct whose single return slot is a generic: base + merged
/// generic stroke + one concrete argument (e.g. `function (): Array<number>`).
fn push_fused_generic(seqs: &mut Vec<Vec<String>>, constructs: &[Construct], p: &Palette) {
    let (Some(one), Some(c)) = (
        p.one,
        constructs
            .iter()
            .find(|c| c.shape.is_some() && c.slots.len() == 1),
    ) else {
        return;
    };
    let Some(shape) = &c.shape else { return };
    let Some(merged) = merge_strokes(one, shape).ok() else {
        return;
    };
    let mut seq = c.base.clone();
    seq.push(merged);
    seq.push(p.concrete(0));
    seqs.push(seq);
}

/// An unmatched base and a completed fill with a dangling extra type stroke.
fn push_negatives(seqs: &mut Vec<Vec<String>>, constructs: &[Construct], p: &Palette) {
    seqs.push(vec!["STKPWHR".to_owned()]);
    if let Some(c) = constructs
        .iter()
        .find(|c| c.shape.is_none() && !c.base.is_empty())
        && let Some(mut seq) = terminal_fill(c, p)
    {
        seq.push(p.concrete(0)); // one type stroke too many
        seqs.push(seq);
    }
}
