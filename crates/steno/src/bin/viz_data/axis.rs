//! Buckets node leaves that share a tree position and a source line into one
//! choice ("axis") entry, so `@count`/`@fuse` alternatives of the same
//! authored line read as one path node with N labeled options instead of N
//! flat, near-identical siblings. See the caller (`tree.rs`) for how
//! positions are computed (real stroke prefixes for descendants, a synthetic
//! per-category position for roots).

use std::collections::HashMap;

use steno::TypedEntry;

/// Which kind of choice an axis represents.
pub enum Kind {
    /// `@count`-fanned digit choice.
    Count,
    /// `@type`/`@fuse` type-name choice.
    Type,
}

impl Kind {
    /// The JSON `"kind"` string.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Count => "count",
            Self::Type => "type",
        }
    }
}

/// One value of a choice axis: a human label plus the real leaf key it
/// resolves to.
pub struct Choice {
    /// What to show for this alternative (a count digit or a type name).
    pub label: String,
    /// The real node key this choice resolves to.
    pub key: String,
}

/// A choice point: pick one of several labeled alternatives.
pub struct Axis {
    /// Which kind of choice this is.
    pub kind: Kind,
    /// The alternatives, in source order.
    pub choices: Vec<Choice>,
}

/// Bucketing output: every position's ordered list of next-step keys (either
/// a plain leaf key, when only one entry sits there, or a synthetic axis key
/// otherwise), plus the axis metadata for each synthetic key (in first-seen
/// order, for deterministic output).
pub struct Grouped {
    /// Tree position -> its ordered next-step keys.
    pub children_of: HashMap<String, Vec<String>>,
    /// Synthetic axis key -> its axis metadata, in first-seen order.
    pub axes: Vec<(String, Axis)>,
}

/// One leaf awaiting bucketing: its key, its tree position, and a
/// representative entry (any member sharing that key after the lineflag
/// merge — they all share `source`).
pub struct Leaf<'a> {
    /// The real node key.
    pub key: String,
    /// This leaf's parent tree position (see `tree::position`).
    pub position: String,
    /// A representative entry for this key (`source`/`count`/`type_label`
    /// are identical across a key's multi-line/one-liner variants).
    pub entry: &'a TypedEntry,
}

/// Which axis kind an entry's choice belongs to. Checked via `type_label`
/// rather than `source.count.is_some()`: a `@count` entry can still have
/// `%t` slots (e.g. "typed param"), so the bucket's own differentiator —
/// whether these particular siblings vary by appended type — is what
/// decides the kind, not whether the source entry has a count directive at
/// all.
const fn kind(entry: &TypedEntry) -> Kind {
    if entry.type_label.is_some() {
        Kind::Type
    } else {
        Kind::Count
    }
}

/// This entry's label for its axis: the resolved count digit, or its type
/// name (falling back to `"default"` for a free type).
fn label(entry: &TypedEntry, kind: &Kind) -> String {
    match kind {
        Kind::Count => entry.count.map_or_else(String::new, |c| c.to_string()),
        Kind::Type => entry
            .type_label
            .clone()
            .unwrap_or_else(|| "default".to_owned()),
    }
}

/// Bucket leaves by `(position, source.stroke_raw)`, in first-seen order.
#[must_use]
pub fn group(leaves: &[Leaf<'_>]) -> Grouped {
    let mut bucket_index: HashMap<(String, String), usize> = HashMap::new();
    let mut buckets: Vec<Vec<&Leaf<'_>>> = Vec::new();
    for leaf in leaves {
        let bucket_key = (leaf.position.clone(), leaf.entry.source.stroke_raw.clone());
        let idx = *bucket_index.entry(bucket_key).or_insert_with(|| {
            buckets.push(Vec::new());
            buckets.len() - 1
        });
        if let Some(b) = buckets.get_mut(idx) {
            b.push(leaf);
        }
    }
    emit(&buckets)
}

/// Turn each bucket into either a plain leaf reference (size 1) or a
/// synthetic axis node (size >1), recording both the per-position children
/// list and the axis metadata. Buckets are numbered by position in the input
/// (not by source line): one authored line can produce several buckets at
/// different tree depths (e.g. a fused shape/type choice and, nested below
/// one of its options, a generic arg's own type choice) — the source line
/// alone isn't a unique axis identity.
fn emit(buckets: &[Vec<&Leaf<'_>>]) -> Grouped {
    let mut children_of: HashMap<String, Vec<String>> = HashMap::new();
    let mut axes = Vec::new();
    for (idx, bucket) in buckets.iter().enumerate() {
        let Some(first) = bucket.first() else {
            continue;
        };
        let key = if bucket.len() == 1 {
            first.key.clone()
        } else {
            let axis_key = format!("#{}.{idx}", first.entry.source.line);
            let k = kind(first.entry);
            let choices = bucket
                .iter()
                .map(|l| Choice {
                    label: label(l.entry, &k),
                    key: l.key.clone(),
                })
                .collect();
            axes.push((axis_key.clone(), Axis { kind: k, choices }));
            axis_key
        };
        children_of
            .entry(first.position.clone())
            .or_default()
            .push(key);
    }
    Grouped { children_of, axes }
}
