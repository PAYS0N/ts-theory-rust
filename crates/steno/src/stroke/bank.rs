//! Count-bank encoding and stroke-merging arithmetic. A bank's keys carry
//! bit weights (LSB-first) so a numeric count merges into a sub-stroke as a
//! set of keys.

use super::keys::{KeySet, is_mid, is_right, parse_stroke, render_stroke};
use crate::error::StrokeError;

/// Which side of the board a count-bank key lives on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Middle bank (vowels / `*`).
    Mid,
    /// Right bank.
    Right,
}

/// One key of a count bank with its bit weight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountBit {
    /// The steno key carrying this bit.
    pub key: char,
    /// Bit weight: `2^i` for the i-th listed key, LSB-first.
    pub weight: u32,
    /// Bank the key belongs to.
    pub side: Side,
}

/// A parsed `@count` bank: keys with weights and the encodable maximum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountBank {
    /// Bank keys with their bit weights, lowest first.
    pub bits: Vec<CountBit>,
    /// Inclusive max count the bank can encode (`2^width - 1`).
    pub max: u32,
}

/// Build a count bank from an `@count` spec.
///
/// CONVENTION: the spec lists keys LSB-first, so the i-th listed key carries
/// weight `2^i` (AOEU => 1,2,4,8). A key is a vowel if it is in the middle
/// bank; otherwise it must be a right-bank key.
///
/// # Errors
/// Returns [`StrokeError`] when a spec key is outside the vowel/right banks
/// or the spec is too long to encode in 32 bits.
pub fn count_bank(spec: &str) -> Result<CountBank, StrokeError> {
    let mut bits = Vec::new();
    let mut weight = 1u32;
    for (i, key) in spec.chars().enumerate() {
        if !is_mid(key) && !is_right(key) {
            return Err(StrokeError::new(format!(
                "@count key \"{key}\" is not a vowel or right-bank key"
            )));
        }
        if i > 0 {
            weight = weight.checked_mul(2).ok_or_else(|| {
                StrokeError::new(format!("@count spec \"{spec}\" is too long to encode"))
            })?;
        }
        let side = if is_mid(key) { Side::Mid } else { Side::Right };
        bits.push(CountBit { key, weight, side });
    }
    let max = bits
        .last()
        .map_or(0, |b| b.weight.checked_mul(2).map_or(u32::MAX, |m| m - 1));
    Ok(CountBank { bits, max })
}

/// Union the keys of two sub-strokes into one.
///
/// # Errors
/// Returns [`StrokeError`] when either side fails to parse or the strokes
/// share a key.
pub fn merge_strokes(a: &str, b: &str) -> Result<String, StrokeError> {
    let mut ka = parse_stroke(a)?;
    let kb = parse_stroke(b)?;
    merge_bank(&mut ka.left, &kb.left, a, b)?;
    merge_bank(&mut ka.mid, &kb.mid, a, b)?;
    merge_bank(&mut ka.right, &kb.right, a, b)?;
    ka.num = ka.num || kb.num;
    Ok(render_stroke(&ka))
}

/// Copy `src` keys into `dst`, erroring on any key already present.
fn merge_bank(dst: &mut KeySet, src: &KeySet, a: &str, b: &str) -> Result<(), StrokeError> {
    for ch in src.keys() {
        if dst.contains(ch) {
            return Err(StrokeError::new(format!(
                "merge conflict on \"{ch}\" in \"{a}\" + \"{b}\""
            )));
        }
        dst.insert(ch);
    }
    Ok(())
}

/// Add a single key (vowel or right-bank) to a sub-stroke segment.
///
/// # Errors
/// Returns [`StrokeError`] when the segment fails to parse, the key is not
/// a vowel/right-bank key, or the key is already present.
pub fn add_key(segment: &str, key: char) -> Result<String, StrokeError> {
    let mut keys = parse_stroke(segment)?;
    let bank = if is_mid(key) {
        &mut keys.mid
    } else if is_right(key) {
        &mut keys.right
    } else {
        return Err(StrokeError::new(format!(
            "cannot add key \"{key}\" (not a vowel/right-bank key)"
        )));
    };
    if bank.contains(key) {
        return Err(StrokeError::new(format!(
            "key \"{key}\" already in \"{segment}\""
        )));
    }
    bank.insert(key);
    Ok(render_stroke(&keys))
}

/// Merge the keys encoding `count` (per `spec`) into one sub-stroke segment.
///
/// # Errors
/// Returns [`StrokeError`] when the count exceeds the bank's maximum or any
/// count key is already present in the segment.
pub fn apply_count(segment: &str, spec: &str, count: u32) -> Result<String, StrokeError> {
    let bank = count_bank(spec)?;
    if count > bank.max {
        return Err(StrokeError::new(format!(
            "count {count} out of range for bank \"{spec}\" (0..{})",
            bank.max
        )));
    }
    let mut keys = parse_stroke(segment)?;
    for bit in &bank.bits {
        if count & bit.weight == 0 {
            continue;
        }
        let target = match bit.side {
            Side::Mid => &mut keys.mid,
            Side::Right => &mut keys.right,
        };
        if target.contains(bit.key) {
            return Err(StrokeError::new(format!(
                "count key \"{}\" already present in \"{segment}\"",
                bit.key
            )));
        }
        target.insert(bit.key);
    }
    Ok(render_stroke(&keys))
}
