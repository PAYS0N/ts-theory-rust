//! Build the Neovim snippet artifacts from `dict.steno`.
//!
//! Emits `out/vim-snippets.json` (stroke → keyset token) and `out/snippets.json`
//! (token → LSP snippet body). Plover stays a dumb lookup table; the nvim plugin
//! loads `snippets.json` and expands the token into a real snippet with
//! tabstops. Keyset collisions abort before any file is written.

use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use steno::{OrderedMap, build_snippets, expand_dict, parse_source, to_json};

/// Entry point: build both nvim artifacts or report why not.
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

/// Fail if any keyset collided (checked before any write).
fn guard(collisions: &[String]) -> Result<(), String> {
    if collisions.is_empty() {
        return Ok(());
    }
    let unique: BTreeSet<&str> = collisions.iter().map(String::as_str).collect();
    let mut msg = format!("ERROR: {} keyset collision(s):", unique.len());
    for k in unique {
        msg.push_str("\n  ");
        msg.push_str(k);
    }
    Err(msg)
}

/// Serialize one map to `out/<name>` and report its size.
fn write_json(out_dir: &Path, name: &str, map: &OrderedMap) -> Result<usize, String> {
    let json = to_json(map);
    let path = out_dir.join(name);
    fs::write(&path, &json).map_err(|e| e.to_string())?;
    out_line(&format!("Wrote {}", path.display()));
    out_line(&format!("  {} keys, {} bytes", map.len(), json.len()));
    Ok(json.len())
}

/// Build the snippet artifacts, guard collisions, then write both files.
fn run() -> Result<(), String> {
    let src = fs::read_to_string(root().join("dict.steno")).map_err(|e| e.to_string())?;
    let entries = parse_source(&src).map_err(|e| e.to_string())?;
    let typed = expand_dict(&entries, None).map_err(|e| e.to_string())?;
    let build = build_snippets(&typed).map_err(|e| e.to_string())?;
    guard(&build.collisions)?;

    let out_dir = root().join("out");
    fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let out_dir = fs::canonicalize(&out_dir).map_err(|e| e.to_string())?;
    let a = write_json(&out_dir, "vim-snippets.json", &build.plover_keys)?;
    let b = write_json(&out_dir, "snippets.json", &build.snippets)?;
    out_line(&format!("  combined: {} bytes", a + b));
    Ok(())
}
