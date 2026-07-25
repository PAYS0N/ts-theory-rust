//! The reference walker: replay the type-append that Pass B would have
//! enumerated, driven by an **obligation stack** rather than a table of rows.
//! Appending an arity-N type pushes N argument obligations; each later type
//! stroke discharges the innermost one. A sequence is terminal iff every
//! construct slot is filled and no obligation remains. It enumerates nothing —
//! cost is proportional to the strokes pressed, and nesting is unbounded.

mod consume;
mod ops;

pub use ops::{TemplateOp, template_ops};

use consume::{consume_type, lookup};

use super::{Construct, InfType, is_slot};
use crate::error::ExpandError;
use crate::expand::TypedEntry;
use crate::parse::{Chunk, Entry, EntryFlags};
use crate::render::{render_plain, render_smart};
use crate::snippet::{render_snippet, wrap_inline_body};
use crate::stroke::subtract_strokes;

/// The walker's verdict for one stroke sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkResult {
    /// The construct template with every type slot resolved into a literal
    /// chunk (D12 fill order); every other chunk (`Landing`/`BodyBreak`/
    /// `Brace`/`Newline`/`Tab`/`Lit`) is carried through intact, so it can be
    /// replayed through the exact render/snippet code the enumerated
    /// `dict.steno` path uses.
    pub template: Vec<Chunk>,
    /// True iff the sequence completes the construct (no obligation remains).
    pub terminal: bool,
}

/// Which rendering pipeline [`render_walk`] should replay a resolved template through.
///
/// The same three profiles the enumerated `dict.steno` path renders: the two
/// Plover dictionaries and the nvim inline snippet body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// The dumb-editor Plover dictionary value.
    Plain,
    /// The auto-close/type-over Plover dictionary value.
    Smart,
    /// The unified inline nvim snippet value (sentinel-wrapped body, or a
    /// plain glued partial when non-terminal).
    Nvim,
}

/// Wrap a resolved walk template in a synthetic [`TypedEntry`] so it can be
/// handed to the exact render/snippet code the enumerated path uses. The
/// infinite path runs no line-flag pass (`one_liner = false`, so `%b` always
/// breaks), and carries no source directives (never `@literal`).
fn synth_entry(r: &WalkResult) -> TypedEntry {
    TypedEntry {
        stroke: String::new(),
        template: r.template.clone(),
        terminal: r.terminal,
        one_liner: false,
        count: None,
        type_label: None,
        source: Entry {
            stroke: Vec::new(),
            stroke_raw: String::new(),
            template: Vec::new(),
            raw: String::new(),
            count: None,
            arity: None,
            flags: EntryFlags::default(),
            line: 0,
        },
    }
}

/// Render a walk result through one of the three static-path renderers.
///
/// Wraps its resolved template in a synthetic [`TypedEntry`] and delegates to
/// the exact code `dict.steno` renders through — this is what makes the
/// programmatic path's golden vectors trustworthy by construction.
///
/// # Errors
/// Propagates any `RenderError`/`SnippetError` (as an [`ExpandError`]) from
/// the underlying renderer.
pub fn render_walk(r: &WalkResult, profile: Profile) -> Result<String, ExpandError> {
    let entry = synth_entry(r);
    Ok(match profile {
        Profile::Plain => render_plain(&entry)?.1,
        Profile::Smart => render_smart(&entry)?.1,
        Profile::Nvim => {
            let s = render_snippet(&entry)?;
            if s.terminal {
                wrap_inline_body(&s.body)
            } else {
                format!("{{^}}{}{{^}}", s.body)
            }
        },
    })
}

/// Resolve a construct template's type slots into literal chunks (template
/// order), keeping every other chunk — `Landing`/`BodyBreak`/`Brace`/
/// `Newline`/`Tab`/`Lit` — intact, the shape `render`/`snippet` already know
/// how to walk.
fn resolve_template(template: &[Chunk], slot_texts: &[String]) -> Vec<Chunk> {
    let mut slots = slot_texts.iter();
    template
        .iter()
        .map(|c| {
            if is_slot(c) {
                Chunk::Lit(slots.next().cloned().unwrap_or_default())
            } else {
                c.clone()
            }
        })
        .collect()
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
        template: resolve_template(&c.template, &slot_texts),
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
