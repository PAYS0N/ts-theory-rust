//! Source-file parser: fenced blocks and `@`-directives -> `Vec<Entry>`.

use super::template::parse_template;
use super::{Entry, EntryFlags};
use crate::error::StenoError;

/// If `line` is a fence (4+ backticks), return the trimmed text after the
/// backticks: the stroke for an opener, empty for a closer.
fn fence_stroke(line: &str) -> Option<&str> {
    let rest = line.trim_start_matches('`');
    let ticks = line.len() - rest.len();
    (ticks >= 4).then(|| rest.trim())
}

/// Parse a whole `.steno` source into entries.
///
/// # Errors
/// Returns [`StenoError`] (with its 1-based line) on unterminated blocks,
/// stray fences or text, malformed directives, or any template parse error.
pub fn parse_source(src: &str) -> Result<Vec<Entry>, StenoError> {
    let lines: Vec<&str> = src.lines().collect();
    let mut entries: Vec<Entry> = Vec::new();
    let mut i = 0_usize;
    while let Some(&raw) = lines.get(i) {
        let line_no = i.saturating_add(1);
        if let Some(stroke_raw) = fence_stroke(raw) {
            if stroke_raw.is_empty() {
                return Err(StenoError::new(
                    "closing fence without an open block",
                    line_no,
                ));
            }
            let (entry, next) = parse_block(&lines, i, stroke_raw)?;
            entries.push(entry);
            i = next;
            continue;
        }
        i = parse_plain_line(raw, line_no, i, &mut entries)?;
    }
    Ok(entries)
}

/// Handle one non-fence line: skip blanks/comments, apply a directive to the
/// entry above, or reject stray text. Returns the next line index.
fn parse_plain_line(
    raw: &str,
    line_no: usize,
    i: usize,
    entries: &mut [Entry],
) -> Result<usize, StenoError> {
    let t = raw.trim();
    if !(t.is_empty() || t.starts_with("//") || t.starts_with('#')) {
        if !t.starts_with('@') {
            return Err(StenoError::new(format!("unexpected text: {t}"), line_no));
        }
        let Some(last) = entries.last_mut() else {
            return Err(StenoError::new("directive before any entry", line_no));
        };
        apply_directive(last, t, line_no)?;
    }
    Ok(i.saturating_add(1))
}

/// Collect a block's content lines up to its closing fence. Returns the
/// joined text and the index after the closer, or None when unterminated.
fn block_text(lines: &[&str], open_idx: usize) -> Option<(String, usize)> {
    let mut content: Vec<&str> = Vec::new();
    let mut i = open_idx.saturating_add(1);
    while let Some(&cl) = lines.get(i) {
        i = i.saturating_add(1);
        if fence_stroke(cl).is_some_and(str::is_empty) {
            return Some((content.join("\n"), i));
        }
        content.push(cl);
    }
    None
}

/// Parse one fenced block starting at `open_idx`. Returns the entry and the
/// index of the first line after the closing fence.
fn parse_block(
    lines: &[&str],
    open_idx: usize,
    stroke_raw: &str,
) -> Result<(Entry, usize), StenoError> {
    let line_no = open_idx.saturating_add(1);
    let Some((text, next)) = block_text(lines, open_idx) else {
        return Err(StenoError::new(
            format!("unterminated block for \"{stroke_raw}\""),
            line_no,
        ));
    };
    let entry = Entry {
        stroke: stroke_raw
            .split('/')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect(),
        stroke_raw: stroke_raw.to_string(),
        template: parse_template(&text, line_no.saturating_add(1))?,
        raw: text,
        count: None,
        arity: None,
        flags: EntryFlags::default(),
        line: line_no,
    };
    Ok((entry, next))
}

/// Split a directive line into its name and trimmed argument.
fn directive_parts(t: &str) -> Option<(&str, &str)> {
    let rest = t.strip_prefix('@')?;
    let name_end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    let (name, tail) = rest.split_at_checked(name_end)?;
    (!name.is_empty()).then(|| (name, tail.trim()))
}

/// Parse the `@arity` argument as a non-negative integer.
fn parse_arity(arg: &str, line_no: usize) -> Result<u32, StenoError> {
    arg.parse().map_err(|_| {
        StenoError::new(
            format!("@arity needs a non-negative integer, got \"{arg}\""),
            line_no,
        )
    })
}

/// Parse and apply a single `@`-directive to an existing entry.
fn apply_directive(e: &mut Entry, t: &str, line_no: usize) -> Result<(), StenoError> {
    let Some((name, arg)) = directive_parts(t) else {
        return Err(StenoError::new(
            format!("malformed directive: {t}"),
            line_no,
        ));
    };
    match name {
        "count" if arg.is_empty() => Err(StenoError::new("@count needs a key list", line_no)),
        "count" => {
            e.count = Some(arg.to_string());
            Ok(())
        },
        "arity" => {
            e.arity = Some(parse_arity(arg, line_no)?);
            Ok(())
        },
        flag if e.flags.set_named(flag) => Ok(()),
        _ => Err(StenoError::new(
            format!("unknown directive @{name}"),
            line_no,
        )),
    }
}
