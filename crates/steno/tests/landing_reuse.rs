//! Every `%N` landing point in a single template must be used exactly once.
//! A landing index that appears twice collapses to one LSP tabstop (`snippet`
//! dedups and renumbers by sort order — see `crate::snippet::sorted_landings`),
//! so a repeated index silently merges two distinct cursor positions into
//! one, and if the repeated index happens to be the template's highest it
//! produces two `${0}` (LSP's final-exit tabstop) in the same snippet body.
//! Nothing else in the pipeline catches this — it parses and expands fine.

use steno::{Chunk, parse_source};

/// The real `dict.steno` corpus, read as the test fixture.
const DICT: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../dict.steno"));

/// Collect every `Landing` index in a template, recursing into `Repeat`
/// bodies/separators (the only nested chunk container).
fn collect_landings(chunks: &[Chunk], out: &mut Vec<u32>) {
    for chunk in chunks {
        match chunk {
            Chunk::Landing(n) => out.push(*n),
            Chunk::Repeat { sep, body } => {
                collect_landings(sep, out);
                collect_landings(body, out);
            },
            _ => {},
        }
    }
}

/// No template in the corpus reuses a landing index across distinct
/// placeholder positions.
#[test]
fn no_dict_template_reuses_a_landing_index() {
    let entries = parse_source(DICT).expect("dict.steno must parse");
    assert!(!entries.is_empty(), "dict.steno produced no entries");

    let mut violations = Vec::new();
    for entry in &entries {
        let mut landings = Vec::new();
        collect_landings(&entry.template, &mut landings);

        let mut seen = Vec::new();
        for n in landings {
            if seen.contains(&n) {
                violations.push(format!(
                    "line {}: stroke \"{}\" reuses landing index %{n} in more than one placeholder",
                    entry.line, entry.stroke_raw
                ));
            } else {
                seen.push(n);
            }
        }
    }
    assert!(violations.is_empty(), "\n{}", violations.join("\n"));
}
