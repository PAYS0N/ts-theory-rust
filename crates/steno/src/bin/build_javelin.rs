//! Build the generated C++ headers for the `javelin-ext` replay dictionary
//! from `dict.infinite.steno`.
//!
//! Pipeline: parse → Pass A (counts) → build the type and construct tables →
//! **fuse ambiguity check** → emit. Pass B never runs (that is the point: the
//! `O(T^s)` enumeration is replaced by a per-lookup walk). Two headers land in
//! `out/`: the self-contained data header the dictionary includes, and a golden
//! header pinning that dictionary to the Rust reference walker (D9). Sibling of
//! `build_dict.rs`; the two corpora build independently (D5).

use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use steno::{
    Construct, InfType, build_tables, check_fuse_ambiguity, emit_data_header, emit_test_header,
    parse_source,
};

/// Entry point: build both headers or report why not.
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fail(&e),
    }
}

/// The workspace root (holds `dict.infinite.steno` and `out/`).
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Write a line to stdout.
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

/// Load, parse, and build the two tables from `dict.infinite.steno`.
fn load_tables() -> Result<(Vec<InfType>, Vec<Construct>), String> {
    let src = fs::read_to_string(root().join("dict.infinite.steno")).map_err(|e| e.to_string())?;
    let entries = parse_source(&src).map_err(|e| e.to_string())?;
    build_tables(&entries).map_err(|e| e.to_string())
}

/// Write one header to `out/<name>` and report its size.
fn write_header(out_dir: &Path, name: &str, body: &str) -> Result<usize, String> {
    let path = out_dir.join(name);
    fs::write(&path, body).map_err(|e| e.to_string())?;
    out_line(&format!("Wrote {}", path.display()));
    out_line(&format!("  {} bytes", body.len()));
    Ok(body.len())
}

/// Build the tables, guard fuse ambiguity, then emit both headers.
fn run() -> Result<(), String> {
    let (types, constructs) = load_tables()?;
    check_fuse_ambiguity(&types, &constructs).map_err(|e| e.to_string())?;
    let data = emit_data_header(&types, &constructs).map_err(|e| e.to_string())?;
    let test = emit_test_header(&types, &constructs).map_err(|e| e.to_string())?;

    let out_dir = root().join("out");
    fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let out_dir = fs::canonicalize(&out_dir).map_err(|e| e.to_string())?;
    let data_bytes = write_header(&out_dir, "steno_generated_dictionary_data.h", &data)?;
    write_header(&out_dir, "steno_generated_testdata.h", &test)?;
    out_line(&format!(
        "  {} types, {} constructs, {data_bytes} bytes of data header",
        types.len(),
        constructs.len(),
    ));
    Ok(())
}
