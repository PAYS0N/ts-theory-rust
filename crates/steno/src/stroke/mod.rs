//! Steno stroke mechanics for the count-expansion pass.
//!
//! A count bank "floats": when a construct's count is non-zero, the bank's
//! keys are merged into one sub-stroke and the result is re-rendered in
//! canonical English steno order. e.g. `STKWR-PBGS/-FLT` with 3 params ->
//! `STKWR-PBGS/AOFLT`.
//!
//! Canonical order (single stroke):
//! `# | S T K P W H R | A O * E U | F R P B L G T S D Z`.
//! The hyphen appears only to separate right-bank keys when there is no
//! middle (vowel or `*`) to do it. With a middle present, no hyphen.

mod bank;
mod keys;

pub use bank::{
    CountBank, CountBit, Side, add_key, apply_count, count_bank, merge_strokes, subtract_strokes,
};
pub use keys::{
    KeySet, LEFT_ORDER, MID_ORDER, RIGHT_ORDER, StrokeKeys, parse_stroke, render_stroke,
};
