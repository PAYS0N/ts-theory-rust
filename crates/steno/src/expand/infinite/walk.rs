//! The reference walker: replay the type-append that Pass B would have
//! enumerated, driven by an **obligation stack** rather than a table of rows.
//! Appending an arity-N type pushes N argument obligations; each later type
//! stroke discharges the innermost one. A sequence is terminal iff every
//! construct slot is filled and no obligation remains. It enumerates nothing —
//! cost is proportional to the strokes pressed, and nesting is unbounded.

use super::{Construct, InfType, is_slot};
use crate::parse::Chunk;
use crate::stroke::subtract_strokes;

/// The walker's verdict for one stroke sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkResult {
    /// Rendered text: the construct template with accumulated types
    /// substituted (D12 fill order).
    pub text: String,
    /// True iff the sequence completes the construct (no obligation remains).
    pub terminal: bool,
}

/// One consumed top-level type: its rendered text and whether it completed.
struct Consumed {
    /// Rendered type text (complete or bracketless partial).
    text: String,
    /// True when the type's obligations were fully discharged.
    complete: bool,
}

/// Find a type-table record by stroke.
fn lookup<'a>(stroke: &str, types: &'a [InfType]) -> Option<&'a InfType> {
    types.iter().find(|t| t.stroke == stroke)
}

/// Substitute `args` into the `%t` markers of a type's text.
fn render_type(text: &str, args: &[String]) -> String {
    let mut out = String::new();
    for (i, part) in text.split("%t").enumerate() {
        if i > 0 {
            out.push_str(args.get(i - 1).map_or("", String::as_str));
        }
        out.push_str(part);
    }
    out
}

/// The bracketless partial form: `Array`, or `Map number`.
fn partial(t: &InfType, args: &[String]) -> String {
    let name = t.text.split('<').next().unwrap_or(&t.text);
    if args.is_empty() {
        name.to_string()
    } else {
        format!("{name} {}", args.join(" "))
    }
}

/// Consume one complete (possibly nested) type from `ts` starting at `*pos`,
/// advancing `*pos`. `None` iff a stroke is not a valid type. An incomplete
/// type (strokes ran out mid-obligation) returns its partial text with
/// `complete = false`.
fn consume_type(ts: &[String], pos: &mut usize, types: &[InfType]) -> Option<Consumed> {
    let t = lookup(ts.get(*pos)?, types)?;
    *pos += 1;
    let mut args: Vec<String> = Vec::new();
    for _ in 0..t.arity {
        if *pos >= ts.len() {
            return Some(Consumed {
                text: partial(t, &args),
                complete: false,
            });
        }
        let arg = consume_type(ts, pos, types)?;
        let complete = arg.complete;
        args.push(arg.text);
        if !complete {
            return Some(Consumed {
                text: partial(t, &args),
                complete: false,
            });
        }
    }
    Some(Consumed {
        text: render_type(&t.text, &args),
        complete: true,
    })
}

/// Surface text of a non-slot chunk (landings/body-breaks carry no text in
/// V1 — there is no board cursor to place).
fn chunk_surface(c: &Chunk) -> &str {
    match c {
        Chunk::Lit(s) => s,
        Chunk::Brace { open: true } => "{",
        Chunk::Brace { open: false } => "}",
        Chunk::Newline => "\n",
        Chunk::Tab => "\t",
        _ => "",
    }
}

/// Split a template's surface text at its type slots (template order).
///
/// `n` slots yield `n + 1` fragments interleaved with the slot texts. This is
/// the exact representation `build-javelin` emits, so the C++ replay and this
/// walker render identically by construction.
#[must_use]
pub fn template_fragments(template: &[Chunk]) -> Vec<String> {
    let mut frags = vec![String::new()];
    for c in template {
        if is_slot(c) {
            frags.push(String::new());
        } else if let Some(last) = frags.last_mut() {
            last.push_str(chunk_surface(c));
        }
    }
    frags
}

/// Render a template with its type slots filled (`slot_texts` in template
/// order; unfilled slots are empty).
#[must_use]
pub fn render_filled(template: &[Chunk], slot_texts: &[String]) -> String {
    let frags = template_fragments(template);
    let mut out = frags.first().cloned().unwrap_or_default();
    for (i, text) in slot_texts.iter().enumerate() {
        out.push_str(text);
        if let Some(frag) = frags.get(i + 1) {
            out.push_str(frag);
        }
    }
    out
}

/// Walk the residual type strokes against one construct, filling its slots in
/// D12 order. `None` iff a stroke is invalid or extra strokes remain.
fn walk_construct(c: &Construct, ts: &[String], types: &[InfType]) -> Option<WalkResult> {
    let mut pos = 0;
    let mut slot_texts = vec![String::new(); c.slots.len()];
    let mut terminal = true;
    for &slot_idx in &c.fill_order {
        if pos >= ts.len() {
            terminal = false;
            break;
        }
        let consumed = consume_type(ts, &mut pos, types)?;
        if !consumed.complete {
            terminal = false;
        }
        if let Some(slot) = slot_texts.get_mut(slot_idx) {
            *slot = consumed.text;
        }
    }
    if pos < ts.len() {
        return None; // extra strokes matched no slot
    }
    Some(WalkResult {
        text: render_filled(&c.template, &slot_texts),
        terminal,
    })
}

/// Match a construct's base against the head of `strokes`, returning a
/// specificity score (longer/fused = higher) and the residual type strokes.
fn match_base(
    c: &Construct,
    strokes: &[String],
    types: &[InfType],
) -> Option<(usize, Vec<String>)> {
    let base_len = c.base.len();
    if strokes.len() < base_len {
        return None;
    }
    let (head, rest) = strokes.split_at(base_len);
    if head != c.base.as_slice() {
        return None;
    }
    match &c.shape {
        None => Some((base_len * 2, rest.to_vec())),
        Some(shape) => {
            let (m, tail) = rest.split_first()?;
            let residual = subtract_strokes(m, shape).ok().flatten()?;
            lookup(&residual, types)?;
            let mut ts = vec![residual];
            ts.extend_from_slice(tail);
            Some((base_len * 2 + 1, ts))
        },
    }
}

/// The reference walker (criterion 3): given a stroke sequence and the two
/// tables, return the rendered text and terminal flag, or `None` when no
/// construct/type rule matches. Enumerates nothing.
#[must_use]
pub fn walk(strokes: &[String], types: &[InfType], constructs: &[Construct]) -> Option<WalkResult> {
    let mut best: Option<(usize, Vec<String>, &Construct)> = None;
    for c in constructs {
        if let Some((score, ts)) = match_base(c, strokes, types)
            && best.as_ref().is_none_or(|b| score > b.0)
        {
            best = Some((score, ts, c));
        }
    }
    let (_, ts, c) = best?;
    walk_construct(c, &ts, types)
}
