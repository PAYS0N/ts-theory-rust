//! Hand-rolled JSON output: an insertion-ordered string map (matching TS object
//! key semantics) and a `JSON.stringify`-compatible one-entry-per-line writer.
//!
//! Zero runtime dependencies — the dictionaries are flat `string -> string`
//! maps, so a small escaper and a fixed layout reproduce the TS output exactly.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

/// A string→string map that preserves first-insertion order, like a JS object.
#[derive(Debug, Clone, Default)]
pub struct OrderedMap {
    /// Keys in first-insertion order.
    order: Vec<String>,
    /// Key→value lookup.
    map: HashMap<String, String>,
}

impl OrderedMap {
    /// An empty map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or update `key`. Returns the previous value if the key existed
    /// (its position in iteration order is retained).
    pub fn insert(&mut self, key: String, value: String) -> Option<String> {
        match self.map.entry(key) {
            Entry::Occupied(mut slot) => Some(slot.insert(value)),
            Entry::Vacant(slot) => {
                self.order.push(slot.key().clone());
                slot.insert(value);
                None
            },
        }
    }

    /// The current value for `key`, if present.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.map.get(key).map(String::as_str)
    }

    /// Number of distinct keys.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.order.len()
    }

    /// True when the map holds no entries.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// Key→value pairs in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.order
            .iter()
            .filter_map(move |k| self.map.get(k).map(|v| (k.as_str(), v.as_str())))
    }
}

/// The lowercase hex digit for a nibble (0–15).
const fn hex_digit(nibble: u32) -> char {
    match nibble {
        0 => '0',
        1 => '1',
        2 => '2',
        3 => '3',
        4 => '4',
        5 => '5',
        6 => '6',
        7 => '7',
        8 => '8',
        9 => '9',
        10 => 'a',
        11 => 'b',
        12 => 'c',
        13 => 'd',
        14 => 'e',
        _ => 'f',
    }
}

/// Append the `\u00XX` escape for a control character below `0x20`.
fn push_control_escape(c: char, out: &mut String) {
    let v = u32::from(c);
    out.push_str("\\u00");
    out.push(hex_digit((v >> 4) & 0xf));
    out.push(hex_digit(v & 0xf));
}

/// Append the `JSON.stringify` escaping of `s` (double-quoted) to `out`.
fn push_json_string(s: &str, out: &mut String) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => push_control_escape(c, out),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Serialize an ordered map as `{\n"k": "v",\n...\n}\n` — one entry per line,
/// `JSON.stringify`-compatible string escaping, insertion order.
#[must_use]
pub fn to_json(map: &OrderedMap) -> String {
    if map.is_empty() {
        return "{}\n".to_owned();
    }
    let mut out = String::from("{\n");
    for (i, (key, value)) in map.iter().enumerate() {
        push_json_string(key, &mut out);
        out.push_str(": ");
        push_json_string(value, &mut out);
        if i + 1 < map.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("}\n");
    out
}
