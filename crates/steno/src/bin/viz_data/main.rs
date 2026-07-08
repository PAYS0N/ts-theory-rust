//! Build `out/viz-data.json`: the category/stroke tree the standalone
//! `viz/index.html` viewer drills through, letting you pick a category (e.g.
//! "functions" or "if") and see the next stroke choices down to the full
//! outline and its rendered snippet text.
//!
//! Categories come only from explicit `## label` markers in `dict.steno`
//! (see `comments`), never guessed from ordinary prose comments.

mod axis;
mod build;
mod comments;
mod grouping;
mod json;
mod tree;

use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use steno::{TypedEntry, expand_dict, parse_source};

/// Entry point: build the viz tree or report why not.
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

/// Load `dict.steno`'s raw text (needed twice: once for the `##` marker
/// scan, once for parsing).
fn load_source() -> Result<String, String> {
    fs::read_to_string(root().join("dict.steno")).map_err(|e| e.to_string())
}

/// Parse and fully expand `dict.steno` from its already-loaded source.
fn load_typed(src: &str) -> Result<Vec<TypedEntry>, String> {
    let entries = parse_source(src).map_err(|e| e.to_string())?;
    expand_dict(&entries, None).map_err(|e| e.to_string())
}

/// Build the tree, serialize it, and write `out/viz-data.json`.
fn run() -> Result<(), String> {
    let src = load_source()?;
    let markers = comments::scan(&src);
    let desc_map = comments::scan_desc(&src);
    let typed = load_typed(&src)?;
    let data = build::build(&typed, &markers, &desc_map)?;
    let json = json::to_json(&data);

    let out_dir = root().join("out");
    fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let out_dir = fs::canonicalize(&out_dir).map_err(|e| e.to_string())?;
    let path = out_dir.join("viz-data.json");
    fs::write(&path, &json).map_err(|e| e.to_string())?;

    out_line(&format!("Wrote {}", path.display()));
    out_line(&format!(
        "  {} categories, {} nodes, {} bytes",
        data.category_count(),
        data.node_count(),
        json.len()
    ));
    Ok(())
}
