//! Sub-stroke parsing and canonical rendering.

use crate::error::StrokeError;

/// Left-bank keys in canonical steno order.
pub const LEFT_ORDER: [char; 7] = ['S', 'T', 'K', 'P', 'W', 'H', 'R'];
/// Middle keys (vowels and `*`) in canonical steno order.
pub const MID_ORDER: [char; 5] = ['A', 'O', '*', 'E', 'U'];
/// Right-bank keys in canonical steno order.
pub const RIGHT_ORDER: [char; 10] = ['F', 'R', 'P', 'B', 'L', 'G', 'T', 'S', 'D', 'Z'];

/// True when `ch` is a middle (vowel or `*`) key.
pub fn is_mid(ch: char) -> bool {
    MID_ORDER.contains(&ch)
}

/// True when `ch` is a left-bank key.
pub fn is_left(ch: char) -> bool {
    LEFT_ORDER.contains(&ch)
}

/// True when `ch` is a right-bank key.
pub fn is_right(ch: char) -> bool {
    RIGHT_ORDER.contains(&ch)
}

/// An unordered set of keys within one bank. Rendering re-imposes canonical
/// order, so insertion order never matters; duplicates collapse silently.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeySet {
    /// The distinct keys present, in insertion order.
    keys: Vec<char>,
}

impl KeySet {
    /// True when `ch` is present.
    #[must_use]
    pub fn contains(&self, ch: char) -> bool {
        self.keys.contains(&ch)
    }

    /// Add `ch`; a duplicate is a no-op (set semantics).
    pub fn insert(&mut self, ch: char) {
        if !self.contains(ch) {
            self.keys.push(ch);
        }
    }

    /// Iterate the keys in insertion order.
    pub fn keys(&self) -> impl Iterator<Item = char> + '_ {
        self.keys.iter().copied()
    }
}

/// One sub-stroke decomposed into its canonical key sets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StrokeKeys {
    /// Number-bar flag (`#` prefix).
    pub num: bool,
    /// Left-bank keys.
    pub left: KeySet,
    /// Middle (vowel/`*`) keys.
    pub mid: KeySet,
    /// Right-bank keys.
    pub right: KeySet,
}

/// Parse one sub-stroke (no `/`) into its canonical key sets.
///
/// # Errors
/// Returns [`StrokeError`] on a second hyphen or any key outside the bank
/// it appears in.
pub fn parse_stroke(s: &str) -> Result<StrokeKeys, StrokeError> {
    let mut keys = StrokeKeys::default();
    let mut body = s;
    if let Some(rest) = body.strip_prefix('#') {
        keys.num = true;
        body = rest;
    }
    if let Some((before, after)) = body.split_once('-') {
        parse_hyphenated(s, before, after, &mut keys)?;
    } else {
        parse_unhyphenated(s, body, &mut keys)?;
    }
    Ok(keys)
}

/// Fill `keys` from a hyphenated sub-stroke: left/mid keys before the hyphen,
/// right-bank keys after it.
fn parse_hyphenated(
    orig: &str,
    before: &str,
    after: &str,
    keys: &mut StrokeKeys,
) -> Result<(), StrokeError> {
    if after.contains('-') {
        return Err(StrokeError::new(format!("two hyphens in \"{orig}\"")));
    }
    for ch in before.chars() {
        insert_left_mid(orig, ch, keys)?;
    }
    for ch in after.chars() {
        if !is_right(ch) {
            return Err(StrokeError::new(format!(
                "bad right key \"{ch}\" in \"{orig}\""
            )));
        }
        keys.right.insert(ch);
    }
    Ok(())
}

/// Classify one pre-hyphen key as middle or left and insert it.
fn insert_left_mid(orig: &str, ch: char, keys: &mut StrokeKeys) -> Result<(), StrokeError> {
    if is_mid(ch) {
        keys.mid.insert(ch);
    } else if is_left(ch) {
        keys.left.insert(ch);
    } else {
        return Err(StrokeError::new(format!(
            "bad left/mid key \"{ch}\" in \"{orig}\""
        )));
    }
    Ok(())
}

/// Which bank the no-hyphen scanner is currently filling.
#[derive(Clone, Copy)]
enum Phase {
    /// Before the first middle key: only left-bank keys are legal.
    Left,
    /// Inside the run of middle keys.
    Mid,
    /// After the middles: only right-bank keys are legal.
    Right,
}

/// Fill `keys` from a hyphen-free sub-stroke: left keys, then the run of
/// middle keys, then right keys. Without any middle, the whole body must be
/// left-bank.
fn parse_unhyphenated(orig: &str, body: &str, keys: &mut StrokeKeys) -> Result<(), StrokeError> {
    let mut phase = Phase::Left;
    for ch in body.chars() {
        phase = step_phase(phase, ch, orig, keys)?;
    }
    Ok(())
}

/// Advance the no-hyphen scanner by one key, inserting it into its bank.
fn step_phase(
    phase: Phase,
    ch: char,
    orig: &str,
    keys: &mut StrokeKeys,
) -> Result<Phase, StrokeError> {
    match phase {
        Phase::Left | Phase::Mid if is_mid(ch) => {
            keys.mid.insert(ch);
            Ok(Phase::Mid)
        },
        Phase::Left if is_left(ch) => {
            keys.left.insert(ch);
            Ok(Phase::Left)
        },
        Phase::Left => Err(StrokeError::new(format!(
            "bad left key \"{ch}\" in \"{orig}\""
        ))),
        Phase::Mid | Phase::Right if is_right(ch) => {
            keys.right.insert(ch);
            Ok(Phase::Right)
        },
        Phase::Mid | Phase::Right => Err(StrokeError::new(format!(
            "bad right key \"{ch}\" in \"{orig}\" (mid key after right keys?)"
        ))),
    }
}

/// Render canonical key sets back to a sub-stroke string.
#[must_use]
pub fn render_stroke(k: &StrokeKeys) -> String {
    let pick = |order: &[char], set: &KeySet| -> String {
        order.iter().copied().filter(|c| set.contains(*c)).collect()
    };
    let left = pick(&LEFT_ORDER, &k.left);
    let mid = pick(&MID_ORDER, &k.mid);
    let right = pick(&RIGHT_ORDER, &k.right);
    let hash = if k.num { "#" } else { "" };
    if !right.is_empty() && mid.is_empty() {
        format!("{hash}{left}-{right}")
    } else {
        format!("{hash}{left}{mid}{right}")
    }
}
