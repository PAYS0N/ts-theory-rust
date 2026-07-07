//! Build the Plover JSON dictionaries (`out/plain-ts.json`, `out/smart-ts.json`)
//! from `dict.steno`.
//!
//! Pipeline: parse → Pass A (counts) → Pass B (types) → render. Both profiles
//! load together on the device, so the real budget is their sum. Collisions in
//! either profile abort before any file is written.

use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use steno::{
    OrderedMap, TypedEntry, build_plain_dict, build_smart_dict, expand_dict, parse_source, to_json,
};

/// Entry point: build both dictionaries or report why not.
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fail(&e),
    }
}

/// The workspace root (holds `dict.steno` and `out/`).
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

/// Load, parse, and fully expand `dict.steno`.
fn load_typed() -> Result<Vec<TypedEntry>, String> {
    let src = fs::read_to_string(root().join("dict.steno")).map_err(|e| e.to_string())?;
    let entries = parse_source(&src).map_err(|e| e.to_string())?;
    expand_dict(&entries, None).map_err(|e| e.to_string())
}

/// Fail if any profile has a stroke collision (checked before any write).
fn guard(name: &str, collisions: &[String]) -> Result<(), String> {
    if collisions.is_empty() {
        return Ok(());
    }
    let unique: BTreeSet<&str> = collisions.iter().map(String::as_str).collect();
    let mut msg = format!("ERROR ({name}): {} stroke collision(s):", unique.len());
    for k in unique {
        msg.push_str("\n  ");
        msg.push_str(k);
    }
    Err(msg)
}

/// Serialize one dictionary to `out/<name>.json` and report its size.
fn write_dict(out_dir: &Path, name: &str, dict: &OrderedMap) -> Result<usize, String> {
    let json = to_json(dict);
    let path = out_dir.join(format!("{name}.json"));
    fs::write(&path, &json).map_err(|e| e.to_string())?;
    out_line(&format!("Wrote {}", path.display()));
    out_line(&format!("  {} strokes, {} bytes", dict.len(), json.len()));
    Ok(json.len())
}

/// Build both profiles, guard collisions, then write both files.
fn run() -> Result<(), String> {
    let typed = load_typed()?;
    let plain = build_plain_dict(&typed).map_err(|e| e.to_string())?;
    let smart = build_smart_dict(&typed).map_err(|e| e.to_string())?;
    guard("plain", &plain.collisions)?;
    guard("smart", &smart.collisions)?;

    let out_dir = root().join("out");
    fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let out_dir = fs::canonicalize(&out_dir).map_err(|e| e.to_string())?;
    let plain_bytes = write_dict(&out_dir, "plain-ts", &plain.dict)?;
    let smart_bytes = write_dict(&out_dir, "smart-ts", &smart.dict)?;
    out_line(&format!(
        "  combined (both load on device): {} bytes",
        plain_bytes + smart_bytes
    ));
    Ok(())
}
