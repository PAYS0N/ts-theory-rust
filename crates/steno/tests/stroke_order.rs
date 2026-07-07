//! Every stroke written in `dict.steno` must already be in canonical key
//! order (`#STKPWHRAO*EUFRPBLGTS`, i.e. left-bank then mid then right-bank in
//! `LEFT_ORDER`/`MID_ORDER`/`RIGHT_ORDER` order). `parse_stroke` accepts keys
//! in any order within a bank and `render_stroke` silently re-sorts them, so
//! a mis-ordered entry parses fine and never surfaces as a build error —
//! this test is the only thing that would catch one.

use steno::{parse_source, parse_stroke, render_stroke};

/// The real `dict.steno` corpus, read as the test fixture.
const DICT: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../dict.steno"));

/// Every sub-stroke round-trips through `parse_stroke` + `render_stroke`
/// unchanged, proving it was already written in canonical bank order.
#[test]
fn all_dict_strokes_are_in_canonical_order() {
    let entries = parse_source(DICT).expect("dict.steno must parse");
    assert!(!entries.is_empty(), "dict.steno produced no entries");

    let mut violations = Vec::new();
    for entry in &entries {
        for sub in &entry.stroke {
            let keys = parse_stroke(sub).unwrap();
            let canonical = render_stroke(&keys);
            if sub != &canonical {
                violations.push(format!(
                    "line {}: stroke \"{sub}\" (in \"{}\") is out of order; canonical form is \"{canonical}\"",
                    entry.line, entry.stroke_raw
                ));
            }
        }
    }
    assert!(violations.is_empty(), "\n{}", violations.join("\n"));
}
