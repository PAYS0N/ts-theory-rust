//! Render the two Pass-A tables into a self-contained C++ header for the
//! `javelin-ext` replay dictionary (criterion 4), plus a companion golden
//! header pinning that dictionary to the Rust reference [walker](super::walk)
//! (D9). No enumerated rows are emitted from either: the data header carries
//! rules, the test header carries walker verdicts on sampled sequences.

mod data;
mod sample;
mod test;

pub use data::emit_data_header;
pub use test::emit_test_header;

use crate::error::ExpandError;
use crate::stroke::{LEFT_ORDER, MID_ORDER, RIGHT_ORDER, StrokeKeys, parse_stroke};

/// Encode one sub-stroke into the javelin `StenoStroke` 32-bit key mask.
///
/// The bit layout is javelin's `StrokeBitIndex`: `#`=0, left `S..R`=1..7,
/// middle `A O * E U`=8..12, right `F R P B L G T S D Z`=13..22. Our canonical
/// bank orders ([`LEFT_ORDER`]/[`MID_ORDER`]/[`RIGHT_ORDER`]) list the keys in
/// exactly that sequence, so a bank index maps to a contiguous bit run.
///
/// # Errors
/// Returns [`ExpandError`] when the stroke fails to parse.
pub(super) fn stroke_mask(stroke: &str) -> Result<u32, ExpandError> {
    let keys: StrokeKeys = parse_stroke(stroke).map_err(|e| ExpandError::new(e.to_string()))?;
    let mut mask = u32::from(keys.num);
    bank_bits(&keys, &mut mask);
    Ok(mask)
}

/// Set the left/middle/right key bits of `keys` into `mask`. Each bank's keys
/// occupy consecutive bits starting at its base index.
fn bank_bits(keys: &StrokeKeys, mask: &mut u32) {
    for (order, set, base) in [
        (LEFT_ORDER.as_slice(), &keys.left, 1u32),
        (MID_ORDER.as_slice(), &keys.mid, 8),
        (RIGHT_ORDER.as_slice(), &keys.right, 13),
    ] {
        for (offset, key) in order.iter().enumerate() {
            if set.contains(*key) {
                *mask |= 1 << (base + u32::try_from(offset).unwrap_or(0));
            }
        }
    }
}

/// Escape a string into the body of a C string literal (no surrounding
/// quotes). Only the characters the corpus can produce are handled.
pub(super) fn c_escape(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}
