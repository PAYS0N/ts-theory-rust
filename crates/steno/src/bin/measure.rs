//! Size matrix: generic-arg pool × (full param counts vs. no-param functions).
//!
//! "no-param" = function shapes carry no AOE param expansion; params would be
//! added afterward by a separate stroke (not counted here). Writes no files;
//! sizes are reported in whole KB (float arithmetic is disallowed here).

use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use steno::{
    BuildResult, Entry, build_plain_dict, build_smart_dict, expand_dict, parse_source,
    parse_template, to_json,
};

/// Function shapes with their param expansion removed.
const NOPARAM: &[(&str, &str)] = &[
    ("STKWR-PBGS/-FLT", "function %0(): %t {%b%1}"),
    ("STKWR-PBGS/-R", "(): %t => {%b%0}"),
    ("STKWR-PBGS/-PL", "%0(): %t {%b%1}"),
    ("STKWR-PBGS/-PB", "async function %0(): Promise<%t> {%b%1}"),
    ("STKWR-PBGS/-D", "function* %0(): Generator<%t> {%b%1}"),
    ("STKWR-PBGS/-F", "(function %0(): %t {%b%1})();"),
];

/// The generic-arg pools to compare (label, restricting stroke set).
const POOLS: &[(&str, Option<&[&str]>)] = &[
    ("all arity-0 (14)", None),
    (
        "str,num,bool,unknown,any (5)",
        Some(&["STR", "TPH", "PW", "TPWH", "STKPWHR"]),
    ),
    ("str,num,bool (3)", Some(&["STR", "TPH", "PW"])),
];

/// Entry point: print the size matrix or report a pipeline error.
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fail(&e),
    }
}

/// The workspace root (holds `dict.steno`).
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Write a line to stdout (a failed write leaves nowhere to report).
fn out_line(s: &str) {
    let mut w = io::stdout();
    let _ = w.write_all(s.as_bytes());
    let _ = w.write_all(b"\n");
}

/// Report an error to stderr and yield the failure code.
fn fail(msg: &str) -> ExitCode {
    let _ = io::stderr().write_all(msg.as_bytes());
    let _ = io::stderr().write_all(b"\n");
    ExitCode::FAILURE
}

/// Replace the listed function entries with their no-param templates.
fn no_params(entries: &[Entry]) -> Result<Vec<Entry>, String> {
    entries
        .iter()
        .map(|e| match NOPARAM.iter().find(|(k, _)| *k == e.stroke_raw) {
            Some((_, t)) => {
                let template = parse_template(t, e.line).map_err(|err| err.to_string())?;
                Ok(Entry {
                    count: None,
                    template,
                    ..e.clone()
                })
            },
            None => Ok(e.clone()),
        })
        .collect()
}

/// The serialized size of a build result in whole KB.
fn kb(result: &BuildResult) -> usize {
    to_json(&result.dict).len() / 1024
}

/// A note about collisions in a build, if any.
fn collision_note(result: &BuildResult) -> String {
    let unique: BTreeSet<&str> = result.collisions.iter().map(String::as_str).collect();
    if unique.is_empty() {
        String::new()
    } else {
        format!("  ({} collisions!)", unique.len())
    }
}

/// Expand one configuration and print its sizes.
fn measure(label: &str, entries: &[Entry], pool: Option<&[&str]>) -> Result<(), String> {
    let typed = expand_dict(entries, pool).map_err(|e| e.to_string())?;
    let plain = build_plain_dict(&typed).map_err(|e| e.to_string())?;
    let smart = build_smart_dict(&typed).map_err(|e| e.to_string())?;
    let (pk, sk) = (kb(&plain), kb(&smart));
    out_line(&format!(
        "  {label:<32} {:>6} strokes  plain {pk:>5} KB  smart {sk:>5} KB  both {:>5} KB{}",
        plain.dict.len(),
        pk + sk,
        collision_note(&plain)
    ));
    Ok(())
}

/// Load the corpus and print both halves of the matrix.
fn run() -> Result<(), String> {
    let src = fs::read_to_string(root().join("dict.steno")).map_err(|e| e.to_string())?;
    let entries = parse_source(&src).map_err(|e| e.to_string())?;

    out_line("FULL params (functions keep AOE 0-7 param counts):");
    for (label, pool) in POOLS {
        measure(label, &entries, *pool)?;
    }
    out_line("NO param strokes (params added later via a separate stroke):");
    let np = no_params(&entries)?;
    for (label, pool) in POOLS {
        measure(label, &np, *pool)?;
    }
    Ok(())
}
