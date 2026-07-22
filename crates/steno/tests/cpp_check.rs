//! Drives the C++ differential check (`scripts/cpp_check.sh`) under `cargo
//! test`, so it runs as part of `ctx-verify` without a separate script-battery
//! hook. This is the Rust half of the of-javelin brief's D9 differential: it
//! regenerates the two `out/` headers from `dict.infinite.steno` using the very
//! same emit functions `build-javelin` calls, then hands off to the script,
//! which compiles `javelin-ext` against `javelin-steno` and replays every
//! golden through `StenoGeneratedDictionary`.
//!
//! Header generation happens here (not by shelling out to `build-javelin`)
//! precisely because this runs inside `cargo test`: a nested `cargo run` would
//! deadlock on the build lock. The script therefore never invokes cargo.
//!
//! A missing C++ toolchain or absent upstream `javelin-steno` tree makes the
//! script emit `SKIP:` and exit 0; this test surfaces that as a passing skip.
//! Unlike `nvim_plugin.rs`, absence is not a loud failure here: the walker's
//! logic is already fully pinned by `infinite.rs`, so the cross-language
//! differential is corroboration, not the sole guardian (see the script header).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use steno::{build_tables, check_fuse_ambiguity, emit_data_header, emit_test_header, parse_source};

/// The workspace root (holds `dict.infinite.steno`, `out/`, and `scripts/`).
fn repo_root() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

/// Regenerate the two C++ headers into `out/` from the infinite corpus, via the
/// same pipeline `build-javelin` uses (parse -> Pass A -> fuse check -> emit).
fn generate_headers(root: &Path) -> Result<(), String> {
    let src = fs::read_to_string(root.join("dict.infinite.steno")).map_err(|e| e.to_string())?;
    let entries = parse_source(&src).map_err(|e| e.to_string())?;
    let (types, constructs) = build_tables(&entries).map_err(|e| e.to_string())?;
    check_fuse_ambiguity(&types, &constructs).map_err(|e| e.to_string())?;
    let data = emit_data_header(&types, &constructs).map_err(|e| e.to_string())?;
    let test = emit_test_header(&types, &constructs).map_err(|e| e.to_string())?;

    let out_dir = root.join("out");
    fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    fs::write(out_dir.join("steno_generated_dictionary_data.h"), data)
        .map_err(|e| e.to_string())?;
    fs::write(out_dir.join("steno_generated_testdata.h"), test).map_err(|e| e.to_string())?;
    Ok(())
}

/// Run `scripts/cpp_check.sh <root>` and capture its output.
fn run_cpp_check(root: &Path) -> Result<Output, String> {
    Command::new("bash")
        .arg(root.join("scripts/cpp_check.sh"))
        .arg(root)
        .output()
        .map_err(|e| e.to_string())
}

/// Generate the headers, run the differential, and require agreement. A `SKIP:`
/// (no compiler / no upstream tree) counts as a pass; any `FAIL:` or nonzero
/// exit fails the test with the script's full output attached.
#[test]
fn cpp_dictionary_matches_reference_walker() {
    let root = repo_root();
    generate_headers(&root).expect("failed to generate the javelin-ext headers");

    let output = run_cpp_check(&root).expect("failed to run scripts/cpp_check.sh");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // A SKIP (no C++ compiler / no upstream tree) is a pass: the walker's logic
    // is already fully covered by the Rust tests, so the differential is
    // corroboration only (see this file's module doc and the script header).
    if stdout.starts_with("SKIP:") {
        return;
    }
    assert!(
        output.status.success(),
        "cpp_check failed:\nstdout: {stdout}\nstderr: {stderr}"
    );
}
