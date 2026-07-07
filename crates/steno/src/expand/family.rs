//! Family-root fixup: a stroke that's a strict `/`-prefix of another
//! dictionary entry can never end an outline, no matter which pass marked it
//! terminal. `expand_types_one` only sees one entry at a time, so a "family
//! root" placeholder with zero `%t` slots (e.g. `STKWR-PBGS`'s "pre-func",
//! always superseded by `STKWR-PBGS/-FLT`, `STKWR-PBGS/-R`, ...) falls
//! through its terminal-by-default branch. This pass runs once over the
//! whole typed batch, after every stroke is known, to correct exactly that.

use super::TypedEntry;

/// Every proper leading `/`-join of `stroke`, e.g. `"a/b/c"` yields `"a"` and
/// `"a/b"`.
fn proper_prefixes(stroke: &str) -> Vec<String> {
    let mut prefixes = Vec::new();
    let mut acc = String::new();
    let mut segs = stroke.split('/').peekable();
    while let Some(seg) = segs.next() {
        if !acc.is_empty() {
            acc.push('/');
        }
        acc.push_str(seg);
        if segs.peek().is_some() {
            prefixes.push(acc.clone());
        }
    }
    prefixes
}

/// Force `terminal = false` on any entry whose stroke is a strict prefix of
/// some other entry's stroke in the same batch — it will always be
/// superseded by a longer chord, regardless of how it was classified.
pub fn fix_family_terminals(entries: &mut [TypedEntry]) {
    let mut roots = std::collections::HashSet::new();
    for e in entries.iter() {
        roots.extend(proper_prefixes(&e.stroke));
    }
    for e in entries.iter_mut() {
        if e.terminal && roots.contains(&e.stroke) {
            e.terminal = false;
        }
    }
}
