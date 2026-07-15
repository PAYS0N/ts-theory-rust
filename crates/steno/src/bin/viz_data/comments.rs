//! Scans `dict.steno`'s raw text for explicit `## label` category markers,
//! `##> label` section markers, and `### text` description markers. Ordinary
//! `#`/`//` comments stay inert prose. A line matching `^##\s+(.+)$` (but not
//! `##>` or `###`) starts (or re-starts) a category, applying to every entry
//! until the next `##` line; a `^##>\s+(.+)$` line starts a section that
//! sub-divides the current category's shared-stroke children until the next
//! `##>` or `##` line; a `^###\s+(.+)$` line describes only the single fenced
//! entry immediately following it (blank lines in between are allowed, any
//! other content clears it).

use std::collections::HashMap;

/// One `## label` marker and the 1-based line it appears on.
pub struct Marker {
    /// 1-based source line the marker appears on.
    pub line: usize,
    /// The category label, trimmed.
    pub label: String,
}

/// Find every `## label` line in `src`, in file order.
pub fn scan(src: &str) -> Vec<Marker> {
    src.lines()
        .enumerate()
        .filter_map(|(i, line)| marker_label(line).map(|label| Marker { line: i + 1, label }))
        .collect()
}

/// `Some(label)` when `line` is a `## label` marker, trimmed. Both the
/// `###` description marker and the `##>` section marker are excluded, since
/// each would otherwise also match the bare `##` prefix.
fn marker_label(line: &str) -> Option<String> {
    let rest = line.trim_start().strip_prefix("##")?;
    if rest.starts_with('#') || rest.starts_with('>') {
        return None;
    }
    let label = rest.trim();
    if label.is_empty() {
        None
    } else {
        Some(label.to_owned())
    }
}

/// Find every `##> label` section line in `src`, in file order. A section
/// sub-divides the children of a shared stroke *within* the current `##`
/// category (e.g. array vs. string methods hanging off one `.` accessor).
pub fn scan_sections(src: &str) -> Vec<Marker> {
    src.lines()
        .enumerate()
        .filter_map(|(i, line)| section_label(line).map(|label| Marker { line: i + 1, label }))
        .collect()
}

/// `Some(label)` when `line` is a `##> label` section marker, trimmed.
fn section_label(line: &str) -> Option<String> {
    let label = line.trim_start().strip_prefix("##>")?.trim();
    if label.is_empty() {
        None
    } else {
        Some(label.to_owned())
    }
}

/// The label whose marker most closely precedes `entry_line`, or
/// `"Uncategorized"` if none does.
pub fn label_for(markers: &[Marker], entry_line: usize) -> &str {
    markers
        .iter()
        .rev()
        .find(|m| m.line < entry_line)
        .map_or("Uncategorized", |m| m.label.as_str())
}

/// The active section label for `entry_line`, or `None`. A `##>` section
/// only applies until the next `##>` or the next plain `##` category — so a
/// section is active only when its marker is more recent than any category
/// marker preceding the entry.
pub fn section_for<'a>(
    sections: &'a [Marker],
    categories: &[Marker],
    entry_line: usize,
) -> Option<&'a str> {
    let sec = sections.iter().rev().find(|m| m.line < entry_line)?;
    let cat_line = categories
        .iter()
        .rev()
        .find(|m| m.line < entry_line)
        .map_or(0, |m| m.line);
    (sec.line > cat_line).then_some(sec.label.as_str())
}

/// Find every `### text` line in `src` that (modulo blank lines) directly
/// precedes a fence-open line, mapping that fence's 1-based opening line to
/// the description text.
#[must_use]
pub fn scan_desc(src: &str) -> HashMap<usize, String> {
    let mut out = HashMap::new();
    let mut pending: Option<String> = None;
    for (i, line) in src.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(text) = desc_text(trimmed) {
            pending = Some(text);
        } else if trimmed.is_empty() {
            // Blank lines don't disturb a pending description.
        } else if is_fence_open(trimmed) {
            if let Some(desc) = pending.take() {
                out.insert(i + 1, desc);
            }
        } else {
            pending = None;
        }
    }
    out
}

/// `Some(text)` when `line` is a `### text` description marker, trimmed.
fn desc_text(line: &str) -> Option<String> {
    let rest = line.strip_prefix("###")?.trim();
    if rest.is_empty() {
        None
    } else {
        Some(rest.to_owned())
    }
}

/// True for a fence-open line (four backticks immediately followed by the
/// stroke text, as opposed to a bare closing fence).
fn is_fence_open(line: &str) -> bool {
    line.strip_prefix("````")
        .is_some_and(|rest| !rest.is_empty())
}
