//! Emit the C++ data header: the type table and the construct table as `const`
//! arrays. No entry rows — the `javelin-ext` walk reconstructs them (criterion
//! 4). Strokes are pre-encoded to javelin key masks so the header parses
//! nothing at load.

use std::fmt::Write as _;

use super::super::{Construct, InfType, TemplateOp, template_ops};
use super::{c_escape, stroke_mask};
use crate::error::ExpandError;
use crate::parse::Chunk;

/// A generous cap on outline length reported to the engine. The walk itself is
/// unbounded (D8); this only bounds how many strokes the engine offers.
const MAX_OUTLINE_LENGTH: u32 = 24;

/// Render the self-contained C++ data header for `types` and `constructs`.
///
/// # Errors
/// Returns [`ExpandError`] when any stroke fails to encode to a key mask.
pub fn emit_data_header(
    types: &[InfType],
    constructs: &[Construct],
) -> Result<String, ExpandError> {
    let mut out = String::from(PREAMBLE);
    out.push_str(&type_rows(types)?);
    let (arrays, rows) = construct_blocks(constructs)?;
    out.push_str(&arrays);
    out.push_str(&rows);
    let _ = write!(
        out,
        "static const uint32_t MAX_OUTLINE_LENGTH = {MAX_OUTLINE_LENGTH}u;\n\n"
    );
    out.push_str("} // namespace steno_generated\n");
    Ok(out)
}

/// The type table array.
fn type_rows(types: &[InfType]) -> Result<String, ExpandError> {
    let mut out = String::from("static const GenType TYPES[] = {\n");
    for t in types {
        let _ = writeln!(
            out,
            "  {{0x{:06X}u, {}u, \"{}\"}},",
            stroke_mask(&t.stroke)?,
            t.arity,
            c_escape(&t.text),
        );
    }
    let _ = write!(
        out,
        "}};\nstatic const uint32_t TYPE_COUNT = {}u;\n\n",
        types.len()
    );
    Ok(out)
}

/// Build the per-construct backing arrays and the `CONSTRUCTS` table together.
fn construct_blocks(constructs: &[Construct]) -> Result<(String, String), ExpandError> {
    let mut arrays = String::new();
    let mut rows = String::from("static const GenConstruct CONSTRUCTS[] = {\n");
    for (i, c) in constructs.iter().enumerate() {
        arrays.push_str(&construct_arrays(i, c)?);
        rows.push_str(&construct_row(i, c)?);
    }
    let _ = write!(
        rows,
        "}};\nstatic const uint32_t CONSTRUCT_COUNT = {}u;\n\n",
        constructs.len(),
    );
    Ok((arrays, rows))
}

/// The three backing arrays (base masks, template ops, fill order) for one
/// construct. An empty base or fill order is elided in favour of `nullptr`.
fn construct_arrays(i: usize, c: &Construct) -> Result<String, ExpandError> {
    let mut out = String::new();
    if !c.base.is_empty() {
        let _ = writeln!(
            out,
            "static const uint32_t c{i}Base[] = {{{}}};",
            base_masks(c)?
        );
    }
    out.push_str(&ops_array(i, &c.template));
    if !c.fill_order.is_empty() {
        let order: Vec<String> = c.fill_order.iter().map(|k| format!("{k}u")).collect();
        let _ = writeln!(
            out,
            "static const uint8_t c{i}Fill[] = {{{}}};",
            order.join(", ")
        );
    }
    Ok(out)
}

/// One `GenOp` initializer for a template op. The second element initializes
/// `GenOp`'s anonymous union: designated for the ops that carry a payload,
/// empty (`text = nullptr`) for the ops that carry none.
fn op_initializer(op: &TemplateOp) -> String {
    match op {
        TemplateOp::Text(s) => format!("{{OP_TEXT, {{.text = \"{}\"}}}}", c_escape(s)),
        TemplateOp::Slot => "{OP_SLOT, {}}".to_owned(),
        TemplateOp::Landing(n) => format!("{{OP_LANDING, {{.landing = {n}u}}}}"),
        TemplateOp::Newline => "{OP_NEWLINE, {}}".to_owned(),
    }
}

/// The `c{i}Ops[]` backing array line for one construct's template.
fn ops_array(i: usize, template: &[Chunk]) -> String {
    let rendered: Vec<String> = template_ops(template).iter().map(op_initializer).collect();
    let mut out = String::new();
    let _ = writeln!(
        out,
        "static const GenOp c{i}Ops[] = {{{}}};",
        rendered.join(", ")
    );
    out
}

/// The comma-joined base-segment key masks of a construct.
fn base_masks(c: &Construct) -> Result<String, ExpandError> {
    let masks: Result<Vec<String>, _> = c
        .base
        .iter()
        .map(|s| stroke_mask(s).map(|m| format!("0x{m:06X}u")))
        .collect();
    Ok(masks?.join(", "))
}

/// One `CONSTRUCTS[]` row referencing the construct's backing arrays.
fn construct_row(i: usize, c: &Construct) -> Result<String, ExpandError> {
    let base = if c.base.is_empty() {
        "nullptr".to_owned()
    } else {
        format!("c{i}Base")
    };
    let fill = if c.fill_order.is_empty() {
        "nullptr".to_owned()
    } else {
        format!("c{i}Fill")
    };
    let (has_shape, shape_mask) = match &c.shape {
        Some(s) => ("true", stroke_mask(s)?),
        None => ("false", 0),
    };
    let op_count = template_ops(&c.template).len();
    Ok(format!(
        "  {{{base}, {}u, {has_shape}, 0x{shape_mask:06X}u, c{i}Ops, {op_count}u, {fill}, {}u}},\n",
        c.base.len(),
        c.slots.len(),
    ))
}

/// The fixed head of the data header: guard, include, struct definitions.
const PREAMBLE: &str = "\
//---------------------------------------------------------------------------
// Generated by build-javelin from dict.infinite.steno.
// Do not edit and do not commit: out/ is a build product (see the of-javelin
// brief, D4). One record per @type and one per Pass-A construct; zero rows.
//---------------------------------------------------------------------------
#pragma once
#include <stdint.h>

namespace steno_generated {

struct GenType {
  uint32_t stroke;
  uint32_t arity;
  const char *text; // literal with %t argument markers
};

static const uint8_t OP_TEXT = 0u;
static const uint8_t OP_SLOT = 1u;
static const uint8_t OP_LANDING = 2u;
static const uint8_t OP_NEWLINE = 3u;

// `text` and `landing` are mutually exclusive — a TEXT op has no landing index
// and a LANDING op no surface bytes — so they share storage: 8 bytes per op on
// the device rather than 12. Read only the member `kind` selects. The saving is
// load-bearing, not cosmetic: the op arrays are the single largest thing this
// header contributes to firmware flash, and the image has to fit under
// STENO_CONFIG_BLOCK_ADDRESS (see scripts/flash_extent_check.sh).
struct GenOp {
  uint8_t kind;        // 0=TEXT 1=SLOT 2=LANDING 3=NEWLINE
  union {
    const char *text;  // TEXT only
    uint32_t landing;  // LANDING only
  };
};

struct GenConstruct {
  const uint32_t *base;      // baseLength segment masks, or nullptr
  uint32_t baseLength;
  bool hasShape;             // true for a @fuse construct
  uint32_t shape;            // fuse shape mask (valid iff hasShape)
  const GenOp *ops;          // opCount template ops, in template order
  uint32_t opCount;
  const uint8_t *fillOrder;  // slotCount slot indices, or nullptr
  uint32_t slotCount;
};

";
